//! Finding the release list without tripping GitHub's limit on anonymous API calls.
//!
//! GitHub answers 60 anonymous API requests an hour per IP address, shared by every program on
//! the network: a household with a few Nodalia apps, a package manager and a shell prompt that
//! asks for releases can spend it, and the next check then fails with `403`. So:
//!
//! - the list is kept on disk with its `ETag`, and asked for again only with `If-None-Match`:
//!   GitHub does not count an answer of `304 Not Modified` against the limit;
//! - a list fetched a few minutes ago is used as it is, so "Try Again" costs nothing;
//! - when the limit is spent, `github.com/<repo>/releases/latest` (a web page redirect, not the
//!   API) still says which version is newest. If the kept list already has it, that list is used;
//!   if not, the release is offered with its page only.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::download::{self, Fetched};
use crate::{App, Error};

/// A list fetched this recently is used without asking GitHub again.
const FRESH: u64 = 5 * 60;

/// A list kept longer than this is not trusted to stand in for one GitHub refused to send.
const STALE: u64 = 7 * 24 * 60 * 60;

/// What the lookup found.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Found {
    /// The API's release list, fresh or confirmed unchanged.
    List(Vec<u8>),
    /// The API refused (its limit is spent); the release page says the newest version is this
    /// tag, and no list on disk describes it.
    LatestTag { tag: String, page: String },
}

/// The kept list: the body and the `ETag` it came with.
struct Kept {
    body: Vec<u8>,
    etag: Option<String>,
    /// When it was last fetched or confirmed, in seconds since the epoch.
    at: u64,
}

pub(crate) fn releases(app: &App) -> Result<Found, Error> {
    let dir = crate::update_dir(app);
    let kept = read_kept(&dir);
    let now = now();
    if let Some(kept) = kept.as_ref().filter(|k| fresh(k.at, now)) {
        return Ok(Found::List(kept.body.clone()));
    }
    let url = format!(
        "https://api.github.com/repos/{}/releases?per_page=20",
        app.repo
    );
    let etag = kept.as_ref().and_then(|k| k.etag.as_deref());
    match download::get_conditional(&url, "application/vnd.github+json", etag)? {
        Fetched::Body { body, etag } => {
            write_kept(&dir, &body, etag.as_deref());
            Ok(Found::List(body))
        }
        Fetched::NotModified => match kept {
            Some(kept) => {
                write_kept(&dir, &kept.body, kept.etag.as_deref());
                Ok(Found::List(kept.body))
            }
            // A 304 with nothing kept: a proxy answering for a request it was not asked. Ask
            // again without the tag.
            None => match download::get_conditional(&url, "application/vnd.github+json", None)? {
                Fetched::Body { body, etag } => {
                    write_kept(&dir, &body, etag.as_deref());
                    Ok(Found::List(body))
                }
                _ => Err(Error::BadResponse),
            },
        },
        Fetched::RateLimited { reset } => {
            let usable = kept.filter(|k| now.saturating_sub(k.at) < STALE);
            let latest = download::redirect_target(&format!(
                "https://github.com/{}/releases/latest",
                app.repo
            ))
            .ok()
            .flatten()
            .and_then(|page| tag_from_page(&page, app.repo).map(|tag| (tag, page)));
            choose_when_limited(usable.map(|k| k.body), latest, reset)
        }
    }
}

/// With the API's limit spent: the kept list if it already knows the newest release (or if the
/// web page could not say), the newest release's page if it does not, the limit's error if there
/// is neither.
fn choose_when_limited(
    kept: Option<Vec<u8>>,
    latest: Option<(String, String)>,
    reset: Option<u64>,
) -> Result<Found, Error> {
    match (kept, latest) {
        (Some(body), Some((tag, page))) => {
            if lists_tag(&body, &tag) {
                Ok(Found::List(body))
            } else {
                Ok(Found::LatestTag { tag, page })
            }
        }
        (Some(body), None) => Ok(Found::List(body)),
        (None, Some((tag, page))) => Ok(Found::LatestTag { tag, page }),
        (None, None) => Err(Error::RateLimited { reset }),
    }
}

/// Whether a release list names this tag.
fn lists_tag(body: &[u8], tag: &str) -> bool {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|list| {
            list.as_array().map(|releases| {
                releases
                    .iter()
                    .any(|r| r.get("tag_name").and_then(serde_json::Value::as_str) == Some(tag))
            })
        })
        .unwrap_or(false)
}

/// `https://github.com/owner/repo/releases/tag/v1.2.3` → `v1.2.3`, only for this repository.
/// The redirect of a repository with no release goes to `/releases`, which names no tag.
pub(crate) fn tag_from_page(page: &str, repo: &str) -> Option<String> {
    let prefix = format!("https://github.com/{repo}/releases/tag/");
    let tag = page.strip_prefix(&prefix)?;
    let tag = tag.split(['?', '#']).next().unwrap_or_default();
    let ok = !tag.is_empty()
        && tag.len() <= 100
        && tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '+'));
    ok.then(|| tag.to_owned())
}

fn fresh(at: u64, now: u64) -> bool {
    now >= at && now - at < FRESH
}

fn body_path(dir: &Path) -> PathBuf {
    dir.join("releases.json")
}

