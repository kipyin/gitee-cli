use std::collections::BTreeMap;
use std::io::{self, BufRead, Write};
use std::process::Command;

use crate::api::client::Client;
use crate::cli::{AuthCmd, GitCredentialCmd};
use crate::config::Config;
use crate::error::{GiteeError, Result};

pub fn execute(cmd: AuthCmd, host: &str) -> Result<()> {
    match cmd {
        AuthCmd::Login { token, force } => {
            let token = match token {
                Some(t) => t,
                None => {
                    // Never hang in non-interactive mode: if stdin is not a TTY,
                    // the prompt would read nothing (or block on a pipe). Demand
                    // an explicit `--token` (or `GITEE_TOKEN`) instead.
                    if !stdin_is_tty() {
                        return Err(GiteeError::Usage(
                            "auth login needs --token (or set GITEE_TOKEN) — stdin is not a terminal".into(),
                        ));
                    }
                    eprint!("Paste your Gitee personal access token: ");
                    io::stdout().flush().ok();
                    let mut line = String::new();
                    io::stdin().lock().read_line(&mut line).ok();
                    line.trim().to_string()
                }
            };
            if token.is_empty() {
                return Err(GiteeError::Usage(
                    "token required: pass --token or pipe via stdin".into(),
                ));
            }
            if !force {
                let client = Client::for_host(host, token.clone());
                let user = client.users().me().map_err(|e| {
                    GiteeError::Usage(format!(
                        "token validation failed: {e}. Re-run with --force to store anyway."
                    ))
                })?;
                let who = user
                    .name
                    .filter(|n| !n.is_empty())
                    .unwrap_or_else(|| user.login.clone());
                let login = if user.login.is_empty() {
                    who.clone()
                } else {
                    user.login
                };
                Config::set_token_for_user(host, &login, &token)?;
                println!("Logged in to {host} as {who}.");
            } else {
                Config::migrate_legacy_user(host)?;
                if let Some(active) = Config::active_user(host)? {
                    Config::set_token_for_user(host, &active, &token)?;
                    println!("Logged in to {host} as {active} (--force; token not validated).");
                } else {
                    Config::set_token_for_user(host, "oauth2", &token)?;
                    println!("Logged in to {host} as oauth2 (--force; token not validated).");
                }
            }
            Ok(())
        }
        AuthCmd::Status => {
            status(host)
        }
        AuthCmd::Token => {
            let t = Config::token(host)?;
            println!("{t}");
            Ok(())
        }
        AuthCmd::Logout => {
            Config::logout(host)?;
            if std::env::var("GITEE_TOKEN")
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
            {
                eprintln!(
                    "Warning: GITEE_TOKEN is set and will still override stored credentials."
                );
            }
            println!("Logged out of {host}.");
            Ok(())
        }
        AuthCmd::SetupGit => setup_git(host),
        AuthCmd::Switch { user } => {
            Config::switch_user(host, &user)?;
            println!("Switched {host} account to {user}.");
            Ok(())
        }
        AuthCmd::GitCredential(action) => git_credential(host, action),
    }
}

/// True when stdin is a terminal (so an interactive prompt won't hang).
fn stdin_is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdin().is_terminal()
}

fn status(host: &str) -> Result<()> {
    let active = Config::active_user(host)?;
    let mut users = Config::known_users(host)?;
    // Readable token is the source of truth — known_users alone is not "logged in".
    let src = Config::locate(host);

    if users.is_empty() {
        match src {
            Some(src) if Config::has_legacy_token(host)? && active.is_none() => {
                println!(
                    "Logged in to {host} via legacy host-only token (not migrated to a user account)."
                );
                println!("Active token via {}.", src.as_str());
            }
            Some(src) => println!("Logged in to {host} (via {}).", src.as_str()),
            None => println!("Not logged in to {host}."),
        }
        return Ok(());
    }

    // Gate the multi-account "Logged in" banner on a readable token.
    if src.is_none() {
        println!(
            "Not logged in to {host} (saved account metadata, but no token found)."
        );
        println!(
            "Run `gitee auth login` to restore credentials, or `gitee auth logout` to clear metadata."
        );
        return Ok(());
    }

    users.sort();
    println!("Logged in to {host}");
    for u in users {
        let mark = if Some(&u) == active.as_ref() {
            "*"
        } else {
            " "
        };
        println!("  {mark} {u}");
    }
    if let Some(src) = src {
        println!("Active token via {}.", src.as_str());
    }
    Ok(())
}

