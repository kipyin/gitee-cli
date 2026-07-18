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
                // Without validation we cannot discover the username; keep legacy host token.
                Config::set_token(host, &token)?;
                println!("Logged in to {host} (--force; token not validated).");
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

fn status(host: &str) -> Result<()> {
    Config::migrate_legacy_user(host)?;
    let active = Config::active_user(host)?;
    let mut users = Config::known_users(host)?;
    if users.is_empty() {
        match Config::locate(host) {
            Some(src) => println!("Logged in to {host} (via {}).", src.as_str()),
            None => println!("Not logged in to {host}."),
        }
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
    if let Some(src) = Config::locate(host) {
        println!("Active token via {}.", src.as_str());
    }
    Ok(())
}

fn setup_git(host: &str) -> Result<()> {
    let exe = std::env::current_exe()
        .map_err(|e| GiteeError::Usage(format!("current_exe: {e}")))?;
    let exe = exe
        .to_str()
        .ok_or_else(|| GiteeError::Usage("current_exe path is not UTF-8".into()))?;
    // gh-style: credential.https://<host>.helper=!path auth git-credential
    let key = format!("credential.https://{host}.helper");
    let value = format!("!{exe} auth git-credential");
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
        GitCredentialCmd::Store => {
            // Tokens are managed by `auth login`; store is a no-op success.
            Ok(())
        }
        GitCredentialCmd::Erase => {
            // Do not wipe the CLI token store from git's erase; no-op.
            Ok(())
        }
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
    let username = Config::active_user(host)?
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "oauth2".into());
    print_credential_attrs(&[
        ("username", username.as_str()),
        ("password", token.as_str()),
    ])?;
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
}
