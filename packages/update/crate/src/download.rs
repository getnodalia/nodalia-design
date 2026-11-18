//! HTTP: the release lookup and the verified download.

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::{App, Error, Release};

const USER_AGENT: &str = "Nodalia-Updater";

/// Tells two downloads in one process apart, so neither can write into the other's partial file.
static DOWNLOADS: AtomicU64 = AtomicU64::new(0);

/// The operating system's certificate store, so a company or distribution trust anchor works.
fn tls() -> ureq::tls::TlsConfig {
    ureq::tls::TlsConfig::builder()
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build()
}

/// The answer to a release lookup.
#[derive(Debug)]
pub(crate) enum Fetched {
    Body {
        body: Vec<u8>,
        etag: Option<String>,
    },
    /// `304`: what was kept is still current.
    NotModified,
    /// GitHub's limit on anonymous API calls is spent; it lifts at `reset` (seconds since the
    /// epoch) when GitHub says.
    RateLimited {
        reset: Option<u64>,
    },
}

fn small_agent(redirects: bool) -> ureq::Agent {
    let mut config = ureq::Agent::config_builder()
        .tls_config(tls())
        .timeout_global(Some(Duration::from_secs(30)))
        // Statuses are read here: a refused request has headers worth reading.
        .http_status_as_error(false);
    if !redirects {
        config = config.max_redirects(0).max_redirects_will_error(false);
    }
    config.build().into()
}

/// Asks for `url`, conditionally on `etag`, and reads what the status means.
pub(crate) fn get_conditional(
    url: &str,
    accept: &str,
    etag: Option<&str>,
) -> Result<Fetched, Error> {
    let mut request = small_agent(true)
        .get(url)
        .header("User-Agent", USER_AGENT)
        .header("Accept", accept)
        .header("X-GitHub-Api-Version", "2022-11-28");
    if let Some(etag) = etag {
        request = request.header("If-None-Match", etag);
    }
    let mut response = request.call().map_err(|e| Error::Network(e.to_string()))?;
    let status = response.status().as_u16();
    let header = |name: &str| {
        response
            .headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned)
    };
    let remaining = header("x-ratelimit-remaining");
    let reset = header("x-ratelimit-reset");
    let retry_after = header("retry-after");
    let etag = header("etag");
    let body = response
        .body_mut()
        .with_config()
        .limit(8 * 1024 * 1024)
        .read_to_vec();
    match status {
        200..=299 => Ok(Fetched::Body {
            body: body.map_err(|e| Error::Network(e.to_string()))?,
            etag,
        }),
        304 => Ok(Fetched::NotModified),
        _ => {
            let text = body.unwrap_or_default();
            match rate_limit(
                status,
                remaining.as_deref(),
                reset.as_deref(),
                retry_after.as_deref(),
                &text,
                crate::lookup::now(),
            ) {
                Some(reset) => Ok(Fetched::RateLimited { reset }),
                None => Err(Error::Network(format!("http status: {status}"))),
            }
        }
    }
}

/// Whether a refused request was refused for GitHub's rate limit, and when the limit lifts if it
/// says: its primary limit (`x-ratelimit-remaining: 0`) or a secondary one (`retry-after`, or a
/// message naming the limit).
pub(crate) fn rate_limit(
    status: u16,
    remaining: Option<&str>,
    reset: Option<&str>,
    retry_after: Option<&str>,
    body: &[u8],
    now: u64,
) -> Option<Option<u64>> {
    if status != 403 && status != 429 {
        return None;
    }
    let spent = remaining.map(str::trim) == Some("0");
    let retry = retry_after.and_then(|r| r.trim().parse::<u64>().ok());
    let says = String::from_utf8_lossy(body.get(..4096).unwrap_or(body))
        .to_ascii_lowercase()
        .contains("rate limit");
    if !(spent || retry.is_some() || says || status == 429) {
        return None;
    }
    let reset = match retry {
        Some(secs) => Some(now.saturating_add(secs.min(24 * 60 * 60))),
        None => reset
            .and_then(|r| r.trim().parse::<u64>().ok())
            .filter(|&r| r > now && r - now <= 24 * 60 * 60),
    };
    Some(reset)
}

/// Where `url` redirects to, without following it: `None` if it answers without a redirect.
pub(crate) fn redirect_target(url: &str) -> Result<Option<String>, Error> {
    let response = small_agent(false)
        .head(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| Error::Network(e.to_string()))?;
    if !response.status().is_redirection() {
        return Ok(None);
    }
    Ok(response
        .headers()
        .get("location")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned))
}

