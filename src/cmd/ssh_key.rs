use std::io::Write;

use super::{confirm, Ctx};
use crate::cli::SshKeyCmd;
use crate::error::{GiteeError, Result};
use crate::out;

pub fn execute(ctx: &Ctx, cmd: SshKeyCmd) -> Result<()> {
    match cmd {
        SshKeyCmd::List { limit } => {
            let items = ctx.client.users().keys(limit.limit)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::ssh_key_table(w, &items))?;
        }
        SshKeyCmd::Add { pubkey_file, title } => {
            let contents = std::fs::read_to_string(&pubkey_file)
                .map_err(|e| GiteeError::Usage(format!("read {pubkey_file}: {e}")))?;
            let key = contents.trim();
            if key.is_empty() {
                return Err(GiteeError::Usage("public key file is empty".into()));
            }
            let title = title.unwrap_or_else(|| default_key_title(key));
            let created = ctx.client.users().add_key(key, &title)?;
            let items = [created];
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::ssh_key_table(w, &items))?;
        }
        SshKeyCmd::Delete { id, yes } => {
            confirm(&format!("Delete SSH key {id}"), yes)?;
            ctx.client.users().delete_key(id)?;
            writeln!(std::io::stdout().lock(), "Deleted SSH key {id}")?;
        }
    }
    Ok(())
}

fn default_key_title(key: &str) -> String {
    // ssh pubkey line: <type> <blob> [comment]
    let comment = key.split_whitespace().nth(2);
    if let Some(c) = comment {
        if !c.is_empty() {
            return c.to_string();
        }
    }
    let host = hostname();
    let date = today();
    format!("{host}-{date}")
}

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "host".into())
}

fn today() -> String {
    // Avoid extra chrono dep: local YYYY-MM-DD via date(1).
    std::process::Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown-date".into())
}

#[cfg(test)]
mod title_tests {
    use super::default_key_title;

    #[test]
    fn uses_pubkey_comment_when_present() {
        let title = default_key_title("ssh-ed25519 AAAAAcel comment@box");
        assert_eq!(title, "comment@box");
    }
}
