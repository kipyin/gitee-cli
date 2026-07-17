use std::io::{self, BufRead};

use crate::cli::AuthCmd;
use crate::config::Config;
use crate::error::{GiteeError, Result};

pub fn execute(cmd: AuthCmd) -> Result<()> {
    match cmd {
        AuthCmd::Login { token, host } => {
            let token = match token {
                Some(t) => t,
                None => {
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
            Config::set_token(&host, &token)?;
            println!("Logged in to {host}.");
            Ok(())
        }
    }
}