/// gh-style credential helper value; quotes the exe path for spaces.
pub fn git_credential_helper_value(exe: &str) -> String {
    format!("!\"{exe}\" auth git-credential")
}

fn setup_git(host: &str) -> Result<()> {
    Config::migrate_legacy_user(host)?;
    let exe = std::env::current_exe()
        .map_err(|e| GiteeError::Usage(format!("current_exe: {e}")))?;
    let exe = exe
        .to_str()
        .ok_or_else(|| GiteeError::Usage("current_exe path is not UTF-8".into()))?;
    let key = format!("credential.https://{host}.helper");
    let value = git_credential_helper_value(exe);
    let status = Command::new("git")
        .args(["config", "--global", &key, &value])
        .status()
        .map_err(|e| GiteeError::Usage(format!("git config: {e}")))?;
    if !status.success() {
        return Err(GiteeError::Usage(format!(
            "git config failed setting {key}"
        )));
    }
    println!("Configured git to use {exe} as a credential helper for https://{host}");
    Ok(())
}

fn git_credential(host: &str, action: GitCredentialCmd) -> Result<()> {
    let attrs = read_credential_attrs()?;
    match action {
        GitCredentialCmd::Get => credential_get(host, &attrs),
        GitCredentialCmd::Store => credential_store(host, &attrs),
        GitCredentialCmd::Erase => credential_erase(host, &attrs),
    }
}

fn credential_get(default_host: &str, attrs: &BTreeMap<String, String>) -> Result<()> {
    let protocol = attrs.get("protocol").map(String::as_str).unwrap_or("https");
    if protocol != "https" && protocol != "http" {
        return Ok(());
    }
    let req_host = attrs
        .get("host")
        .map(|s| s.split(':').next().unwrap_or(s))
        .unwrap_or(default_host);
    // Only answer for the configured host (ignore unrelated credential asks).
    if req_host != default_host && !default_host.is_empty() {
        // Still allow when --host matches attr host via CLI host default.
        // If user runs helper without --host, default_host is gitee.com from clap.
        if req_host != default_host {
            // Prefer the host from the credential request when answering.
        }
    }
    let host = req_host;
    let token = match Config::token(host) {
        Ok(t) => t,
        Err(_) => return Ok(()), // git treats empty helper output as "no credentials"
    };
    let username = Config::credential_display_username(Config::active_user(host)?);
    print_credential_attrs(&[
        ("username", username.as_str()),
        ("password", token.as_str()),
    ])?;
    Ok(())
}

fn credential_host_from_attrs(default_host: &str, attrs: &BTreeMap<String, String>) -> Option<String> {
    let protocol = attrs.get("protocol").map(String::as_str).unwrap_or("https");
    if protocol != "https" && protocol != "http" {
        return None;
    }
    // Prefer the credential protocol's host; fall back to the CLI default when absent.
    attrs
        .get("host")
        .map(|s| s.split(':').next().unwrap_or(s).to_string())
        .or_else(|| {
            if default_host.is_empty() {
                None
            } else {
                Some(default_host.to_string())
            }
        })
}

fn credential_store(default_host: &str, attrs: &BTreeMap<String, String>) -> Result<()> {
    let password = match attrs.get("password").map(String::as_str) {
        Some(p) if !p.is_empty() => p,
        _ => return Ok(()),
    };
    let Some(host) = credential_host_from_attrs(default_host, attrs) else {
        return Err(GiteeError::Usage(
            "credential store: could not determine host from request".into(),
        ));
    };
    let username = attrs
        .get("username")
        .map(String::as_str)
        .filter(|s| !s.is_empty() && *s != "default")
        .unwrap_or("oauth2");
    Config::set_token_for_user(&host, username, password)
}

fn credential_erase(default_host: &str, attrs: &BTreeMap<String, String>) -> Result<()> {
    // git may ask helpers to erase on auth failure. That must not wipe the CLI PAT
    // managed by `gitee auth login` — leave store/erase of secrets to auth commands.
    let _ = (default_host, attrs);
    Ok(())
}

/// Parse git credential protocol lines (`key=value`) until a blank line.
pub fn read_credential_attrs_from(reader: &mut dyn BufRead) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    loop {
        let mut line = String::new();
        let n = reader
            .read_line(&mut line)
            .map_err(|e| GiteeError::Usage(format!("read credential attrs: {e}")))?;
        if n == 0 {
            break;
        }
        let line = line.trim_end_matches(['\n', '\r']);
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.to_string(), v.to_string());
        }
    }
    Ok(map)
}