/// Downloads the release's package into [`update_dir`](crate::update_dir) and returns its path.
///
/// `progress(received, total)` is called as bytes arrive; `total` is `None` when the server does
/// not say. The file is hashed while it is written, and is kept only if the SHA-256 matches the
/// digest GitHub published for it: a release without one is refused.
pub fn download(
    app: &App,
    release: &Release,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<PathBuf, Error> {
    let asset = release
        .asset
        .as_ref()
        .ok_or(Error::NoPackage(release.version.clone()))?;
    let expected = asset.sha256.as_deref().ok_or(Error::NoDigest)?;

    let dir = crate::update_dir(app);
    fs::create_dir_all(&dir)
        .map_err(|e| Error::Io(format!("Could not create {}: {e}", dir.display())))?;
    // A release's asset name is the server's text: keep only its last component.
    let name = std::path::Path::new(&asset.name)
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|n| !n.is_empty())
        .ok_or(Error::BadResponse)?;
    let dest = dir.join(name);
    // Unique per call: two downloads of one asset must not truncate each other's file, nor
    // publish bytes that were not the ones they hashed.
    let partial = dir.join(format!(
        "{name}.{}-{}.part",
        std::process::id(),
        DOWNLOADS.fetch_add(1, Ordering::Relaxed)
    ));

    let result = fetch_verified(
        &asset.url,
        asset.size,
        expected,
        &partial,
        &dest,
        &mut progress,
    );
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
}

/// How long the download may go without receiving a single byte before the attempt gives up.
const STALL: Duration = Duration::from_secs(60);

/// Attempts in a row that may end without a new byte before the download gives up.
const ATTEMPTS: u32 = 5;

/// Downloads `url` into `partial`, picking up after a dropped connection with a `Range` request
/// for what is missing, then checks the whole file against `expected` and moves it to `dest`.
///
/// A slow connection is never cut off: only one that stops sending for [`STALL`] is.
fn fetch_verified(
    url: &str,
    size: Option<u64>,
    expected: &str,
    partial: &std::path::Path,
    dest: &std::path::Path,
    progress: &mut impl FnMut(u64, Option<u64>),
) -> Result<PathBuf, Error> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(tls())
        // No deadline for the whole body, which a large package on a slow link outlasts; every
        // wait for the server is bounded instead, so a connection that stalls gives up.
        .timeout_resolve(Some(Duration::from_secs(15)))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_per_read(Some(STALL))
        .http_status_as_error(false)
        .build()
        .into();
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(partial)
        .map_err(|e| Error::Io(e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut received = 0u64;
    let mut total = size;
    let mut failures = 0u32;
    loop {
        let before = received;
        match fetch_from(
            &agent,
            url,
            &mut file,
            &mut hasher,
            &mut received,
            &mut total,
            progress,
        ) {
            Ok(()) => break,
            Err(Retry::No(error)) => return Err(error),
            Err(Retry::Yes(error)) => {
                failures = if received > before { 1 } else { failures + 1 };
                if failures >= ATTEMPTS {
                    return Err(error);
                }
                std::thread::sleep(backoff(failures));
            }
        }
    }
    file.flush().map_err(|e| Error::Io(e.to_string()))?;
    drop(file);
    if !hex(&hasher.finalize()).eq_ignore_ascii_case(expected) {
        return Err(Error::Checksum);
    }
    fs::rename(partial, dest).map_err(|e| Error::Io(e.to_string()))?;
    // What was verified, written beside the package: the macOS helper re-checks the bytes it
    // installs against this instead of trusting that the file is unchanged since now.
    let note = crate::verified_digest_path(dest);
    fs::write(&note, expected.to_ascii_lowercase()).map_err(|e| {
        let _ = fs::remove_file(dest);
        Error::Io(e.to_string())
    })?;
    Ok(dest.to_path_buf())
}

/// Whether a failed attempt is worth another.
enum Retry {
    Yes(Error),
    No(Error),
}

/// The wait before attempt `failures + 1`: 1, 2, 4, 8 seconds.
fn backoff(failures: u32) -> Duration {
    Duration::from_secs(1u64 << failures.saturating_sub(1).min(3))
}

/// One request: the whole file, or what is missing after `received` bytes.
fn fetch_from(
    agent: &ureq::Agent,
    url: &str,
    file: &mut fs::File,
    hasher: &mut Sha256,
    received: &mut u64,
    total: &mut Option<u64>,
    progress: &mut impl FnMut(u64, Option<u64>),
) -> Result<(), Retry> {
    let interrupted = |e: String| Retry::Yes(Error::Interrupted(e));
    let mut request = agent.get(url).header("User-Agent", USER_AGENT);
    if *received > 0 {
        request = request.header("Range", format!("bytes={received}-"));
    }
    let response = request.call().map_err(|e| interrupted(e.to_string()))?;
    let status = response.status().as_u16();
    let length = header_u64(response.headers().get("content-length"));
    match status {
        206 if *received > 0 => {
            let range = response
                .headers()
                .get("content-range")
                .and_then(|v| v.to_str().ok())
                .and_then(content_range);
            match range {
                Some((start, whole)) if start == *received => {
                    *total = whole.or(*total);
                }
                // Not the bytes asked for: start over rather than splice the wrong ones in.
                _ => {
                    restart(file, hasher, received)?;
                    return Err(interrupted("the server resumed at the wrong place".into()));
                }
            }
        }
        200 => {
            // A server that ignores the range sends the whole file again.
            if *received > 0 {
                restart(file, hasher, received)?;
            }
            *total = length.or(*total);
        }
        408 | 429 | 500..=599 => return Err(interrupted(format!("http status: {status}"))),
        _ => {
            return Err(Retry::No(Error::Network(format!("http status: {status}"))));
        }
    }
    progress(*received, *total);

    let mut reader = response.into_body().into_reader();
    let mut buffer = vec![0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| interrupted(e.to_string()))?;
        let Some(chunk) = buffer.get(..read).filter(|c| !c.is_empty()) else {
            break;
        };
        hasher.update(chunk);
        file.write_all(chunk)
            .map_err(|e| Retry::No(Error::Io(e.to_string())))?;
        *received += read as u64;
        progress(*received, *total);
    }
    // A body that ends early without an error is still short.
    match *total {
        Some(total) if *received < total => Err(interrupted("the connection closed early".into())),
        _ => Ok(()),
    }
}

