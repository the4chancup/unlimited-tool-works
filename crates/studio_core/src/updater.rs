//! The update check: whether a newer release is out, and what the offer keeps
//! of it. The shell starts a check when the settings say one is due
//! (`start_check`, on a thread of its own so no frame waits on the network),
//! reads the newest release by semver out of the fetched body
//! (`newest_release`), offers it when it is newer than the built version and
//! not the skipped one (`offers`), and stamps `last_update_check` when a check
//! completes. The `Release` the offer keeps (notes, page URL and assets)
//! feeds the Details dialog and the download. Native only: the browser build
//! has no binary to swap and no settings file.

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, bounded};
use semver::Version;
use serde::Deserialize;
use thiserror::Error;

use crate::settings::CommonSettings;

/// The releases list the check reads: `per_page=30`, so a run of tags that are
/// no release still leaves the newest release inside the one page.
const RELEASES_URL: &str =
    "https://api.github.com/repos/the4chancup/unlimited-tool-works/releases?per_page=30";

/// The whole check's budget: a list unanswered within it is a failed check,
/// which shows nothing.
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// A release's downloadable file: its name, the URL it downloads from and its
/// size in bytes.
#[derive(Debug)]
pub struct ReleaseAsset {
    /// The asset's file name, which the download picks the platform's archive by.
    pub name: String,
    /// The URL the file downloads from.
    pub download_url: String,
    /// The file's size in bytes.
    pub size: u64,
}

/// A release from the repository's releases list: its version parsed out of
/// the `v<semver>` tag, its notes (`body`), its page URL and its assets. The
/// offer keeps it whole: the Details dialog shows the notes and opens the
/// page, the download picks an asset.
#[derive(Debug)]
pub struct Release {
    /// The tag's semver, its `v` stripped.
    pub version: Version,
    /// The release's notes, shown as Markdown in the Details dialog.
    pub notes: String,
    /// The release's page on GitHub, the "Open release page" target.
    pub page_url: String,
    /// Its downloadable files.
    pub assets: Vec<ReleaseAsset>,
}

/// The releases list's entries as GitHub returns them: every field but the
/// tag passes through to `Release` unchanged.
#[derive(Deserialize)]
struct RawRelease {
    /// The git tag the release was published from (`v0.0.2`, `nightly`).
    tag_name: String,
    /// The release's notes; GitHub writes `null` for a release without any.
    body: Option<String>,
    /// The release's page on GitHub.
    html_url: String,
    /// Its downloadable files.
    assets: Vec<RawAsset>,
}

/// One asset of `RawRelease`, named as GitHub names it.
#[derive(Deserialize)]
struct RawAsset {
    /// The file's name.
    name: String,
    /// The URL it downloads from.
    browser_download_url: String,
    /// Its size in bytes.
    size: u64,
}

/// `raw` as a `Release`, or `None` when its tag is not `v` followed by a
/// semver: `v0.0.2` parses, `nightly` does not.
fn release(raw: RawRelease) -> Option<Release> {
    let version = raw.tag_name.strip_prefix('v')?.parse::<Version>().ok()?;
    Some(Release {
        version,
        notes: raw.body.unwrap_or_default(),
        page_url: raw.html_url,
        assets: raw
            .assets
            .into_iter()
            .map(|asset| ReleaseAsset {
                name: asset.name,
                download_url: asset.browser_download_url,
                size: asset.size,
            })
            .collect(),
    })
}

