//! Background GitHub Releases check and stderr Update notice tip.

use std::io::{IsTerminal, Write};
use std::thread::JoinHandle;
use std::time::Duration;

/// Production GitHub API base URL.
pub const GITHUB_API_BASE: &str = "https://api.github.com";

const FETCH_TIMEOUT: Duration = Duration::from_secs(2);

/// Latest release fields used for the Update notice tip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    pub version: String,
    pub url: String,
}

/// Background Update notice check started before command work.
pub struct UpdateNotice {
    current: String,
    handle: Option<JoinHandle<Option<ReleaseInfo>>>,
}

impl UpdateNotice {
    /// Spawn a background fetch against `api_base` (e.g. [`GITHUB_API_BASE`]).
    pub fn spawn(current_version: &str, api_base: &str) -> Self {
        let api_base = api_base.to_string();
        let handle = std::thread::spawn(move || fetch_latest(&api_base, FETCH_TIMEOUT));
        Self {
            current: current_version.to_string(),
            handle: Some(handle),
        }
    }

    /// On command success: join the check and maybe write the tip.
    /// Network/decode failures are silent; write errors are ignored.
    pub fn finish_on_success(mut self, w: &mut impl Write) {
        let Some(handle) = self.handle.take() else {
            return;
        };
        let Ok(Some(info)) = handle.join() else {
            return;
        };
        if !is_strictly_newer(&info.version, &self.current) {
            return;
        }
        let current = strip_leading_v(&self.current);
        let tip = format_tip(
            current,
            &info.version,
            &info.url,
            tip_color_enabled(),
        );
        let _ = write!(w, "{tip}");
    }
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
        version: strip_leading_v(&body.tag_name).to_string(),
        url: body.html_url,
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
pub fn format_tip(current: &str, latest: &str, url: &str, color: bool) -> String {
    format!(
        "\n{}{}{}{}\n{}\n\n",
        paint_if(color, "33", "A new release of gitee is available: "),
        paint_if(color, "36", current),
        paint_if(color, "33", " → "),
        paint_if(color, "36", latest),
        paint_if(color, "33", url),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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
        );
        assert_eq!(
            tip,
            "\nA new release of gitee is available: 0.1.5 → 0.2.0\nhttps://github.com/kipyin/gitee-cli/releases/tag/v0.2.0\n\n"
        );
    }

    #[test]
    fn format_tip_color_marks_label_url_yellow_and_versions_cyan() {
        let tip = format_tip(
            "0.1.5",
            "0.2.0",
            "https://github.com/kipyin/gitee-cli/releases/tag/v0.2.0",
            true,
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
}