/// Empties the partial file to take a whole new copy.
fn restart(file: &mut fs::File, hasher: &mut Sha256, received: &mut u64) -> Result<(), Retry> {
    use std::io::Seek;
    file.set_len(0)
        .and_then(|()| file.rewind())
        .map_err(|e| Retry::No(Error::Io(e.to_string())))?;
    *hasher = Sha256::new();
    *received = 0;
    Ok(())
}

fn header_u64(value: Option<&ureq::http::HeaderValue>) -> Option<u64> {
    value?.to_str().ok()?.trim().parse().ok()
}

/// `bytes 100-199/1000` → the first byte, and the whole size when the server knows it.
fn content_range(value: &str) -> Option<(u64, Option<u64>)> {
    let rest = value.trim().strip_prefix("bytes ")?;
    let (span, whole) = rest.split_once('/')?;
    let (start, _) = span.split_once('-')?;
    let start = start.trim().parse().ok()?;
    let whole = match whole.trim() {
        "*" => None,
        n => Some(n.parse().ok()?),
    };
    Some((start, whole))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_spent_limit_counts_as_one() {
        let now = 1_000;
        // The primary limit, with its reset time.
        assert_eq!(
            rate_limit(403, Some("0"), Some("1600"), None, b"", now),
            Some(Some(1600))
        );
        // A secondary limit: retry-after wins over the reset header.
        assert_eq!(
            rate_limit(403, Some("12"), Some("1600"), Some("60"), b"", now),
            Some(Some(1060))
        );
        assert_eq!(rate_limit(429, None, None, None, b"", now), Some(None));
        assert_eq!(
            rate_limit(
                403,
                None,
                None,
                None,
                br#"{"message":"API rate limit exceeded for 1.2.3.4."}"#,
                now
            ),
            Some(None)
        );
        // A reset in the past, or absurdly far, is not shown.
        assert_eq!(
            rate_limit(403, Some("0"), Some("10"), None, b"", now),
            Some(None)
        );
        assert_eq!(
            rate_limit(403, Some("0"), Some("999999999"), None, b"", now),
            Some(None)
        );
        // Any other refusal is not the limit.
        assert_eq!(
            rate_limit(403, Some("40"), None, None, b"Forbidden", now),
            None
        );
        assert_eq!(rate_limit(404, Some("0"), None, None, b"", now), None);
        assert_eq!(rate_limit(500, None, None, Some("5"), b"", now), None);
    }

    #[test]
    fn content_range_reads_the_start_and_the_size() {
        assert_eq!(content_range("bytes 100-199/1000"), Some((100, Some(1000))));
        assert_eq!(content_range("bytes 5-9/*"), Some((5, None)));
        assert_eq!(content_range("bytes */1000"), None);
        assert_eq!(content_range("items 1-2/3"), None);
    }

    #[test]
    fn backoff_doubles_and_stops_growing() {
        let secs: Vec<u64> = (1..=6).map(|f| backoff(f).as_secs()).collect();
        assert_eq!(secs, [1, 2, 4, 8, 8, 8]);
    }

    /// A server for `body` that cuts the first answer off at `cut` bytes, then answers what each
    /// request asks for, honouring `Range` only when `ranges` is set. Returns the URL and the
    /// `Range` headers it saw.
    fn flaky_server(
        body: Vec<u8>,
        cut: usize,
        ranges: bool,
    ) -> (String, std::sync::mpsc::Receiver<Option<String>>) {
        use std::io::BufRead;
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/package.dmg", listener.local_addr().unwrap());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for (n, stream) in listener.incoming().enumerate() {
                let mut stream = stream.unwrap();
                let mut reader = std::io::BufReader::new(stream.try_clone().unwrap());
                let mut range = None;
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).unwrap() == 0 || line.trim().is_empty() {
                        break;
                    }
                    if let Some(v) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
                        range = v.trim().trim_end_matches('-').parse::<usize>().ok();
                    }
                }
                let _ = tx.send(range.map(|r| r.to_string()));
                let (status, start) = match range {
                    Some(r) if ranges => ("206 Partial Content", r),
                    _ => ("200 OK", 0),
                };
                let rest = &body[start..];
                let mut head = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n",
                    rest.len()
                );
                if start > 0 {
                    head += &format!(
                        "Content-Range: bytes {start}-{}/{}\r\n",
                        body.len() - 1,
                        body.len()
                    );
                }
                head += "\r\n";
                stream.write_all(head.as_bytes()).unwrap();
                let send = if n == 0 { &rest[..cut] } else { rest };
                stream.write_all(send).unwrap();
                stream.flush().unwrap();
            }
        });
        (url, rx)
    }

    fn fetch_into(dir: &std::path::Path, url: &str, body: &[u8]) -> Result<Vec<u8>, Error> {
        let expected = hex(&Sha256::digest(body));
        let partial = dir.join("package.dmg.part");
        let dest = dir.join("package.dmg");
        let mut last = (0, None);
        let path = fetch_verified(
            url,
            Some(body.len() as u64),
            &expected,
            &partial,
            &dest,
            &mut |r, t| last = (r, t),
        )?;
        assert_eq!(last, (body.len() as u64, Some(body.len() as u64)));
        assert!(!partial.exists());
        Ok(fs::read(path).unwrap())
    }

    fn package() -> Vec<u8> {
        (0..300_000u32).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn a_dropped_download_carries_on_from_where_it_stopped() {
        let body = package();
        let (url, seen) = flaky_server(body.clone(), 200_000, true);
        let dir = tempdir();
        assert_eq!(fetch_into(&dir, &url, &body).unwrap(), body);
        assert_eq!(seen.recv().unwrap(), None);
        assert_eq!(seen.recv().unwrap().as_deref(), Some("200000"));
    }

    #[test]
    fn a_server_that_ignores_the_range_sends_the_whole_file_again() {
        let body = package();
        let (url, _) = flaky_server(body.clone(), 120_000, false);
        let dir = tempdir();
        assert_eq!(fetch_into(&dir, &url, &body).unwrap(), body);
    }

    #[test]
    fn a_missing_package_is_not_retried() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/gone", listener.local_addr().unwrap());
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = stream.unwrap();
                let mut buf = [0u8; 4096];
                let _ = stream.read(&mut buf);
                tx.send(()).unwrap();
                let _ = stream.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
            }
        });
        let dir = tempdir();
        let err = fetch_into(&dir, &url, b"x").unwrap_err();
        assert!(matches!(err, Error::Network(_)), "{err:?}");
        assert!(rx.recv().is_ok());
        assert!(rx.try_recv().is_err(), "a 404 was asked for again");
    }

    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nodalia-update-test-{}-{}",
            std::process::id(),
            DOWNLOADS.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn hex_is_lower_case_and_padded() {
        assert_eq!(hex(&[0x00, 0xab, 0x0f]), "00ab0f");
    }
}
