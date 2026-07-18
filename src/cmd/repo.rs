use std::io::Write;

use super::Ctx;
use crate::cli::RepoCmd;
use crate::error::{GiteeError, Result};
use crate::out;
use crate::repo::Repo;

pub fn execute(ctx: &Ctx, cmd: RepoCmd) -> Result<()> {
    match cmd {
        RepoCmd::View { target } => {
            let rr = match target {
                Some(s) => Repo::from_spec(&s)?,
                None => ctx.repo()?.clone(),
            };
            let details = ctx.client.repos().get(&rr.owner, &rr.name)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &details, |w| out::one_repo(w, &details))?;
        }
        RepoCmd::List { owner, limit } => {
            let items = match &owner {
                Some(o) => ctx.client.repos().list_user(o, limit.limit)?,
                None => ctx.client.repos().list_mine(limit.limit)?,
            };
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::repo_table(w, &items))?;
        }
        RepoCmd::Clone { spec, dir, ssh } => {
            let rr = Repo::from_spec(&spec)?;
            let det = ctx.client.repos().get(&rr.owner, &rr.name)?;
            let url = det.preferred_url(ssh);
            let mut args: Vec<String> = vec!["clone".to_string(), url.clone()];
            if let Some(d) = dir {
                args.push(d);
            }
            let status = std::process::Command::new("git")
                .args(&args)
                .status()
                .map_err(|e| GiteeError::Usage(format!("git clone: {e}")))?;
            if !status.success() {
                return Err(GiteeError::Usage(format!("git clone failed for {url}")));
            }
        }
        RepoCmd::Fork { add_remote } => {
            let rr = ctx.repo()?.clone();
            let forked = ctx.client.repos().fork(&rr.owner, &rr.name)?;
            let mut out = std::io::stdout().lock();
            writeln!(out, "Forked to {}", forked.full_name)?;
            ctx.out
                .render(&mut out, &forked, |w| out::one_repo(w, &forked))?;
            if let Some(name) = add_remote {
                let url = forked.preferred_url(true);
                let status = std::process::Command::new("git")
                    .args(["remote", "add", &name, &url])
                    .status()
                    .map_err(|e| GiteeError::Usage(format!("git remote add: {e}")))?;
                if status.success() {
                    writeln!(out, "Added remote '{name}' -> {url}")?;
                }
            }
        }
    }
    Ok(())
}
