use crate::error::{GiteeError, Result};
use std::fs;
use std::path::PathBuf;

pub struct Config;

impl Config {
    fn dir() -> Result<PathBuf> {
        let base = dirs::config_dir()
            .ok_or_else(|| GiteeError::Config("no config directory available".into()))?;
        Ok(base.join("gitee"))
    }

    fn token_path(host: &str) -> Result<PathBuf> {
        Ok(Self::dir()?.join(format!("{host}.token")))
    }

    pub fn token(host: &str) -> Result<String> {
        let p = Self::token_path(host)?;
        match fs::read_to_string(&p) {
            Ok(s) => Ok(s.trim().to_string()),
            Err(_) => Err(GiteeError::NotLoggedIn),
        }
    }

    pub fn set_token(host: &str, token: &str) -> Result<()> {
        let dir = Self::dir()?;
        fs::create_dir_all(&dir).map_err(|e| GiteeError::Config(e.to_string()))?;
        fs::write(Self::token_path(host)?, token).map_err(|e| GiteeError::Config(e.to_string()))
    }
}