/// Why a check failed: the releases list could not be fetched, or a fetched
/// body did not parse. A failed check shows nothing; the shell logs it.
#[derive(Debug, Error)]
pub enum UpdateError {
    /// The GET failed (offline, rate-limited, a refused certificate) or the
    /// body could not be read.
    #[error("the releases list could not be fetched: {0}")]
    Fetch(#[from] ureq::Error),
    /// The fetched body was not the releases list's JSON.
    #[error("the releases list did not parse: {0}")]
    Parse(#[from] serde_json::Error),
}

/// What the check asks the network for: the releases list's body.
/// `fetch_releases` is the real one; a test hands its own.
pub type Fetch = dyn Fn() -> Result<String, UpdateError> + Send + Sync;

/// The newest release in a fetched releases list: the highest tag of the
/// `v<semver>` shape, pre-releases ordered as semver orders them (`0.0.1-rc.2`
/// under `0.0.1`). A tag of another shape is passed over; `releases/latest`
/// would skip a release marked Pre-release, which a 0.0.x may be. `None` comes
/// back when no tag parses.
///
/// Errors when `body` is not the list's JSON: a failed check, same as a
/// failed fetch.
pub fn newest_release(body: &str) -> Result<Option<Release>, UpdateError> {
    let releases: Vec<RawRelease> = serde_json::from_str(body)?;
    Ok(releases
        .into_iter()
        .filter_map(release)
        .max_by(|left, right| left.version.cmp(&right.version)))
}

/// Whether the offer is made for `release`: it is newer than `own`, the
/// built-in version, and not the version the member chose to skip
/// (`skipped_version`; a value that does not parse skips nothing).
pub fn offers(release: &Release, own: &Version, skipped: Option<&str>) -> bool {
    release.version > *own
        && skipped
            .and_then(|version| version.parse::<Version>().ok())
            .as_ref()
            != Some(&release.version)
}

/// Whether the shell starts a check: `check_for_updates` is on and the last
/// check is older than `check_interval_minutes`, or never ran. `now` is
/// seconds since the Unix epoch, `last_update_check`'s unit; a clock set
/// behind the last check keeps it inside the interval rather than counting it
/// past.
pub fn check_due(common: &CommonSettings, now: u64) -> bool {
    common.check_for_updates
        && common.last_update_check.is_none_or(|last| {
            now.saturating_sub(last) >= u64::from(common.check_interval_minutes) * 60
        })
}

/// Seconds since the Unix epoch, the unit `last_update_check` stores. A clock
/// set before the epoch reads 0: a check that always reads due beats one that
/// can never run.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

/// Starts the check on a thread of its own and returns the reply's channel:
/// `fetch`'s body parsed into the newest release (`newest_release`), `None`
/// when the list holds no `v<semver>` tag, or the check's failure; one send,
/// then the thread is done. The shell polls the receiver each frame, and the
/// send asks `egui_ctx` for a repaint so a finished check does not wait on
/// input. The fetch is the caller's (`fetch_releases` in the window, a fixture
/// in the tests), so the check's whole path runs in tests without the network.
pub fn start_check(
    fetch: Arc<Fetch>,
    egui_ctx: &egui::Context,
) -> Receiver<Result<Option<Release>, UpdateError>> {
    let (tx, rx) = bounded(1);
    let repaint = egui_ctx.clone();
    std::thread::spawn(move || {
        let outcome = fetch().and_then(|body| newest_release(&body));
        // One send, then the thread is done: a shell that dropped the receiver
        // closed mid-check, and the reply goes with it.
        if tx.send(outcome).is_ok() {
            repaint.request_repaint();
        } else {
            log::debug!("the update check's reply had no receiver left");
        }
    });
    rx
}

/// The real fetch: one GET of the releases list, with rustls on `ring` as the
/// crypto provider and the system's certificate store, a `User-Agent` GitHub
/// answers to and one global timeout. The body is read with ureq's default
/// cap; the list is a few dozen kilobytes.
pub fn fetch_releases() -> Result<String, UpdateError> {
    let agent = ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::Rustls)
                .root_certs(ureq::tls::RootCerts::PlatformVerifier)
                .unversioned_rustls_crypto_provider(Arc::new(
                    rustls::crypto::ring::default_provider(),
                ))
                .build(),
        )
        .user_agent(concat!("unlimited-tool-works/", env!("CARGO_PKG_VERSION")))
        .timeout_global(Some(FETCH_TIMEOUT))
        .build()
        .new_agent();
    let mut response = agent.get(RELEASES_URL).call()?;
    Ok(response.body_mut().read_to_string()?)
}

/// The releases list the updater's and the shell's tests share: `v0.0.2`
/// first (GitHub's `prerelease` flag on, which `releases/latest` would skip),
/// then a `nightly` tag that is no semver and `v0.0.1` last, so the newest by
/// semver is neither the first nor the last entry.
#[cfg(test)]
pub(crate) const RELEASES_FIXTURE: &str = r#"[
    {
        "tag_name": "v0.0.2",
        "prerelease": true,
        "body": "Fixes and polish.\n",
        "html_url": "https://github.com/the4chancup/unlimited-tool-works/releases/tag/v0.0.2",
        "assets": [
            {
                "name": "unlimited-tool-works-0.0.2-windows.7z",
                "browser_download_url": "https://github.com/the4chancup/unlimited-tool-works/releases/download/v0.0.2/unlimited-tool-works-0.0.2-windows.7z",
                "size": 11234567
            },
            {
                "name": "SHA256SUMS.txt",
                "browser_download_url": "https://github.com/the4chancup/unlimited-tool-works/releases/download/v0.0.2/SHA256SUMS.txt",
                "size": 123
            }
        ]
    },
    {
        "tag_name": "nightly",
        "prerelease": false,
        "body": null,
        "html_url": "https://github.com/the4chancup/unlimited-tool-works/releases/tag/nightly",
        "assets": []
    },
    {
        "tag_name": "v0.0.1",
        "prerelease": false,
        "body": "The preview.\n",
        "html_url": "https://github.com/the4chancup/unlimited-tool-works/releases/tag/v0.0.1",
        "assets": []
    }
]"#;

