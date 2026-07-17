use crate::error::{GiteeError, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Env var carrying a Gitee PAT. Highest precedence (CI / headless), mirroring
/// `gh`'s `GH_TOKEN`. One token, host-agnostic.
const ENV_TOKEN: &str = "GITEE_TOKEN";
/// OS keyring service name; the account is the host (e.g. `gitee.com`).
const KEYRING_SERVICE: &str = "gitee-cli";

#[derive(Debug, Clone, Copy)]
pub enum TokenSource {
    Env,
    Keyring,
    File,
}

impl TokenSource {
    pub fn as_str(self) -> &'static str {
        match self {
            TokenSource::Env => "$GITEE_TOKEN",
            TokenSource::Keyring => "keyring",
            TokenSource::File => "token file",
        }
    }
}

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

    /// Resolve the active token. Precedence (mirrors `gh`):
    ///   1. `$GITEE_TOKEN` env var   — CI / headless
    ///   2. OS keyring               — interactive default (encrypted)
    ///   3. plaintext file           — fallback when keyring is unavailable
    pub fn token(host: &str) -> Result<String> {
        if let Ok(t) = std::env::var(ENV_TOKEN) {
            let t = t.trim().to_string();
            if !t.is_empty() {
                return Ok(t);
            }
        }
        if let Ok(t) = keyring_get(host) {
            return Ok(t);
        }
        match fs::read_to_string(Self::token_path(host)?) {
            Ok(s) => {
                let s = s.trim().to_string();
                if s.is_empty() {
                    Err(GiteeError::NotLoggedIn)
                } else {
                    Ok(s)
                }
            }
            Err(_) => Err(GiteeError::NotLoggedIn),
        }
    }

    /// Where the currently active token comes from, if any (for `auth status`).
    pub fn locate(host: &str) -> Option<TokenSource> {
        if std::env::var(ENV_TOKEN)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
        {
            return Some(TokenSource::Env);
        }
        if keyring_get(host).is_ok() {
            return Some(TokenSource::Keyring);
        }
        let Ok(p) = Self::token_path(host) else {
            return None;
        };
        match fs::read_to_string(&p) {
            Ok(s) if !s.trim().is_empty() => Some(TokenSource::File),
            _ => None,
        }
    }

    /// Store a token. Writes to the OS keyring when available; otherwise falls
    /// back to a plaintext file (chmod 600) so headless boxes still work.
    pub fn set_token(host: &str, token: &str) -> Result<()> {
        match keyring::Entry::new(KEYRING_SERVICE, host).and_then(|e| e.set_password(token)) {
            Ok(()) => {
                // Active secret now lives only in the keyring; drop any stale file.
                let _ = fs::remove_file(Self::token_path(host)?);
                Ok(())
            }
            Err(_) => {
                let p = Self::token_path(host)?;
                if let Some(parent) = p.parent() {
                    fs::create_dir_all(parent).map_err(|e| GiteeError::Config(e.to_string()))?;
                }
                fs::write(&p, token).map_err(|e| GiteeError::Config(e.to_string()))?;
                restrict_perms(&p)?;
                Ok(())
            }
        }
    }

    /// Forget the token for a host, in both keyring and file.
    pub fn logout(host: &str) -> Result<()> {
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, host) {
            let _ = entry.delete_credential();
        }
        match fs::remove_file(Self::token_path(host)?) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(GiteeError::Config(e.to_string())),
        }
    }
}

fn keyring_get(host: &str) -> std::result::Result<String, ()> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, host).map_err(|_| ())?;
    entry.get_password().map_err(|_| ())
}

#[cfg(unix)]
fn restrict_perms(p: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(p, fs::Permissions::from_mode(0o600))
        .map_err(|e| GiteeError::Config(e.to_string()))
}

#[cfg(not(unix))]
fn restrict_perms(_p: &Path) -> Result<()> {
    Ok(())
}
