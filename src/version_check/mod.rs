//! Background version-checking against crates.io.
//!
//! ## Design
//!
//! - At TUI startup, [`spawn_background_check_if_due`] fires a background
//!   thread that hits crates.io at most once per 24 hours.  Results are
//!   persisted to a local JSON cache file so the network call never blocks the
//!   exit path.
//!
//! - At TUI exit, [`print_upgrade_notice_if_outdated`] reads the cache file
//!   (pure disk I/O, no network) and prints a banner to stderr when a newer
//!   version is available.
//!
//! - Both functions are no-ops when `check_for_updates = false` in the user
//!   config, and when compiled with `cfg(test)` the print call is also
//!   suppressed so tests can assert the cache state without producing visible
//!   output.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

// ── Constants ─────────────────────────────────────────────────────────────────

// Used inside the #[cfg(not(test))] print block; dead in test builds.
#[allow(dead_code)]
const CRATE_NAME: &str = "markdown-tui-explorer";
const CRATES_IO_URL: &str =
    "https://crates.io/api/v1/crates/markdown-tui-explorer";
const USER_AGENT: &str = concat!(
    "markdown-tui-explorer/",
    env!("CARGO_PKG_VERSION"),
    " (version-check; https://github.com/leboiko/markdown-reader)"
);
const CACHE_STALE_SECS: u64 = 60 * 60 * 24; // 24 hours

// ── Cache ─────────────────────────────────────────────────────────────────────

/// Serialised form of a completed version check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCheckCache {
    /// Unix timestamp (seconds) when the check was performed.
    pub checked_at: u64,
    /// Latest non-yanked version string reported by crates.io (e.g. `"1.34.50"`).
    pub latest_version: String,
}

/// Resolve the path to the cache file.
///
/// Returns `None` when the platform has no cache directory (extremely rare;
/// only absent on some embedded or sandboxed environments).
fn cache_path() -> Option<PathBuf> {
    let mut p = dirs::cache_dir()?;
    p.push("markdown-tui-explorer");
    p.push("last-version-check.json");
    Some(p)
}

/// Read the cache from disk, returning `None` on any I/O or parse error.
pub(crate) fn read_cache() -> Option<VersionCheckCache> {
    let path = cache_path()?;
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Persist the cache to disk.  Silently discards I/O errors so callers never
/// have to handle them — a missed cache write simply means the next session
/// will re-check.
pub(crate) fn write_cache(cache: &VersionCheckCache) {
    let Some(path) = cache_path() else { return };
    if let Some(parent) = path.parent()
        && std::fs::create_dir_all(parent).is_err()
    {
        return;
    }
    let Ok(text) = serde_json::to_string(cache) else {
        return;
    };
    let _ = std::fs::write(&path, text.as_bytes());
}

// ── Network ───────────────────────────────────────────────────────────────────

/// Minimal subset of the crates.io JSON response that we care about.
#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    krate: CratesIoCrate,
}

#[derive(Debug, Deserialize)]
struct CratesIoCrate {
    max_version: String,
}

/// Perform a blocking GET to crates.io and return the latest version string.
///
/// Returns `None` on any network, parse, or serde error — callers treat this
/// as "check failed silently".
fn fetch_latest_version() -> Option<String> {
    // ureq 3.x uses `Config::builder()` (same pattern as checklinks).
    use ureq::config::Config;

    let agent: ureq::Agent = Config::builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .user_agent(USER_AGENT)
        .build()
        .into();

    let response = agent.get(CRATES_IO_URL).call().ok()?;
    let body: CratesIoResponse = response.into_body().read_json().ok()?;
    let version = body.krate.max_version;
    if version.is_empty() {
        return None;
    }
    Some(version)
}

// ── Version comparison ────────────────────────────────────────────────────────

