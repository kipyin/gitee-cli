//! Background GitHub Releases check and stderr Update notice tip.

use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::config::{Config, Settings};

/// Production GitHub API base URL.
pub const GITHUB_API_BASE: &str = "https://api.github.com";

/// Session opt-out: any non-empty value skips the Update notice check.
const ENV_NO_UPDATE_NOTIFIER: &str = "GITEE_NO_UPDATE_NOTIFIER";

const FETCH_TIMEOUT: Duration = Duration::from_secs(2);
const CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);
const STATE_FILE: &str = "state.json";

/// Injectable process gates for [`should_run_update_check`] (TTY + `--json`).
#[derive(Debug, Clone, Copy)]
pub struct UpdateCheckGates {
    pub json: bool,
    pub stdout_is_tty: bool,
    pub stderr_is_tty: bool,
}

/// True when `key` is present in the environment with a non-empty value.
fn env_nonempty(key: &str) -> bool {
    std::env::var_os(key).is_some_and(|v| !v.is_empty())
}

/// Whether CI heuristics apply (`CI`, `BUILD_NUMBER`, or `RUN_ID` non-empty).
fn env_is_ci() -> bool {
    env_nonempty("CI") || env_nonempty("BUILD_NUMBER") || env_nonempty("RUN_ID")
}

/// Decide whether this invocation may start an Update notice check.
///
/// Gate order: env opt-out → `--json` → non-TTY stdout/stderr → CI →
/// `CODESPACES` → config `disabled` → run.
pub fn should_run_update_check(gates: &UpdateCheckGates, settings: &Settings) -> bool {
    if env_nonempty(ENV_NO_UPDATE_NOTIFIER) {
        return false;
    }
    if gates.json {
        return false;
    }
    if !gates.stdout_is_tty || !gates.stderr_is_tty {
        return false;
    }
    if env_is_ci() {
        return false;
    }
    if env_nonempty("CODESPACES") {
        return false;
    }
    if settings.update_notifier.as_deref() == Some("disabled") {
        return false;
    }
    true
}

/// Start a background check when skip gates allow; otherwise `None` (no
/// network, no cache read, no tip).
pub fn maybe_spawn(
    current_version: &str,
    api_base: &str,
    gates: &UpdateCheckGates,
    settings: &Settings,
) -> Option<UpdateNotice> {
    if !should_run_update_check(gates, settings) {
        return None;
    }
    Some(UpdateNotice::spawn(current_version, api_base))
}

/// Cached release entry persisted in `state.json` (`version` keeps leading `v`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedRelease {
    pub version: String,
    pub url: String,
    pub published_at: String,
}

/// On-disk Update notice cache (`{Config::dir()}/state.json`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateState {
    pub checked_for_update_at: String,
    pub latest_release: CachedRelease,
}

/// True when `checked_for_update_at` parses as RFC3339 and falls within the
/// last 24 hours before `now`. Invalid or future timestamps are not fresh.
pub fn cache_is_fresh(checked_for_update_at: &str, now: SystemTime) -> bool {
    let Ok(checked) = chrono::DateTime::parse_from_rfc3339(checked_for_update_at) else {
        return false;
    };
    let checked: SystemTime = checked.with_timezone(&chrono::Utc).into();
    match now.duration_since(checked) {
        Ok(age) => age < CACHE_TTL,
        // Future timestamp (clock skew): not within the past 24h.
        Err(_) => false,
    }
}

fn state_path() -> Option<PathBuf> {
    Config::dir().ok().map(|d| d.join(STATE_FILE))
}