#[cfg(test)]
mod tests {
    use super::*;

    // ST-UPD-01
    #[test]
    fn the_newest_release_is_the_highest_v_semver_tag() {
        let release = newest_release(RELEASES_FIXTURE)
            .expect("the fixture parses")
            .expect("a release is found");
        assert_eq!(release.version, Version::new(0, 0, 2));
        assert_eq!(release.notes, "Fixes and polish.\n");
        assert_eq!(
            release.page_url,
            "https://github.com/the4chancup/unlimited-tool-works/releases/tag/v0.0.2"
        );
        assert_eq!(release.assets.len(), 2);
        assert_eq!(
            release.assets[0].name,
            "unlimited-tool-works-0.0.2-windows.7z"
        );
        assert_eq!(
            release.assets[0].download_url,
            "https://github.com/the4chancup/unlimited-tool-works/releases/download/v0.0.2/unlimited-tool-works-0.0.2-windows.7z"
        );
        assert_eq!(release.assets[0].size, 11234567);
        assert_eq!(release.assets[1].name, "SHA256SUMS.txt");
    }

    // ST-UPD-01
    #[test]
    fn the_offer_is_refused_for_the_skipped_version() {
        let release = newest_release(RELEASES_FIXTURE)
            .expect("the fixture parses")
            .expect("a release is found");
        let own = Version::new(0, 0, 1);
        assert!(offers(&release, &own, None));
        assert!(
            offers(&release, &own, Some("0.0.3")),
            "a skip of another version still offers"
        );
        assert!(!offers(&release, &own, Some("0.0.2")));
    }

    // ST-UPD-01
    #[test]
    fn a_release_no_newer_than_the_build_is_not_offered() {
        let body = r#"[{"tag_name": "v0.0.1", "body": null, "html_url": "u", "assets": []}]"#;
        let release = newest_release(body)
            .expect("the fixture parses")
            .expect("a release is found");
        assert_eq!(release.version, Version::new(0, 0, 1), "the premise");
        assert!(!offers(&release, &Version::new(0, 0, 1), None));
    }

    // ST-UPD-01
    #[test]
    fn pre_releases_order_as_semver_orders_them() {
        let release = newest_release(
            r#"[
                {"tag_name": "v0.0.1-rc.2", "body": null, "html_url": "u", "assets": []},
                {"tag_name": "v0.0.1-rc.1", "body": null, "html_url": "u", "assets": []}
            ]"#,
        )
        .expect("the fixture parses")
        .expect("a release is found");
        assert_eq!(release.version.to_string(), "0.0.1-rc.2");

        let release = newest_release(
            r#"[
                {"tag_name": "v0.0.1-rc.2", "body": null, "html_url": "u", "assets": []},
                {"tag_name": "v0.0.1", "body": null, "html_url": "u", "assets": []}
            ]"#,
        )
        .expect("the fixture parses")
        .expect("a release is found");
        assert_eq!(
            release.version,
            Version::new(0, 0, 1),
            "a pre-release ranks under its release"
        );
    }

    // ST-UPD-01
    #[test]
    fn a_check_is_due_past_the_interval_with_checks_on() {
        const INTERVAL: u64 = 120 * 60;
        let now = 2_000_000_000;
        let mut common = CommonSettings::default();
        assert_eq!(
            common.check_interval_minutes, 120,
            "the interval the constants below assume"
        );
        common.check_for_updates = false;
        assert!(!check_due(&common, now), "checks off");
        common.check_for_updates = true;
        assert!(check_due(&common, now), "never checked");
        common.last_update_check = Some(now - 119 * 60);
        assert!(!check_due(&common, now), "a stamp 119 minutes old");
        common.last_update_check = Some(now - INTERVAL + 1);
        assert!(!check_due(&common, now), "inside the interval");
        common.last_update_check = Some(now - INTERVAL);
        assert!(check_due(&common, now), "a stamp 120 minutes old");
        common.last_update_check = Some(1);
        assert!(check_due(&common, now), "long past");
    }
}
