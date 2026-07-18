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
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