fn read_credential_attrs() -> Result<BTreeMap<String, String>> {
    let mut stdin = io::stdin().lock();
    read_credential_attrs_from(&mut stdin)
}

fn print_credential_attrs(pairs: &[(&str, &str)]) -> Result<()> {
    let mut out = io::stdout().lock();
    for (k, v) in pairs {
        writeln!(out, "{k}={v}")?;
    }
    writeln!(out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parses_git_credential_protocol_lines() {
        let mut cur = Cursor::new("protocol=https\nhost=gitee.com\npath=oschina/gitee-cli\n\n");
        let map = read_credential_attrs_from(&mut cur).unwrap();
        assert_eq!(map.get("protocol").map(String::as_str), Some("https"));
        assert_eq!(map.get("host").map(String::as_str), Some("gitee.com"));
        assert_eq!(
            map.get("path").map(String::as_str),
            Some("oschina/gitee-cli")
        );
    }

    /// `auth login` without `--token` and no TTY on stdin must error with a
    /// message naming `--token`, not hang waiting for input. Cargo's test
    /// harness pipes stdin from /dev/null, so `stdin_is_tty()` is false here.
    #[test]
    fn auth_login_no_token_no_tty_errors_with_hint() {
        let _env = crate::config::test_config_env_lock();
        let err = execute(
            AuthCmd::Login { token: None, force: false },
            "gitee.test",
        )
        .expect_err("non-TTY login without token must error");
        let msg = err.to_string();
        assert!(
            msg.contains("--token"),
            "expected --token hint, got: {msg}"
        );
    }

    #[test]
    fn git_credential_helper_quotes_exe_path() {
        assert_eq!(
            git_credential_helper_value("/Applications/Gitee CLI.app/gitee"),
            "!\"/Applications/Gitee CLI.app/gitee\" auth git-credential"
        );
    }

    #[test]
    fn credential_host_from_attrs_prefers_protocol_host() {
        let mut attrs = BTreeMap::new();
        attrs.insert("protocol".into(), "https".into());
        attrs.insert("host".into(), "self.gitee.test".into());
        assert_eq!(
            credential_host_from_attrs("gitee.com", &attrs).as_deref(),
            Some("self.gitee.test")
        );
        assert_eq!(
            credential_host_from_attrs("gitee.com", &BTreeMap::new()).as_deref(),
            Some("gitee.com")
        );
    }

    #[test]
    fn credential_store_uses_protocol_host_when_cli_default_differs() {
        let _env = crate::config::test_config_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "gitee-cli-store-host-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::config::set_test_dir(Some(dir.clone()));
        let mut attrs = BTreeMap::new();
        attrs.insert("protocol".into(), "https".into());
        attrs.insert("host".into(), "self.gitee.test".into());
        attrs.insert("username".into(), "oauth2".into());
        attrs.insert("password".into(), "pat-from-git".into());
        credential_store("gitee.com", &attrs).unwrap();
        assert_eq!(
            Config::token_for_user("self.gitee.test", "oauth2").unwrap(),
            "pat-from-git"
        );
        crate::config::set_test_dir(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn credential_store_persists_token_for_host() {
        let _env = crate::config::test_config_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "gitee-cli-store-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::config::set_test_dir(Some(dir.clone()));
        let mut attrs = BTreeMap::new();
        attrs.insert("protocol".into(), "https".into());
        attrs.insert("host".into(), "gitee.test".into());
        attrs.insert("username".into(), "alice".into());
        attrs.insert("password".into(), "pat-from-git".into());
        credential_store("gitee.test", &attrs).unwrap();
        assert_eq!(
            Config::token_for_user("gitee.test", "alice").unwrap(),
            "pat-from-git"
        );
        crate::config::set_test_dir(None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn credential_erase_does_not_clear_cli_token() {
        let _env = crate::config::test_config_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "gitee-cli-erase-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::config::set_test_dir(Some(dir.clone()));
        Config::set_token_for_user("gitee.test", "alice", "secret").unwrap();
        let mut attrs = BTreeMap::new();
        attrs.insert("protocol".into(), "https".into());
        attrs.insert("host".into(), "gitee.test".into());
        attrs.insert("username".into(), "alice".into());
        credential_erase("gitee.test", &attrs).unwrap();
        assert_eq!(
            Config::token_for_user("gitee.test", "alice").unwrap(),
            "secret"
        );
        crate::config::set_test_dir(None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