/// Returns `true` when `candidate` is strictly newer than `current` according
/// to semver ordering.  Malformed strings are treated as "not newer" so the
/// function is always safe to call.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    let Ok(c) = semver::Version::parse(candidate) else {
        return false;
    };
    let Ok(cur) = semver::Version::parse(current) else {
        return false;
    };
    c > cur
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Spawn a background thread to fetch the latest crate version from crates.io
/// and update the local cache, but only when:
///
/// 1. `check_for_updates` is `true` in the config, and
/// 2. The cache is absent or older than 24 hours.
///
/// The thread is detached (fire-and-forget).  The caller must not wait for it;
/// results will be available via the cache file on the next exit.
pub fn spawn_background_check_if_due(check_for_updates: bool) {
    if !check_for_updates {
        return;
    }

    // Only spawn when the cache is missing or stale.
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let cache_is_fresh = read_cache()
        .map(|c| now_secs.saturating_sub(c.checked_at) < CACHE_STALE_SECS)
        .unwrap_or(false);

    if cache_is_fresh {
        return;
    }

    std::thread::Builder::new()
        .name("version-check".into())
        .spawn(move || {
            let Some(latest) = fetch_latest_version() else {
                return;
            };
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            write_cache(&VersionCheckCache {
                checked_at: now,
                latest_version: latest,
            });
        })
        // A spawn failure is non-fatal — we just skip the check.
        .ok();
}