/// Load `state.json`. Missing or invalid content ⇒ `None` (check due).
pub fn load_state() -> Option<UpdateState> {
    let path = state_path()?;
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Write `state.json` with the same restricted mode as other config files.
pub fn save_state(state: &UpdateState) -> Result<(), String> {
    let path = state_path().ok_or_else(|| "no config directory".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let body = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    std::fs::write(&path, body + "\n").map_err(|e| e.to_string())?;
    crate::config::restrict_perms(&path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Latest release fields used for the Update notice tip.
/// `version` is the GitHub `tag_name` (keeps a leading `v` when present).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: String,
    pub url: String,
    pub published_at: String,
}

/// Background Update notice check started before command work.
pub struct UpdateNotice {
    current: String,
    /// Present when a fresh cache skipped the network fetch.
    cached: Option<ReleaseInfo>,
    handle: Option<JoinHandle<Option<ReleaseInfo>>>,
    /// Wall clock used when writing `checked_for_update_at`.
    now: SystemTime,
}

impl UpdateNotice {
    /// Spawn a background fetch against `api_base` (e.g. [`GITHUB_API_BASE`]).
    pub fn spawn(current_version: &str, api_base: &str) -> Self {
        Self::spawn_at(current_version, api_base, SystemTime::now())
    }

    /// Like [`Self::spawn`], with an injectable `now` for cache TTL tests.
    pub fn spawn_at(current_version: &str, api_base: &str, now: SystemTime) -> Self {
        if let Some(state) = load_state() {
            if cache_is_fresh(&state.checked_for_update_at, now) {
                return Self {
                    current: current_version.to_string(),
                    cached: Some(ReleaseInfo {
                        version: state.latest_release.version,
                        url: state.latest_release.url,
                        published_at: state.latest_release.published_at,
                    }),
                    handle: None,
                    now,
                };
            }
        }
        let api_base = api_base.to_string();
        let handle = std::thread::spawn(move || fetch_latest(&api_base, FETCH_TIMEOUT));
        Self {
            current: current_version.to_string(),
            cached: None,
            handle: Some(handle),
            now,
        }
    }

    /// On command success: join the check and maybe write the tip.
    /// Network/decode failures are silent; write errors are ignored.
    /// A successful fetch rewrites `state.json`; failed/None leaves it alone.
    pub fn finish_on_success(mut self, w: &mut impl Write) {
        let info = if let Some(handle) = self.handle.take() {
            match handle.join() {
                Ok(Some(info)) => {
                    let state = UpdateState {
                        checked_for_update_at: format_rfc3339(self.now),
                        latest_release: CachedRelease {
                            version: info.version.clone(),
                            url: info.url.clone(),
                            published_at: info.published_at.clone(),
                        },
                    };
                    let _ = save_state(&state);
                    Some(info)
                }
                _ => None,
            }
        } else {
            self.cached.take()
        };

        let Some(info) = info else {
            return;
        };
        if !is_strictly_newer(&info.version, &self.current) {
            return;
        }
        let Some(brew_upgrade) =
            homebrew_tip_mode(detect_homebrew_install(), &info.published_at, self.now)
        else {
            return;
        };
        let tip = format_tip(
            strip_leading_v(&self.current),
            strip_leading_v(&info.version),
            &info.url,
            tip_color_enabled(),
            brew_upgrade,
        );
        let _ = write!(w, "{tip}");
    }
}

/// True when `exe` lives under `{brew_prefix}/bin/` (path-component prefix).
pub fn is_homebrew_install(exe: impl AsRef<Path>, brew_prefix: impl AsRef<Path>) -> bool {
    let bin_dir = brew_prefix.as_ref().join("bin");
    exe.as_ref().starts_with(bin_dir)
}

/// True when `published_at` parses as RFC3339 and falls within the last 24 hours
/// before `now`. Invalid or future timestamps are not within the grace window.
pub fn release_within_homebrew_grace(published_at: &str, now: SystemTime) -> bool {
    // Same strict `< 24h` window as [`cache_is_fresh`].
    cache_is_fresh(published_at, now)
}

/// Tip mode after a newer release is already known.
///
/// - `None` — Homebrew within grace: suppress the entire notice
/// - `Some(true)` — Homebrew outside grace: include the brew upgrade line
/// - `Some(false)` — non-Homebrew: headline + URL only
pub fn homebrew_tip_mode(is_homebrew: bool, published_at: &str, now: SystemTime) -> Option<bool> {
    if !is_homebrew {
        return Some(false);
    }
    if release_within_homebrew_grace(published_at, now) {
        None
    } else {
        Some(true)
    }
}

#[cfg(test)]
static TEST_HOMEBREW_PROBE: std::sync::Mutex<Option<(Option<PathBuf>, Option<PathBuf>)>> =
    std::sync::Mutex::new(None);

/// Inject `(current_exe, brew --prefix)` for tests. `None` either side ⇒ probe failure
/// (non-Homebrew). Pass `None` for the whole override to restore production probing.
#[cfg(test)]
pub fn set_test_homebrew_probe(probe: Option<(Option<PathBuf>, Option<PathBuf>)>) {
    *TEST_HOMEBREW_PROBE
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = probe;
}

fn probe_brew_prefix() -> Option<PathBuf> {
    let output = std::process::Command::new("brew")
        .arg("--prefix")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let prefix = String::from_utf8(output.stdout).ok()?;
    let prefix = prefix.trim();
    if prefix.is_empty() {
        return None;
    }
    Some(PathBuf::from(prefix))
}

/// Runtime Homebrew detect: `current_exe` under `{brew --prefix}/bin/`.
/// Any failure (no brew, prefix fail, exe path fail, mismatch) ⇒ non-Homebrew.
fn detect_homebrew_install() -> bool {
    #[cfg(test)]
    {
        if let Some((exe, prefix)) = TEST_HOMEBREW_PROBE
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
        {
            return match (exe, prefix) {
                (Some(exe), Some(prefix)) => is_homebrew_install(exe, prefix),
                _ => false,
            };
        }
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(prefix) = probe_brew_prefix() else {
        return false;
    };
    is_homebrew_install(exe, prefix)
}

fn format_rfc3339(t: SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Strip one leading `v` or `V` from a release tag (e.g. `v1.2.3` → `1.2.3`).
pub fn strip_leading_v(tag: &str) -> &str {
    tag.strip_prefix(['v', 'V']).unwrap_or(tag)
}

/// Fetch `/repos/kipyin/gitee-cli/releases/latest` from `api_base`.
/// Returns `None` on any network, status, or decode failure (silent).
pub fn fetch_latest(api_base: &str, timeout: Duration) -> Option<ReleaseInfo> {
    #[derive(serde::Deserialize)]
    struct LatestRelease {
        tag_name: String,
        html_url: String,
        published_at: String,
    }

    let base = api_base.trim_end_matches('/');
    let url = format!("{base}/repos/kipyin/gitee-cli/releases/latest");
    let http = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .user_agent(format!("gitee-cli/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let resp = http
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let body: LatestRelease = resp.json().ok()?;
    Some(ReleaseInfo {
        // Persist tag_name as-is (leading `v`); tip/compare strip later.
        version: body.tag_name,
        url: body.html_url,
        published_at: body.published_at,
    })
}

/// True when `remote_tag` (after stripping a leading `v`) is a valid semver
/// strictly greater than `current` (also stripped).
pub fn is_strictly_newer(remote_tag: &str, current: &str) -> bool {
    let Ok(remote) = semver::Version::parse(strip_leading_v(remote_tag)) else {
        return false;
    };
    let Ok(local) = semver::Version::parse(strip_leading_v(current)) else {
        return false;
    };
    remote > local
}

/// Whether ANSI color is allowed for the Update notice tip (stderr stream).
fn tip_color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
}

fn paint_if(enabled: bool, code: &str, s: &str) -> String {
    if enabled {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

/// Format the Update notice tip (leading/trailing blank lines).
/// When `color` is true, label + URL are yellow and version numbers cyan.
/// When `brew_upgrade` is true, inserts a plain (never colored) brew line
/// between the headline and the URL.
pub fn format_tip(
    current: &str,
    latest: &str,
    url: &str,
    color: bool,
    brew_upgrade: bool,
) -> String {
    let headline = format!(
        "{}{}{}{}",
        paint_if(color, "33", "A new release of gitee is available: "),
        paint_if(color, "36", current),
        paint_if(color, "33", " → "),
        paint_if(color, "36", latest),
    );
    let url_line = paint_if(color, "33", url);
    if brew_upgrade {
        format!("\n{headline}\nTo upgrade, run: brew upgrade gitee\n{url_line}\n\n")
    } else {
        format!("\n{headline}\n{url_line}\n\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn ts(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn rfc3339(secs: u64) -> String {
        chrono::DateTime::<chrono::Utc>::from(ts(secs)).to_rfc3339_opts(
            chrono::SecondsFormat::Secs,
            true,
        )
    }

    #[test]
    fn cache_is_fresh_within_24h_boundaries() {
        let checked = rfc3339(1_000_000);
        // exactly 24h later → not fresh (age must be strictly < 24h)
        assert!(!cache_is_fresh(
            &checked,
            ts(1_000_000 + 24 * 60 * 60)
        ));
        // one second under 24h → fresh
        assert!(cache_is_fresh(
            &checked,
            ts(1_000_000 + 24 * 60 * 60 - 1)
        ));
        // well inside window → fresh
        assert!(cache_is_fresh(&checked, ts(1_000_000 + 60)));
        // past 24h → due
        assert!(!cache_is_fresh(
            &checked,
            ts(1_000_000 + 24 * 60 * 60 + 1)
        ));
        // invalid timestamp → due
        assert!(!cache_is_fresh("not-a-timestamp", ts(1_000_000)));
        // future timestamp → due (not within the past 24h)
        assert!(!cache_is_fresh(&rfc3339(1_000_000 + 60), ts(1_000_000)));
    }

    #[test]
    fn load_state_missing_or_invalid_means_due() {
        let _env = crate::config::test_config_env_lock();
        let dir = tempfile::tempdir().unwrap();
        crate::config::set_test_dir(Some(dir.path().to_path_buf()));

        assert!(load_state().is_none(), "missing state.json ⇒ due");

        std::fs::write(dir.path().join("state.json"), "{not-json").unwrap();
        assert!(load_state().is_none(), "invalid state.json ⇒ due");

        std::fs::write(dir.path().join("state.json"), "{}").unwrap();
        assert!(load_state().is_none(), "incomplete state.json ⇒ due");

        crate::config::set_test_dir(None);
    }

    #[test]
    fn save_state_round_trips_locked_json_shape() {
        let _env = crate::config::test_config_env_lock();
        let dir = tempfile::tempdir().unwrap();
        crate::config::set_test_dir(Some(dir.path().to_path_buf()));

        let state = UpdateState {
            checked_for_update_at: "2026-01-15T12:00:00Z".into(),
            latest_release: CachedRelease {
                version: "v0.2.0".into(),
                url: "https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0".into(),
                published_at: "2026-01-15T11:00:00Z".into(),
            },
        };
        save_state(&state).expect("save");

        let path = dir.path().join("state.json");
        let raw = std::fs::read_to_string(&path).unwrap();
        let v: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["checked_for_update_at"], "2026-01-15T12:00:00Z");
        assert_eq!(v["latest_release"]["version"], "v0.2.0");
        assert_eq!(
            v["latest_release"]["url"],
            "https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0"
        );
        assert_eq!(v["latest_release"]["published_at"], "2026-01-15T11:00:00Z");

        assert_eq!(load_state().as_ref(), Some(&state));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "state.json mode should match other config files");
        }

        crate::config::set_test_dir(None);
    }

    fn seed_state(dir: &std::path::Path, checked_at: &str, version: &str) {
        seed_state_with_published(dir, checked_at, version, "2026-01-15T11:00:00Z");
    }

    fn seed_state_with_published(
        dir: &std::path::Path,
        checked_at: &str,
        version: &str,
        published_at: &str,
    ) {
        let state = UpdateState {
            checked_for_update_at: checked_at.into(),
            latest_release: CachedRelease {
                version: version.into(),
                url: format!("https://github.com/kipyin/gitee-cli/releases/tag/{version}"),
                published_at: published_at.into(),
            },
        };
        let body = serde_json::to_string_pretty(&state).unwrap();
        std::fs::write(dir.join("state.json"), body + "\n").unwrap();
    }

    #[test]
    fn fresh_cache_skips_network_and_tips_from_cache() {
        let _env = crate::config::test_config_env_lock();
        let dir = tempfile::tempdir().unwrap();
        crate::config::set_test_dir(Some(dir.path().to_path_buf()));

        let now = ts(1_000_000);
        seed_state(dir.path(), &rfc3339(1_000_000 - 60), "v0.2.0");

        // Any HTTP here would panic: no mockito server is listening on this base.
        let notice = UpdateNotice::spawn_at("0.1.5", "http://127.0.0.1:1", now);
        let mut buf = Vec::new();
        notice.finish_on_success(&mut buf);

        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.contains("A new release of gitee is available: 0.1.5 → 0.2.0"),
            "expected tip from cache, got {out:?}"
        );
        // Cache file left unchanged (no rewrite on cache hit).
        let loaded = load_state().unwrap();
        assert_eq!(loaded.checked_for_update_at, rfc3339(1_000_000 - 60));

        crate::config::set_test_dir(None);
    }

    #[test]
    fn stale_cache_hits_network_and_rewrites_state() {
        let _env = crate::config::test_config_env_lock();
        let dir = tempfile::tempdir().unwrap();
        crate::config::set_test_dir(Some(dir.path().to_path_buf()));

        let now = ts(1_000_000);
        // Stale: checked 25h ago.
        seed_state(
            dir.path(),
            &rfc3339(1_000_000 - 25 * 60 * 60),
            "v0.1.9",
        );

        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/repos/kipyin/gitee-cli/releases/latest")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{
  "tag_name": "v0.2.0",
  "html_url": "https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0",
  "published_at": "2026-01-15T12:00:00Z"
}"#,
            )
            .create();

        let notice = UpdateNotice::spawn_at("0.1.5", &server.url(), now);
        std::thread::sleep(Duration::from_millis(50));
        let mut buf = Vec::new();
        notice.finish_on_success(&mut buf);

        mock.assert();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("0.1.5 → 0.2.0"), "got {out:?}");

        let loaded = load_state().unwrap();
        assert_eq!(loaded.checked_for_update_at, rfc3339(1_000_000));
        assert_eq!(loaded.latest_release.version, "v0.2.0");
        assert_eq!(
            loaded.latest_release.published_at,
            "2026-01-15T12:00:00Z"
        );

        crate::config::set_test_dir(None);
    }

    #[test]
    fn failed_fetch_leaves_prior_state_unchanged() {
        let _env = crate::config::test_config_env_lock();
        let dir = tempfile::tempdir().unwrap();
        crate::config::set_test_dir(Some(dir.path().to_path_buf()));

        let now = ts(1_000_000);
        let prior_checked = rfc3339(1_000_000 - 25 * 60 * 60);
        seed_state(dir.path(), &prior_checked, "v0.2.0");
        let prior_raw = std::fs::read_to_string(dir.path().join("state.json")).unwrap();

        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/repos/kipyin/gitee-cli/releases/latest")
            .with_status(500)
            .with_body("oops")
            .create();

        let notice = UpdateNotice::spawn_at("0.1.5", &server.url(), now);
        std::thread::sleep(Duration::from_millis(50));
        let mut buf = Vec::new();
        notice.finish_on_success(&mut buf);

        mock.assert();
        assert!(buf.is_empty(), "failed fetch must not tip");
        let after = std::fs::read_to_string(dir.path().join("state.json")).unwrap();
        assert_eq!(after, prior_raw, "prior good state must not be overwritten");

        crate::config::set_test_dir(None);
    }

    #[test]
    fn strip_leading_v_removes_one_v_or_v() {
        assert_eq!(strip_leading_v("v1.2.3"), "1.2.3");
        assert_eq!(strip_leading_v("V0.1.5"), "0.1.5");
        assert_eq!(strip_leading_v("1.2.3"), "1.2.3");
        assert_eq!(strip_leading_v("vv1.0.0"), "v1.0.0");
    }

    #[test]
    fn is_strictly_newer_compares_semver_after_v_strip() {
        assert!(is_strictly_newer("v0.2.0", "0.1.5"));
        assert!(is_strictly_newer("0.1.6", "v0.1.5"));
        assert!(!is_strictly_newer("v0.1.5", "0.1.5"));
        assert!(!is_strictly_newer("v0.1.4", "0.1.5"));
        assert!(!is_strictly_newer("not-a-version", "0.1.5"));
        assert!(!is_strictly_newer("v0.2.0", "also-bad"));
    }

    #[test]
    fn format_tip_plain_has_padding_copy_and_url() {
        let tip = format_tip(
            "0.1.5",
            "0.2.0",
            "https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0",
            false,
            false,
        );
        assert_eq!(
            tip,
            "\nA new release of gitee is available: 0.1.5 → 0.2.0\nhttps://github.com/kipyin/gitee-cli/releases/tag/v0.2.0\n\n"
        );
    }

    #[test]
    fn format_tip_includes_plain_brew_line_when_requested() {
        let tip = format_tip(
            "0.1.5",
            "0.2.0",
            "https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0",
            true,
            true,
        );
        let yellow = |s: &str| format!("\x1b[33m{s}\x1b[0m");
        let cyan = |s: &str| format!("\x1b[36m{s}\x1b[0m");
        let expected = format!(
            "\n{}{}{}{}\nTo upgrade, run: brew upgrade gitee\n{}\n\n",
            yellow("A new release of gitee is available: "),
            cyan("0.1.5"),
            yellow(" → "),
            cyan("0.2.0"),
            yellow("https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0"),
        );
        assert_eq!(tip, expected);
        assert!(
            !tip.contains("\x1b[33mTo upgrade"),
            "brew line must stay uncolored"
        );
    }

    #[test]
    fn format_tip_color_marks_label_url_yellow_and_versions_cyan() {
        let tip = format_tip(
            "0.1.5",
            "0.2.0",
            "https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0",
            true,
            false,
        );
        let yellow = |s: &str| format!("\x1b[33m{s}\x1b[0m");
        let cyan = |s: &str| format!("\x1b[36m{s}\x1b[0m");
        let expected = format!(
            "\n{}{}{}{}\n{}\n\n",
            yellow("A new release of gitee is available: "),
            cyan("0.1.5"),
            yellow(" → "),
            cyan("0.2.0"),
            yellow("https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0"),
        );
        assert_eq!(tip, expected);
    }

    #[test]
    fn is_homebrew_install_requires_prefix_bin_path() {
        assert!(is_homebrew_install(
            "/opt/homebrew/bin/gitee",
            "/opt/homebrew"
        ));
        assert!(is_homebrew_install(
            "/usr/local/bin/gitee",
            "/usr/local"
        ));
        assert!(
            !is_homebrew_install("/opt/homebrew/Cellar/gitee/0.2.0/bin/gitee", "/opt/homebrew"),
            "Cellar path alone is not Homebrew for tip purposes"
        );
        assert!(!is_homebrew_install(
            "/home/user/.cargo/bin/gitee",
            "/opt/homebrew"
        ));
        assert!(!is_homebrew_install(
            "/opt/homebrew/binfoo/gitee",
            "/opt/homebrew"
        ));
    }

    #[test]
    fn release_within_homebrew_grace_matches_24h_strict_window() {
        let published = rfc3339(1_000_000);
        assert!(!release_within_homebrew_grace(
            &published,
            ts(1_000_000 + 24 * 60 * 60)
        ));
        assert!(release_within_homebrew_grace(
            &published,
            ts(1_000_000 + 24 * 60 * 60 - 1)
        ));
        assert!(release_within_homebrew_grace(
            &published,
            ts(1_000_000 + 60)
        ));
        assert!(!release_within_homebrew_grace(
            &published,
            ts(1_000_000 + 24 * 60 * 60 + 1)
        ));
        assert!(!release_within_homebrew_grace(
            "not-a-timestamp",
            ts(1_000_000)
        ));
        assert!(!release_within_homebrew_grace(
            &rfc3339(1_000_000 + 60),
            ts(1_000_000)
        ));
    }

    #[test]
    fn homebrew_tip_mode_suppresses_brew_or_plain() {
        let now = ts(1_000_000);
        let recent = rfc3339(1_000_000 - 60);
        let older = rfc3339(1_000_000 - 25 * 60 * 60);

        assert_eq!(homebrew_tip_mode(false, &recent, now), Some(false));
        assert_eq!(homebrew_tip_mode(false, &older, now), Some(false));
        assert_eq!(homebrew_tip_mode(true, &recent, now), None);
        assert_eq!(homebrew_tip_mode(true, &older, now), Some(true));
        assert_eq!(
            homebrew_tip_mode(true, "not-a-timestamp", now),
            Some(true),
            "invalid published_at is outside grace"
        );
    }

    #[test]
    fn finish_on_success_homebrew_brew_line_and_grace_via_inject() {
        let _env = crate::config::test_config_env_lock();
        let dir = tempfile::tempdir().unwrap();
        crate::config::set_test_dir(Some(dir.path().to_path_buf()));

        let now = ts(1_000_000);
        let brew_prefix = PathBuf::from("/opt/homebrew");
        let brew_exe = PathBuf::from("/opt/homebrew/bin/gitee");

        // Outside grace + Homebrew → tip includes brew line.
        seed_state_with_published(
            dir.path(),
            &rfc3339(1_000_000 - 60),
            "v0.2.0",
            &rfc3339(1_000_000 - 25 * 60 * 60),
        );
        set_test_homebrew_probe(Some((Some(brew_exe.clone()), Some(brew_prefix.clone()))));
        let notice = UpdateNotice::spawn_at("0.1.5", "http://127.0.0.1:1", now);
        let mut buf = Vec::new();
        notice.finish_on_success(&mut buf);
        let out = String::from_utf8(buf).unwrap();
        assert!(
            out.contains("To upgrade, run: brew upgrade gitee"),
            "expected brew line, got {out:?}"
        );

        // Within grace + Homebrew → suppress entire notice.
        seed_state_with_published(
            dir.path(),
            &rfc3339(1_000_000 - 60),
            "v0.2.0",
            &rfc3339(1_000_000 - 60),
        );
        let notice = UpdateNotice::spawn_at("0.1.5", "http://127.0.0.1:1", now);
        let mut buf = Vec::new();
        notice.finish_on_success(&mut buf);
        assert!(
            buf.is_empty(),
            "Homebrew grace must suppress tip; got {:?}",
            String::from_utf8_lossy(&buf)
        );

        // Probe failure → non-Homebrew tip (no brew line, still shown).
        set_test_homebrew_probe(Some((Some(brew_exe), None)));
        let notice = UpdateNotice::spawn_at("0.1.5", "http://127.0.0.1:1", now);
        let mut buf = Vec::new();
        notice.finish_on_success(&mut buf);
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("0.1.5 → 0.2.0"), "got {out:?}");
        assert!(
            !out.contains("brew upgrade"),
            "probe failure must omit brew line"
        );

        set_test_homebrew_probe(None);
        crate::config::set_test_dir(None);
    }

    const SKIP_ENV_KEYS: &[&str] = &[
        "GITEE_NO_UPDATE_NOTIFIER",
        "CI",
        "BUILD_NUMBER",
        "RUN_ID",
        "CODESPACES",
    ];

    /// Clear skip-related env vars for the duration of `f`, then restore.
    fn with_cleared_skip_env<T>(f: impl FnOnce() -> T) -> T {
        let prev: Vec<_> = SKIP_ENV_KEYS
            .iter()
            .map(|k| (*k, std::env::var_os(k)))
            .collect();
        for k in SKIP_ENV_KEYS {
            std::env::remove_var(k);
        }
        let out = f();
        for (k, v) in prev {
            match v {
                Some(v) => std::env::set_var(k, v),
                None => std::env::remove_var(k),
            }
        }
        out
    }

    fn set_env(key: &str, value: Option<&str>) {
        match value {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }

    fn interactive_gates() -> UpdateCheckGates {
        UpdateCheckGates {
            json: false,
            stdout_is_tty: true,
            stderr_is_tty: true,
        }
    }

    #[test]
    fn should_run_update_check_matrix() {
        let _env = crate::config::test_config_env_lock();

        #[derive(Clone, Copy)]
        struct Case {
            name: &'static str,
            env_no_update: Option<&'static str>,
            json: bool,
            stdout_tty: bool,
            stderr_tty: bool,
            ci: Option<&'static str>,
            build_number: Option<&'static str>,
            run_id: Option<&'static str>,
            codespaces: Option<&'static str>,
            update_notifier: Option<&'static str>,
            expect: bool,
        }

        let cases = [
            Case {
                name: "default interactive enabled",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: None,
                expect: true,
            },
            Case {
                name: "config enabled explicit",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: Some("enabled"),
                expect: true,
            },
            Case {
                name: "env non-empty skips",
                env_no_update: Some("1"),
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: Some("enabled"),
                expect: false,
            },
            Case {
                name: "env empty string does not skip",
                env_no_update: Some(""),
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: None,
                expect: true,
            },
            Case {
                name: "json skips",
                env_no_update: None,
                json: true,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: None,
                expect: false,
            },
            Case {
                name: "stdout non-tty skips",
                env_no_update: None,
                json: false,
                stdout_tty: false,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: None,
                expect: false,
            },
            Case {
                name: "stderr non-tty skips",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: false,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: None,
                expect: false,
            },
            Case {
                name: "CI set skips",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: Some("true"),
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: None,
                expect: false,
            },
            Case {
                name: "CI empty string does not skip",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: Some(""),
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: None,
                expect: true,
            },
            Case {
                name: "BUILD_NUMBER set skips",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: Some("42"),
                run_id: None,
                codespaces: None,
                update_notifier: None,
                expect: false,
            },
            Case {
                name: "BUILD_NUMBER empty string does not skip",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: Some(""),
                run_id: None,
                codespaces: None,
                update_notifier: None,
                expect: true,
            },
            Case {
                name: "RUN_ID set skips",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: Some("run-1"),
                codespaces: None,
                update_notifier: None,
                expect: false,
            },
            Case {
                name: "RUN_ID empty string does not skip",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: Some(""),
                codespaces: None,
                update_notifier: None,
                expect: true,
            },
            Case {
                name: "CODESPACES non-empty skips",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: Some("true"),
                update_notifier: None,
                expect: false,
            },
            Case {
                name: "CODESPACES empty string does not skip",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: Some(""),
                update_notifier: None,
                expect: true,
            },
            Case {
                name: "config disabled skips",
                env_no_update: None,
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: Some("disabled"),
                expect: false,
            },
            Case {
                name: "env wins over config enabled",
                env_no_update: Some("yes"),
                json: false,
                stdout_tty: true,
                stderr_tty: true,
                ci: None,
                build_number: None,
                run_id: None,
                codespaces: None,
                update_notifier: Some("enabled"),
                expect: false,
            },
        ];

        for case in cases {
            with_cleared_skip_env(|| {
                set_env("GITEE_NO_UPDATE_NOTIFIER", case.env_no_update);
                set_env("CI", case.ci);
                set_env("BUILD_NUMBER", case.build_number);
                set_env("RUN_ID", case.run_id);
                set_env("CODESPACES", case.codespaces);

                let settings = Settings {
                    update_notifier: case.update_notifier.map(str::to_string),
                    ..Settings::default()
                };
                let gates = UpdateCheckGates {
                    json: case.json,
                    stdout_is_tty: case.stdout_tty,
                    stderr_is_tty: case.stderr_tty,
                };
                let got = should_run_update_check(&gates, &settings);
                assert_eq!(
                    got, case.expect,
                    "case {:?}: expected {}, got {}",
                    case.name, case.expect, got
                );
            });
        }

        // Clearing env leaves config in charge: disabled still skips; enabled runs.
        with_cleared_skip_env(|| {
            set_env("GITEE_NO_UPDATE_NOTIFIER", Some("1"));
            let settings = Settings {
                update_notifier: Some("disabled".into()),
                ..Settings::default()
            };
            assert!(!should_run_update_check(&interactive_gates(), &settings));
            set_env("GITEE_NO_UPDATE_NOTIFIER", None);
            assert!(!should_run_update_check(&interactive_gates(), &settings));

            let enabled = Settings {
                update_notifier: Some("enabled".into()),
                ..Settings::default()
            };
            assert!(should_run_update_check(&interactive_gates(), &enabled));
        });
    }

    #[test]
    fn maybe_spawn_skip_does_not_hit_http() {
        let _env = crate::config::test_config_env_lock();
        let dir = tempfile::tempdir().unwrap();
        crate::config::set_test_dir(Some(dir.path().to_path_buf()));

        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/repos/kipyin/gitee-cli/releases/latest")
            .with_status(200)
            .with_body(
                r#"{"tag_name":"v0.2.0","html_url":"https://example.com","published_at":"2026-01-15T12:00:00Z"}"#,
            )
            .expect(0)
            .create();

        with_cleared_skip_env(|| {
            set_env("GITEE_NO_UPDATE_NOTIFIER", Some("1"));
            let settings = Settings::default();
            let gates = interactive_gates();
            let notice = maybe_spawn("0.1.5", &server.url(), &gates, &settings);
            assert!(notice.is_none(), "skip must not spawn UpdateNotice");
        });

        mock.assert();
        crate::config::set_test_dir(None);
    }
}
