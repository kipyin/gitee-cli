use thiserror::Error;

pub type Result<T, E = GiteeError> = std::result::Result<T, E>;

#[derive(Error, Debug)]
pub enum GiteeError {
    #[error("gitee API error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("authentication failed (HTTP 401): token is missing, invalid, or expired — run `gitee auth login` (or set GITEE_TOKEN)")]
    Unauthorized,
    #[error("not found (HTTP 404): {0}")]
    NotFound(String),
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("config error: {0}")]
    Config(String),
    #[error("not logged in: run `gitee auth login --token <TOKEN>` first (or set the GITEE_TOKEN env var)")]
    NotLoggedIn,
    #[error("could not determine repository (pass --repo owner/repo): {0}")]
    RepoResolve(String),
    #[error("{0}")]
    Usage(String),
    /// Gitee returned 429 — too many requests. Maps to exit code 5.
    #[error("rate limited (HTTP 429): {0}")]
    RateLimited(String),
    /// Could not reach the Gitee host (DNS failure, connection refused, etc.).
    /// Maps to exit code 6.
    #[error("network error: {0}")]
    Network(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Stable, documented exit codes (see README "Exit codes").
///
/// - `0` success
/// - `1` generic failure (API error, unexpected)
/// - `2` usage error (missing flag, bad arg, non-TTY prompt attempted)
/// - `3` auth error (no token, invalid, expired)
/// - `4` not found (repo/issue/PR/release)
/// - `5` rate limited (HTTP 429)
/// - `6` network error (host unreachable)
impl GiteeError {
    pub fn exit_code(&self) -> i32 {
        use GiteeError::*;
        match self {
            Api { status: 429, .. } | RateLimited(_) => 5,
            Api { status: 401, .. } | Unauthorized | NotLoggedIn => 3,
            Api { status: 404, .. } | NotFound(_) => 4,
            Http(e) if e.is_connect() || e.is_timeout() => 6,
            Network(_) => 6,
            Http(_) => 1,
            Io(_) => 1,
            Config(_) => 2,
            RepoResolve(_) => 2,
            Usage(_) => 2,
            Api { .. } | Other(_) => 1,
        }
    }

    /// Stable, machine-readable `code` slug for `--json` error envelopes.
    pub fn code_slug(&self) -> &'static str {
        use GiteeError::*;
        match self {
            Api { status: 429, .. } | RateLimited(_) => "rate_limited",
            Api { status: 401, .. } | Unauthorized | NotLoggedIn => "auth",
            Api { status: 404, .. } | NotFound(_) => "not_found",
            Http(e) if e.is_connect() || e.is_timeout() => "network",
            Network(_) => "network",
            Http(_) => "http",
            Io(_) => "io",
            Config(_) => "config",
            RepoResolve(_) => "repo_resolve",
            Usage(_) => "usage",
            Api { .. } | Other(_) => "error",
        }
    }
}

#[cfg(test)]
mod exit_code_tests {
    use super::GiteeError;

    #[test]
    fn api_429_maps_to_exit_5() {
        assert_eq!(GiteeError::RateLimited("slow down".into()).exit_code(), 5);
        assert_eq!(
            GiteeError::Api { status: 429, message: "x".into() }.exit_code(),
            5
        );
    }

    #[test]
    fn unauthorized_maps_to_exit_3() {
        assert_eq!(GiteeError::Unauthorized.exit_code(), 3);
        assert_eq!(
            GiteeError::Api { status: 401, message: "x".into() }.exit_code(),
            3
        );
        assert_eq!(GiteeError::NotLoggedIn.exit_code(), 3);
    }

    #[test]
    fn not_found_maps_to_exit_4() {
        assert_eq!(GiteeError::NotFound("repo".into()).exit_code(), 4);
        assert_eq!(
            GiteeError::Api { status: 404, message: "x".into() }.exit_code(),
            4
        );
    }

    #[test]
    fn usage_maps_to_exit_2() {
        assert_eq!(GiteeError::Usage("x".into()).exit_code(), 2);
        assert_eq!(GiteeError::Config("x".into()).exit_code(), 2);
        assert_eq!(
            GiteeError::RepoResolve("x".into()).exit_code(),
            2
        );
    }

    #[test]
    fn generic_api_error_maps_to_exit_1() {
        assert_eq!(
            GiteeError::Api { status: 500, message: "x".into() }.exit_code(),
            1
        );
    }

    #[test]
    fn network_error_maps_to_exit_6() {
        assert_eq!(GiteeError::Network("x".into()).exit_code(), 6);
    }

    #[test]
    fn code_slugs_are_stable() {
        assert_eq!(GiteeError::NotFound("x".into()).code_slug(), "not_found");
        assert_eq!(GiteeError::Unauthorized.code_slug(), "auth");
        assert_eq!(GiteeError::NotLoggedIn.code_slug(), "auth");
        assert_eq!(GiteeError::RateLimited("x".into()).code_slug(), "rate_limited");
        assert_eq!(GiteeError::Usage("x".into()).code_slug(), "usage");
    }
}
