use std::io::Write;

use super::Ctx;
use crate::api::repos::{CreateRepo, EditRepo};
use crate::cli::RepoCmd;
use crate::error::{GiteeError, Result};
use crate::out;
use crate::repo::Repo;

pub fn execute(ctx: &Ctx, cmd: RepoCmd) -> Result<()> {
    match cmd {
        RepoCmd::View { target, web } => {
            let rr = match target {
                Some(s) => Repo::from_spec(&s)?,
                None => ctx.repo()?.clone(),
            };
            if web {
                let url = crate::web::repo_url(&ctx.host, &rr);
                return crate::web::open_or_print(&url);
            }
            let mut details = ctx.client.repos().get(&rr.owner, &rr.name)?;
            // Cheap check endpoints for JSON/human extras (ticket 22).
            details.starred = Some(ctx.client.repos().is_starred(&rr.owner, &rr.name)?);
            details.watching = Some(ctx.client.repos().is_watching(&rr.owner, &rr.name)?);
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
        RepoCmd::Create {
            name,
            org,
            private,
            description,
            homepage,
            gitignore,
            license,
        } => {
            let req = CreateRepo {
                name: &name,
                org: org.as_deref(),
                description: description.as_deref(),
                homepage: homepage.as_deref(),
                gitignore_template: gitignore.as_deref(),
                license_template: license.as_deref(),
                private,
            };
            let details = ctx.client.repos().create(&req)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &details, |w| out::one_repo(w, &details))?;
        }
        RepoCmd::Edit {
            description,
            homepage,
            private,
            public,
            default_branch,
        } => {
            let rr = ctx.repo()?.clone();
            let private_val = if private {
                Some(true)
            } else if public {
                Some(false)
            } else {
                None
            };
            let req = EditRepo {
                name: &rr.name,
                description: description.as_deref(),
                homepage: homepage.as_deref(),
                private: private_val,
                default_branch: default_branch.as_deref(),
            };
            // PATCH returns the updated repository (verified live 2026-07-18).
            let details = ctx.client.repos().edit(&rr.owner, &rr.name, &req)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &details, |w| out::one_repo(w, &details))?;
        }
        RepoCmd::Rename { new_path } => {
            let rr = ctx.repo()?.clone();
            let details = ctx
                .client
                .repos()
                .rename(&rr.owner, &rr.name, &rr.name, &new_path)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &details, |w| out::one_repo(w, &details))?;
        }
        RepoCmd::Delete { yes } => {
            let rr = ctx.repo()?.clone();
            let full_name = format!("{}/{}", rr.owner, rr.name);
            super::confirm(&format!("Delete repository {full_name}"), yes)?;
            ctx.client.repos().delete(&rr.owner, &rr.name)?;
            let mut out = std::io::stdout().lock();
            writeln!(out, "Deleted repository {full_name}")?;
        }
        RepoCmd::Star => {
            let rr = ctx.repo()?.clone();
            ctx.client.repos().star(&rr.owner, &rr.name)?;
            let starred = ctx.client.repos().is_starred(&rr.owner, &rr.name)?;
            writeln!(
                std::io::stdout().lock(),
                "Starred {}/{} (starred={starred})",
                rr.owner, rr.name
            )?;
        }
        RepoCmd::Unstar => {
            let rr = ctx.repo()?.clone();
            ctx.client.repos().unstar(&rr.owner, &rr.name)?;
            let starred = ctx.client.repos().is_starred(&rr.owner, &rr.name)?;
            writeln!(
                std::io::stdout().lock(),
                "Unstarred {}/{} (starred={starred})",
                rr.owner, rr.name
            )?;
        }
        RepoCmd::Watch => {
            let rr = ctx.repo()?.clone();
            ctx.client.repos().watch(&rr.owner, &rr.name)?;
            let watching = ctx.client.repos().is_watching(&rr.owner, &rr.name)?;
            writeln!(
                std::io::stdout().lock(),
                "Watching {}/{} (watching={watching})",
                rr.owner, rr.name
            )?;
        }
        RepoCmd::Unwatch => {
            let rr = ctx.repo()?.clone();
            ctx.client.repos().unwatch(&rr.owner, &rr.name)?;
            let watching = ctx.client.repos().is_watching(&rr.owner, &rr.name)?;
            writeln!(
                std::io::stdout().lock(),
                "Unwatched {}/{} (watching={watching})",
                rr.owner, rr.name
            )?;
        }
    }
    Ok(())
}
