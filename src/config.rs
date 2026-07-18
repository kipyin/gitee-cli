use crate::error::{GiteeError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Env var carrying a Gitee PAT. Highest precedence (CI / headless), mirroring
/// `gh`'s `GH_TOKEN`. One token, host-agnostic.
const ENV_TOKEN: &str = "GITEE_TOKEN";
/// Override config directory (tests / portable installs).
const ENV_CONFIG_DIR: &str = "GITEE_CONFIG_DIR";
/// OS keyring service name; the account is the host (e.g. `gitee.com`).
const KEYRING_SERVICE: &str = "gitee-cli";

const CONFIG_KEYS: &[&str] = &["host", "remote", "editor"];

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub aliases: BTreeMap<String, String>,
}

pub struct Config;

impl Config {
    pub fn dir() -> Result<PathBuf> {
        if let Ok(p) = std::env::var(ENV_CONFIG_DIR) {
            let p = p.trim();
            if !p.is_empty() {
                return Ok(PathBuf::from(p));
            }
        }
        let base = dirs::config_dir()
            .ok_or_else(|| GiteeError::Config("no config directory available".into()))?;
        Ok(base.join("gitee"))
    }

    fn token_path(host: &str) -> Result<PathBuf> {
        Ok(Self::dir()?.join(format!("{host}.token")))
    }

    fn settings_path() -> Result<PathBuf> {
        Ok(Self::dir()?.join("config.json"))
    }

