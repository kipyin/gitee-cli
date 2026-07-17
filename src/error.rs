use thiserror::Error;

pub type Result<T> = std::result::Result<T, GiteeError>;

#[derive(Error, Debug)]
pub enum GiteeError {
    #[error("gitee API error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("http request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("config error: {0}")]
    Config(String),
    #[error("not logged in: run `gitee auth login --token <TOKEN>` first")]
    NotLoggedIn,
    #[error("could not determine repository (pass --repo owner/repo): {0}")]
    RepoResolve(String),
    #[error("{0}")]
    Usage(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