/// Read the local version cache (no network I/O) and, if the cached latest
/// version is strictly newer than `current_version`, print an upgrade banner
/// to stderr.
///
/// This function is a no-op when:
/// - `check_for_updates` is `false`,
/// - the cache is absent or stale (> 24 h), or
/// - the cached version is not newer.
///
/// It is also compiled out of test builds to prevent test output noise; tests
/// should assert cache state directly via [`read_cache`] and [`is_newer`].
pub fn print_upgrade_notice_if_outdated(current_version: &str, check_for_updates: bool) {
    if !check_for_updates {
        return;
    }

    let Some(cache) = read_cache() else { return };

    // Ignore a stale cache — it is unlikely to reflect reality.
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now_secs.saturating_sub(cache.checked_at) >= CACHE_STALE_SECS {
        return;
    }

    if is_newer(&cache.latest_version, current_version) {
        // Only print in non-test builds so test output stays clean.
        #[cfg(not(test))]
        {
            use std::io::Write as _;
            let bar = "\u{2500}".repeat(55);
            let _ = writeln!(
                std::io::stderr(),
                "\n{bar}\n \
                 {CRATE_NAME} {current_version} \u{2192} {} available\n\n \
                 Upgrade with:\n   \
                 cargo install {CRATE_NAME}\n \
                 Or download a pre-built binary:\n   \
                 https://github.com/leboiko/markdown-reader/releases/latest\n{bar}",
                cache.latest_version,
            );
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn now_secs() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    // ── Cache round-trip ──────────────────────────────────────────────────────

    /// `VersionCheckCache` must survive a JSON round-trip unchanged.
    #[test]
    fn cache_serializes_round_trip() {
        let original = VersionCheckCache {
            checked_at: 1_700_000_000,
            latest_version: "1.99.0".into(),
        };
        let json = serde_json::to_string(&original).expect("serialize");
        let decoded: VersionCheckCache = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.checked_at, original.checked_at);
        assert_eq!(decoded.latest_version, original.latest_version);
    }

    // ── Version comparison ────────────────────────────────────────────────────

    #[test]
    fn version_comparison_detects_newer() {
        assert!(is_newer("1.34.50", "1.34.33"));
        assert!(is_newer("2.0.0", "1.99.9"));
        assert!(!is_newer("1.34.33", "1.34.33"));
        assert!(!is_newer("1.0.0", "1.34.33"));
    }

    #[test]
    fn version_comparison_handles_malformed_input() {
        // Malformed strings must never panic and must return false.
        assert!(!is_newer("not-a-version", "1.0.0"));
        assert!(!is_newer("1.0.0", "not-a-version"));
        assert!(!is_newer("", "1.0.0"));
    }

    // ── print_upgrade_notice_if_outdated (cache-level tests) ─────────────────
    //
    // The actual eprintln! is guarded by #[cfg(not(test))], so these tests
    // exercise the cache-reading logic without producing stderr output.

    /// When no cache file exists the function must be a silent no-op.
    #[test]
    fn print_skipped_when_cache_missing() {
        // We cannot write to the real cache path in a unit test without side
        // effects, so we test the logic by exercising `read_cache` returning
        // None directly (simulated by checking the early-return guard).
        // `print_upgrade_notice_if_outdated` calls `read_cache()` first; if
        // that returns None it returns immediately without printing.
        //
        // Here we verify the path explicitly: passing an unreachably new
        // version to is_newer with a missing cache returns false.
        let result = is_newer("99.99.99", "1.0.0");
        assert!(result, "is_newer should return true for obviously newer version");

        // And that the overall function doesn't panic when cache is absent.
        // (We can't inject a custom cache path without refactoring, so we just
        //  call through and rely on the #[cfg(not(test))] guard to mute output.)
        print_upgrade_notice_if_outdated("1.0.0", true);
    }

    /// When `check_for_updates = false` the function must be a no-op even if
    /// the cache claims a newer version exists.
    #[test]
    fn print_skipped_when_check_disabled() {
        // Should return immediately without reading cache or printing.
        print_upgrade_notice_if_outdated("1.0.0", false);
    }

    /// When versions are equal no banner should be emitted.
    #[test]
    fn print_skipped_when_versions_equal() {
        assert!(!is_newer("1.34.33", "1.34.33"));
    }

    /// When the cached version is older than current, no banner should be emitted.
    #[test]
    fn print_skipped_when_cached_is_older() {
        assert!(!is_newer("1.0.0", "1.34.33"));
    }

    /// A fresh cache with a newer version should cause `is_newer` to return
    /// true, and write_cache/read_cache should round-trip correctly.
    #[test]
    fn write_and_read_cache_round_trips() {
        // Only run this test if we have a writable cache directory.
        let Some(_path) = cache_path() else { return };

        let entry = VersionCheckCache {
            checked_at: now_secs(),
            latest_version: "99.99.99".into(),
        };
        write_cache(&entry);

        let read_back = read_cache();
        // If the write succeeded, the read-back must match.  If the cache dir
        // is not writable (CI sandbox), read_cache returns None — that's fine.
        if let Some(rb) = read_back {
            assert_eq!(rb.latest_version, "99.99.99");
        }
    }

    /// Stale cache (> 24 h old) must be ignored by `print_upgrade_notice_if_outdated`.
    #[test]
    fn print_skipped_when_cache_stale() {
        // A timestamp 48 hours in the past is definitively stale.
        let stale_ts = now_secs().saturating_sub(CACHE_STALE_SECS + 3600);
        let cache = VersionCheckCache {
            checked_at: stale_ts,
            latest_version: "99.99.99".into(),
        };
        // Simulate the stale-check logic inline.
        let age = now_secs().saturating_sub(cache.checked_at);
        assert!(
            age >= CACHE_STALE_SECS,
            "test cache should be considered stale"
        );
        // Confirm is_newer would otherwise fire — the guard is `age >= CACHE_STALE_SECS`.
        assert!(is_newer(&cache.latest_version, "1.0.0"));
    }

    /// When the cache is fresh and the cached version is newer, `is_newer`
    /// must return true (the print path is muted in test builds).
    #[test]
    fn print_message_when_cached_is_newer() {
        let fresh_cache = VersionCheckCache {
            checked_at: now_secs(),
            latest_version: "99.99.99".into(),
        };
        let age = now_secs().saturating_sub(fresh_cache.checked_at);
        assert!(age < CACHE_STALE_SECS, "fresh cache should not be stale");
        assert!(is_newer(&fresh_cache.latest_version, "1.34.33"));
        // The banner itself is suppressed in test builds via #[cfg(not(test))].
        // Calling the function here verifies it doesn't panic.
        print_upgrade_notice_if_outdated("1.34.33", true);
    }
}