    pub fn load_settings() -> Result<Settings> {
        let p = Self::settings_path()?;
        match fs::read_to_string(&p) {
            Ok(s) if s.trim().is_empty() => Ok(Settings::default()),
            Ok(s) => serde_json::from_str(&s)
                .map_err(|e| GiteeError::Config(format!("invalid config.json: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(e) => Err(GiteeError::Config(e.to_string())),
        }
    }

    pub fn save_settings(settings: &Settings) -> Result<()> {
        let p = Self::settings_path()?;
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| GiteeError::Config(e.to_string()))?;
        }
        let body = serde_json::to_string_pretty(settings)
            .map_err(|e| GiteeError::Config(e.to_string()))?;
        fs::write(&p, body + "\n").map_err(|e| GiteeError::Config(e.to_string()))?;
        restrict_perms(&p)?;
        Ok(())
    }

    pub fn get_key(key: &str) -> Result<Option<String>> {
        validate_config_key(key)?;
        let s = Self::load_settings()?;
        Ok(match key {
            "host" => s.host,
            "remote" => s.remote,
            "editor" => s.editor,
            _ => None,
        })
    }

    pub fn set_key(key: &str, value: &str) -> Result<()> {
        validate_config_key(key)?;
        let mut s = Self::load_settings()?;
        let v = value.to_string();
        match key {
            "host" => s.host = Some(v),
            "remote" => s.remote = Some(v),
            "editor" => s.editor = Some(v),
            _ => unreachable!(),
        }
        Self::save_settings(&s)
    }

    pub fn list_keys() -> Result<Vec<(String, String)>> {
        let s = Self::load_settings()?;
        let mut out = Vec::new();
        if let Some(v) = s.host {
            out.push(("host".into(), v));
        }
        if let Some(v) = s.remote {
            out.push(("remote".into(), v));
        }
        if let Some(v) = s.editor {
            out.push(("editor".into(), v));
        }
        Ok(out)
    }

    pub fn alias_set(name: &str, expansion: &str) -> Result<()> {
        validate_alias_name(name)?;
        let mut s = Self::load_settings()?;
        s.aliases.insert(name.to_string(), expansion.to_string());
        Self::save_settings(&s)
    }

    pub fn alias_delete(name: &str) -> Result<()> {
        let mut s = Self::load_settings()?;
        if s.aliases.remove(name).is_none() {
            return Err(GiteeError::Usage(format!("alias not found: {name}")));
        }
        Self::save_settings(&s)
    }

    pub fn alias_list() -> Result<Vec<(String, String)>> {
        let s = Self::load_settings()?;
        Ok(s.aliases.into_iter().collect())
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

fn validate_config_key(key: &str) -> Result<()> {
    if CONFIG_KEYS.contains(&key) {
        Ok(())
    } else {
        Err(GiteeError::Usage(format!(
            "unknown config key '{key}'; expected one of {}",
            CONFIG_KEYS.join(", ")
        )))
    }
}

fn validate_alias_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.starts_with('-')
        || name.contains('/')
        || name.contains(' ')
        || name.contains('\\')
    {
        return Err(GiteeError::Usage(format!("invalid alias name: {name}")));
    }
    Ok(())
}

/// Expand the first argv token when it matches an alias. Re-expands until the
/// command token is not an alias. Errors on cycles / self-recursion.
pub fn expand_aliases(argv: Vec<String>, aliases: &BTreeMap<String, String>) -> Result<Vec<String>> {
    if argv.len() < 2 {
        return Ok(argv);
    }
    let mut out = argv;
    let mut seen = std::collections::HashSet::new();
    loop {
        // Skip global flags before the command token.
        let cmd_idx = match first_command_index(&out) {
            Some(i) => i,
            None => return Ok(out),
        };
        let name = out[cmd_idx].clone();
        let Some(expansion) = aliases.get(&name) else {
            return Ok(out);
        };
        if !seen.insert(name.clone()) {
            return Err(GiteeError::Usage(format!(
                "alias cycle detected at '{name}'"
            )));
        }
        let expanded = shell_words::split(expansion)
            .map_err(|e| GiteeError::Usage(format!("alias '{name}' expansion: {e}")))?;
        if expanded.is_empty() {
            return Err(GiteeError::Usage(format!("alias '{name}' expands to empty")));
        }
        if expanded[0] == name {
            return Err(GiteeError::Usage(format!(
                "alias '{name}' expands to itself"
            )));
        }
        // Replace the command token with the expansion words; keep trailing args.
        let mut next = out[..cmd_idx].to_vec();
        next.extend(expanded);
        next.extend(out[cmd_idx + 1..].to_vec());
        out = next;
    }
}

/// Inject `--host` / `--remote` from settings when the user did not pass them.
pub fn apply_defaults(mut argv: Vec<String>, settings: &Settings) -> Vec<String> {
    if let Some(host) = &settings.host {
        if !has_long_opt(&argv, "host") {
            argv.insert(1, format!("--host={host}"));
        }
    }
    if let Some(remote) = &settings.remote {
        if !has_long_opt(&argv, "remote") {
            argv.insert(1, format!("--remote={remote}"));
        }
    }
    argv
}

fn has_long_opt(argv: &[String], name: &str) -> bool {
    let prefixed = format!("--{name}");
    let eq = format!("--{name}=");
    argv.iter()
        .any(|a| a == &prefixed || a.starts_with(&eq))
}

fn first_command_index(argv: &[String]) -> Option<usize> {
    // argv[0] is program name. Skip global options and their values.
    let mut i = 1;
    while i < argv.len() {
        let a = &argv[i];
        if a == "--" {
            return Some(i + 1).filter(|&j| j < argv.len());
        }
        if a.starts_with("--") {
            if a.contains('=') {
                i += 1;
                continue;
            }
            // known globals that take a value
            if matches!(
                a.as_str(),
                "--host" | "--remote" | "--repo" | "--json" | "-j" | "--jq"
            ) {
                i += 2;
                continue;
            }
            // boolean globals: --debug, bare --json already handled via default_missing
            i += 1;
            continue;
        }
        if a.starts_with('-') && a.len() == 2 {
            // short -j may take optional value; treat next non-flag as value when present
            if a == "-j" {
                if i + 1 < argv.len() && !argv[i + 1].starts_with('-') {
                    i += 2;
                } else {
                    i += 1;
                }
                continue;
            }
            i += 1;
            continue;
        }
        return Some(i);
    }
    None
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn expand_alias_first_token_with_trailing_args() {
        let mut aliases = BTreeMap::new();
        aliases.insert("co".into(), "pr checkout".into());
        let argv = vec![
            "gitee".into(),
            "co".into(),
            "12".into(),
        ];
        let out = expand_aliases(argv, &aliases).unwrap();
        assert_eq!(out, vec!["gitee", "pr", "checkout", "12"]);
    }

    #[test]
    fn expand_alias_shell_quoting() {
        let mut aliases = BTreeMap::new();
        aliases.insert("x".into(), r#"issue create --title "hello world""#.into());
        let argv = vec!["gitee".into(), "x".into()];
        let out = expand_aliases(argv, &aliases).unwrap();
        assert_eq!(
            out,
            vec!["gitee", "issue", "create", "--title", "hello world"]
        );
    }

    #[test]
    fn expand_alias_self_recursion_errors() {
        let mut aliases = BTreeMap::new();
        aliases.insert("co".into(), "co 1".into());
        let err = expand_aliases(vec!["gitee".into(), "co".into()], &aliases).unwrap_err();
        assert!(err.to_string().contains("itself") || err.to_string().contains("cycle"));
    }

    #[test]
    fn expand_alias_cycle_errors() {
        let mut aliases = BTreeMap::new();
        aliases.insert("a".into(), "b".into());
        aliases.insert("b".into(), "a".into());
        let err = expand_aliases(vec!["gitee".into(), "a".into()], &aliases).unwrap_err();
        assert!(err.to_string().contains("cycle"));
    }

    #[test]
    fn settings_round_trip_in_temp_dir() {
        let dir = std::env::temp_dir().join(format!(
            "gitee-cli-config-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        std::env::set_var(ENV_CONFIG_DIR, &dir);
        Config::set_key("host", "gitee.com").unwrap();
        Config::set_key("editor", "vim").unwrap();
        Config::alias_set("co", "pr checkout").unwrap();
        assert_eq!(Config::get_key("host").unwrap().as_deref(), Some("gitee.com"));
        assert_eq!(Config::get_key("editor").unwrap().as_deref(), Some("vim"));
        let aliases = Config::alias_list().unwrap();
        assert_eq!(aliases, vec![("co".into(), "pr checkout".into())]);
        Config::alias_delete("co").unwrap();
        assert!(Config::alias_list().unwrap().is_empty());
        std::env::remove_var(ENV_CONFIG_DIR);
        let _ = fs::remove_dir_all(&dir);
    }
}
