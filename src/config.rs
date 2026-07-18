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
    /// Active username per host (ticket 17).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub active_users: BTreeMap<String, String>,
    /// Known usernames per host that have stored tokens.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub known_users: BTreeMap<String, Vec<String>>,
}

pub struct Config;

#[cfg(test)]
static TEST_CONFIG_DIR: std::sync::Mutex<Option<PathBuf>> = std::sync::Mutex::new(None);

#[cfg(test)]
pub fn set_test_dir(dir: Option<PathBuf>) {
    *TEST_CONFIG_DIR.lock().unwrap_or_else(|e| e.into_inner()) = dir;
}


impl Config {
    pub fn dir() -> Result<PathBuf> {
        #[cfg(test)]
        if let Some(p) = TEST_CONFIG_DIR.lock().unwrap_or_else(|e| e.into_inner()).clone() {
            return Ok(p);
        }
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


    /// Reject usernames that would be unsafe or ambiguous in token file paths.
    pub fn sanitize_username(user: &str) -> Result<()> {
        if user.is_empty() {
            return Err(GiteeError::Usage("username must not be empty".into()));
        }
        if user.contains('/') || user.contains('\\') || user.contains('\0') {
            return Err(GiteeError::Usage(format!("invalid username: {user}")));
        }
        if user == ".." || user.contains("..") {
            return Err(GiteeError::Usage(format!("invalid username: {user}")));
        }
        Ok(())
    }

    /// Username emitted to git for credential fill; never expose synthetic `default`.
    pub fn credential_display_username(active: Option<String>) -> String {
        active
            .filter(|s| !s.is_empty() && s != "default")
            .unwrap_or_else(|| "oauth2".into())
    }

    fn user_token_path(host: &str, user: &str) -> Result<PathBuf> {
        Self::sanitize_username(user)?;
        Ok(Self::dir()?.join(format!("{host}.{user}.token")))
    }

    fn keyring_account(host: &str, user: Option<&str>) -> String {
        match user {
            Some(u) => format!("{host}:{u}"),
            None => host.to_string(),
        }
    }

    pub fn active_user(host: &str) -> Result<Option<String>> {
        Ok(Self::load_settings()?
            .active_users
            .get(host)
            .cloned())
    }

    pub fn set_active_user(host: &str, user: &str) -> Result<()> {
        Self::sanitize_username(user)?;
        let mut s = Self::load_settings()?;
        s.active_users.insert(host.to_string(), user.to_string());
        let users = s.known_users.entry(host.to_string()).or_default();
        if !users.iter().any(|u| u == user) {
            users.push(user.to_string());
        }
        Self::save_settings(&s)
    }

    pub fn remember_user(host: &str, user: &str) -> Result<()> {
        Self::sanitize_username(user)?;
        let mut s = Self::load_settings()?;
        let users = s.known_users.entry(host.to_string()).or_default();
        if !users.iter().any(|u| u == user) {
            users.push(user.to_string());
        }
        if !s.active_users.contains_key(host) {
            s.active_users.insert(host.to_string(), user.to_string());
        }
        Self::save_settings(&s)
    }

    pub fn known_users(host: &str) -> Result<Vec<String>> {
        let s = Self::load_settings()?;
        Ok(s.known_users.get(host).cloned().unwrap_or_default())
    }

    pub fn switch_user(host: &str, user: &str) -> Result<()> {
        Self::sanitize_username(user)?;
        Self::migrate_legacy_user(host)?;
        let _ = Self::token_for_user(host, user)?;
        Self::set_active_user(host, user)
    }

    pub fn token_for_user(host: &str, user: &str) -> Result<String> {
        if let Ok(t) = keyring_get(&Self::keyring_account(host, Some(user))) {
            return Ok(t);
        }
        match fs::read_to_string(Self::user_token_path(host, user)?) {
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

    fn store_user_token(host: &str, user: &str, token: &str) -> Result<()> {
        let account = Self::keyring_account(host, Some(user));
        let keyring_ok = keyring::Entry::new(KEYRING_SERVICE, &account)
            .and_then(|e| e.set_password(token))
            .is_ok()
            && keyring_get(&account).ok().as_deref() == Some(token);
        if keyring_ok {
            let _ = fs::remove_file(Self::user_token_path(host, user)?);
            return Ok(());
        }
        let p = Self::user_token_path(host, user)?;
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).map_err(|e| GiteeError::Config(e.to_string()))?;
        }
        fs::write(&p, token).map_err(|e| GiteeError::Config(e.to_string()))?;
        restrict_perms(&p)?;
        Ok(())
    }

    pub fn set_token_for_user(host: &str, user: &str, token: &str) -> Result<()> {
        Self::sanitize_username(user)?;
        Self::store_user_token(host, user, token)?;
        Self::remember_user(host, user)?;
        Self::set_active_user(host, user)?;
        let _ = fs::remove_file(Self::token_path(host)?);
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, host) {
            let _ = entry.delete_credential();
        }
        Ok(())
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
        Self::migrate_legacy_user(host)?;
        if let Some(user) = Self::active_user(host)? {
            if let Ok(t) = Self::token_for_user(host, &user) {
                return Ok(t);
            }
        }
        // Legacy single-token (pre–ticket 17) host-only store.
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
        if let Ok(Some(user)) = Self::active_user(host) {
            if keyring_get(&Self::keyring_account(host, Some(&user))).is_ok() {
                return Some(TokenSource::Keyring);
            }
            if let Ok(p) = Self::user_token_path(host, &user) {
                if matches!(fs::read_to_string(&p), Ok(s) if !s.trim().is_empty()) {
                    return Some(TokenSource::File);
                }
            }
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

    fn keyring_delete_error(account: &str, err: keyring::Error) -> Option<String> {
        let msg = err.to_string();
        if msg.contains("No matching entry") || msg.to_ascii_lowercase().contains("not found") {
            None
        } else {
            Some(format!("keyring ({account}): {err}"))
        }
    }

    fn legacy_token(host: &str) -> Option<String> {
        keyring_get(host).ok().or_else(|| {
            fs::read_to_string(Self::token_path(host).ok()?)
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
    }

    pub fn has_legacy_token(host: &str) -> Result<bool> {
        Ok(Self::legacy_token(host).is_some())
    }

    fn clear_legacy_token(host: &str) -> Result<()> {
        let mut errors = Vec::new();
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, host) {
            if let Err(e) = entry.delete_credential() {
                if let Some(msg) = Self::keyring_delete_error(host, e) {
                    errors.push(msg);
                }
            }
        }
        match fs::remove_file(Self::token_path(host)?) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!("legacy token file: {e}")),
        }
        if !errors.is_empty() {
            return Err(GiteeError::Config(errors.join("; ")));
        }
        Ok(())
    }

    pub fn clear_token_for_user(host: &str, user: &str) -> Result<()> {
        Self::sanitize_username(user)?;
        let mut errors = Vec::new();
        let account = Self::keyring_account(host, Some(user));
        if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, &account) {
            if let Err(e) = entry.delete_credential() {
                if let Some(msg) = Self::keyring_delete_error(&account, e) {
                    errors.push(msg);
                }
            }
        }
        match fs::remove_file(Self::user_token_path(host, user)?) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(format!("token file ({user}): {e}")),
        }
        let mut settings = Self::load_settings()?;
        if let Some(users) = settings.known_users.get_mut(host) {
            users.retain(|u| u != user);
            if users.is_empty() {
                settings.known_users.remove(host);
            }
        }
        if settings.active_users.get(host) == Some(&user.to_string()) {
            settings.active_users.remove(host);
        }
        Self::save_settings(&settings)?;
        if !errors.is_empty() {
            return Err(GiteeError::Config(errors.join("; ")));
        }
        Ok(())
    }

    pub fn clear_stored_credentials(host: &str, username: Option<&str>) -> Result<()> {
        match username {
            Some(u) => Self::clear_token_for_user(host, u),
            None => {
                if let Some(active) = Self::active_user(host)? {
                    return Self::clear_token_for_user(host, &active);
                }
                Self::clear_legacy_token(host)
            }
        }
    }

    /// Forget tokens for a host (legacy + all known per-user stores).
    pub fn logout(host: &str) -> Result<()> {
        let mut errors = Vec::new();
        let users = Self::known_users(host).unwrap_or_default();
        for user in &users {
            if let Err(e) = Self::clear_token_for_user(host, user) {
                errors.push(e.to_string());
            }
        }
        if let Err(e) = Self::clear_legacy_token(host) {
            errors.push(e.to_string());
        }
        let mut settings = Self::load_settings()?;
        settings.active_users.remove(host);
        settings.known_users.remove(host);
        if let Err(e) = Self::save_settings(&settings) {
            errors.push(e.to_string());
        }
        if !errors.is_empty() {
            return Err(GiteeError::Config(errors.join("; ")));
        }
        Ok(())
    }

    /// Migrate legacy host-only tokens and the old synthetic `default` user slot
    /// into the per-user store under `oauth2`.
    pub fn migrate_legacy_user(host: &str) -> Result<()> {
        if Self::active_user(host)?.as_deref() == Some("default") {
            if let Ok(token) = Self::token_for_user(host, "default") {
                let _ = Self::clear_token_for_user(host, "default");
                return Self::set_token_for_user(host, "oauth2", &token);
            }
        }
        if Self::active_user(host)?.is_some() {
            return Ok(());
        }
        let Some(token) = Self::legacy_token(host) else {
            return Ok(());
        };
        Self::set_token_for_user(host, "oauth2", &token)
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
pub fn test_config_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
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
    fn sanitize_username_rejects_unsafe_values() {
        assert!(Config::sanitize_username("alice").is_ok());
        assert!(Config::sanitize_username("oauth2").is_ok());
        assert!(Config::sanitize_username("").is_err());
        assert!(Config::sanitize_username("../x").is_err());
        assert!(Config::sanitize_username("a/b").is_err());
        assert!(Config::sanitize_username("..").is_err());
    }

    #[test]
    fn credential_display_username_never_emits_default() {
        assert_eq!(Config::credential_display_username(None), "oauth2");
        assert_eq!(Config::credential_display_username(Some("default".into())), "oauth2");
        assert_eq!(Config::credential_display_username(Some("alice".into())), "alice");
    }

    #[test]
    fn migrate_legacy_user_uses_oauth2_not_default() {
        let _env = test_config_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "gitee-cli-migrate-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        set_test_dir(Some(dir.clone()));
        let host = "gitee.com";
        fs::write(dir.join(format!("{host}.token")), "legacy-token\n").unwrap();
        Config::migrate_legacy_user(host).unwrap();
        assert_eq!(Config::active_user(host).unwrap().as_deref(), Some("oauth2"));
        assert!(!dir.join(format!("{host}.token")).exists());
        assert_eq!(
            Config::token_for_user(host, "oauth2").unwrap(),
            "legacy-token"
        );
        set_test_dir(None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_stored_credentials_removes_user_token_file() {
        let _env = test_config_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "gitee-cli-clear-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        set_test_dir(Some(dir.clone()));
        let host = "gitee.com";
        Config::set_token_for_user(host, "alice", "secret").unwrap();
        assert!(dir.join(format!("{host}.alice.token")).exists());
        Config::clear_stored_credentials(host, Some("alice")).unwrap();
        assert!(!dir.join(format!("{host}.alice.token")).exists());
        set_test_dir(None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_round_trip_in_temp_dir() {
        let _env = test_config_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "gitee-cli-config-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        set_test_dir(Some(dir.clone()));
        Config::set_key("host", "gitee.com").unwrap();
        Config::set_key("editor", "vim").unwrap();
        Config::alias_set("co", "pr checkout").unwrap();
        assert_eq!(Config::get_key("host").unwrap().as_deref(), Some("gitee.com"));
        assert_eq!(Config::get_key("editor").unwrap().as_deref(), Some("vim"));
        let aliases = Config::alias_list().unwrap();
        assert_eq!(aliases, vec![("co".into(), "pr checkout".into())]);
        Config::alias_delete("co").unwrap();
        assert!(Config::alias_list().unwrap().is_empty());
        set_test_dir(None);
        let _ = fs::remove_dir_all(&dir);
    }
}
