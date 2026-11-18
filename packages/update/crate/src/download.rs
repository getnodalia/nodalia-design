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

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(tls())
        // Large packages: no overall deadline, but every wait for the server is bounded, so a
        // stalled connection, a server that never answers or one that stops mid-body gives up.
        .timeout_resolve(Some(Duration::from_secs(15)))
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .timeout_recv_body(Some(Duration::from_secs(60)))
        .build()
        .into();
    let response = agent
        .get(&asset.url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| Error::Network(e.to_string()))?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse().ok())
        .or(asset.size);

    let result = (|| {
        let mut reader = response.into_body().into_reader();
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&partial)
            .map_err(|e| Error::Io(e.to_string()))?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 64 * 1024];
        let mut received = 0u64;
        loop {
            let read = reader
                .read(&mut buffer)
                .map_err(|e| Error::Network(e.to_string()))?;
            let Some(chunk) = buffer.get(..read).filter(|c| !c.is_empty()) else {
                break;
            };
            hasher.update(chunk);
            file.write_all(chunk)
                .map_err(|e| Error::Io(e.to_string()))?;
            received += read as u64;
            progress(received, total);
        }
        file.flush().map_err(|e| Error::Io(e.to_string()))?;
        if !hex(&hasher.finalize()).eq_ignore_ascii_case(expected) {
            return Err(Error::Checksum);
        }
        fs::rename(&partial, &dest).map_err(|e| Error::Io(e.to_string()))?;
        // What was verified, written beside the package: the macOS helper re-checks the bytes it
        // installs against this instead of trusting that the file is unchanged since now.
        let note = crate::verified_digest_path(&dest);
        fs::write(&note, expected.to_ascii_lowercase()).map_err(|e| {
            let _ = fs::remove_file(&dest);
            Error::Io(e.to_string())
        })?;
        Ok(dest)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&partial);
    }
    result
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
    fn hex_is_lower_case_and_padded() {
        assert_eq!(hex(&[0x00, 0xab, 0x0f]), "00ab0f");
    }
}