fn meta_path(dir: &Path) -> PathBuf {
    dir.join("releases.meta")
}

/// The kept list, if both its files are there and readable.
fn read_kept(dir: &Path) -> Option<Kept> {
    let body = std::fs::read(body_path(dir)).ok()?;
    let meta = std::fs::read_to_string(meta_path(dir)).ok()?;
    let (at, etag) = parse_meta(&meta)?;
    Some(Kept { body, etag, at })
}

/// `<seconds>\n<etag>`: the time it was fetched, then its `ETag` (an empty line if none).
fn parse_meta(meta: &str) -> Option<(u64, Option<String>)> {
    let mut lines = meta.lines();
    let at = lines.next()?.trim().parse().ok()?;
    let etag = lines
        .next()
        .map(str::trim)
        .filter(|e| !e.is_empty() && e.len() <= 200 && e.is_ascii())
        .map(str::to_owned);
    Some((at, etag))
}

/// Writes the body before the meta file, so a meta file always describes a whole body. A failure
/// only costs the next check its shortcut.
fn write_kept(dir: &Path, body: &[u8], etag: Option<&str>) {
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let _ = std::fs::remove_file(meta_path(dir));
    if std::fs::write(body_path(dir), body).is_ok() {
        let _ = std::fs::write(
            meta_path(dir),
            format!("{}\n{}\n", now(), etag.unwrap_or_default()),
        );
    }
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIST: &[u8] = br#"[{"tag_name":"v1.2.0","draft":false,"prerelease":false,"assets":[]}]"#;

    #[test]
    fn the_tag_comes_from_this_repository_s_release_page_only() {
        let repo = "getnodalia/forja";
        assert_eq!(
            tag_from_page(
                "https://github.com/getnodalia/forja/releases/tag/v0.2.1",
                repo
            ),
            Some("v0.2.1".into())
        );
        assert_eq!(
            tag_from_page(
                "https://github.com/getnodalia/forja/releases/tag/v0.2.1?x=1",
                repo
            ),
            Some("v0.2.1".into())
        );
        for page in [
            "https://github.com/getnodalia/forja/releases",
            "https://github.com/getnodalia/other/releases/tag/v9.0.0",
            "https://github.com/getnodalia/forja/releases/tag/",
            "https://github.com/getnodalia/forja/releases/tag/v1/../../x",
            "http://github.com/getnodalia/forja/releases/tag/v1.0.0",
        ] {
            assert_eq!(tag_from_page(page, repo), None, "{page}");
        }
    }

    #[test]
    fn a_list_is_fresh_for_a_few_minutes_only() {
        assert!(fresh(1000, 1000));
        assert!(fresh(1000, 1000 + FRESH - 1));
        assert!(!fresh(1000, 1000 + FRESH));
        assert!(!fresh(2000, 1000), "a clock that went back");
    }

    #[test]
    fn the_meta_file_reads_back_what_was_written() {
        assert_eq!(
            parse_meta("12\n\"abc\"\n"),
            Some((12, Some("\"abc\"".into())))
        );
        assert_eq!(parse_meta("12\n\n"), Some((12, None)));
        assert_eq!(parse_meta("12"), Some((12, None)));
        assert_eq!(parse_meta("x\n\"abc\""), None);
        assert_eq!(parse_meta(""), None);
    }

    #[test]
    fn the_kept_list_round_trips_through_the_disk() {
        let dir = std::env::temp_dir().join(format!("nodalia-update-k-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert!(read_kept(&dir).is_none());
        write_kept(&dir, LIST, Some("W/\"e1\""));
        let kept = read_kept(&dir).unwrap();
        assert_eq!(kept.body, LIST);
        assert_eq!(kept.etag.as_deref(), Some("W/\"e1\""));
        assert!(fresh(kept.at, now()));
        // A body without its meta file is not used.
        std::fs::remove_file(meta_path(&dir)).unwrap();
        assert!(read_kept(&dir).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn with_the_limit_spent_the_newest_release_still_shows() {
        let page = "https://github.com/o/r/releases/tag/v1.2.0".to_owned();
        let newer = "https://github.com/o/r/releases/tag/v1.3.0".to_owned();
        // The kept list already knows the newest release: use it, with its packages.
        assert_eq!(
            choose_when_limited(Some(LIST.to_vec()), Some(("v1.2.0".into(), page)), None),
            Ok(Found::List(LIST.to_vec()))
        );
        // It does not: the release page.
        assert_eq!(
            choose_when_limited(
                Some(LIST.to_vec()),
                Some(("v1.3.0".into(), newer.clone())),
                None
            ),
            Ok(Found::LatestTag {
                tag: "v1.3.0".into(),
                page: newer.clone()
            })
        );
        assert_eq!(
            choose_when_limited(None, Some(("v1.3.0".into(), newer.clone())), None),
            Ok(Found::LatestTag {
                tag: "v1.3.0".into(),
                page: newer
            })
        );
        // The page could not say: whatever is kept.
        assert_eq!(
            choose_when_limited(Some(LIST.to_vec()), None, None),
            Ok(Found::List(LIST.to_vec()))
        );
        // Nothing at all: the limit, with when it lifts.
        assert!(matches!(
            choose_when_limited(None, None, Some(99)),
            Err(Error::RateLimited { reset: Some(99) })
        ));
    }
}
