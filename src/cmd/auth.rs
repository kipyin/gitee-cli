use std::io::{self, BufRead, Write};

use crate::api::client::Client;
use crate::cli::AuthCmd;
use crate::config::Config;
use crate::error::{GiteeError, Result};
use crate::models::UserBasic;

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
            // Validate before storing so a typo doesn't land silently and explode
            // on the next command. --force skips this (offline, restricted token).
            if !force {
                let client = Client::for_host(host, token.clone());
                let who = client
                    .get::<UserBasic>("/user", &[])
                    .map(|u| u.name.filter(|n| !n.is_empty()).unwrap_or(u.login))
                    .map_err(|e| {
                        GiteeError::Usage(format!(
                            "token validation failed: {e}. Re-run with --force to store anyway."
                        ))
                    })?;
                Config::set_token(host, &token)?;
                println!("Logged in to {host} as {who}.");
            } else {
                Config::set_token(host, &token)?;
                println!("Logged in to {host} (--force; token not validated).");
            }
            Ok(())
        }
        AuthCmd::Status => {
            match Config::locate(host) {
                Some(src) => println!("Logged in to {host} (via {}).", src.as_str()),
                None => println!("Not logged in to {host}."),
            }
            Ok(())
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
    }
}
