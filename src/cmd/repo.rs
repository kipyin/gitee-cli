use super::Ctx;
use crate::cli::RepoCmd;
use crate::error::{GiteeError, Result};
use crate::models::RepoDetails;
use crate::out;
use crate::repo::Repo;

pub fn execute(ctx: &Ctx, cmd: RepoCmd) -> Result<()> {
    match cmd {
        RepoCmd::View { repo } => {
            let (o, r) = match repo {
                Some(s) => {
                    let rr = Repo::from_spec(&s)?;
                    (rr.owner, rr.name)
                }
                None => (ctx.repo.owner.clone(), ctx.repo.name.clone()),
            };
            let details: RepoDetails = ctx.client.get(&format!("/repos/{o}/{r}"), &[])?;
            ctx.out.render(&details, || out::one_repo(&details));
        }
        RepoCmd::List { owner, limit } => {
            // Bare lists the authenticated user's repos; with an arg, that
            // user's public repos. Gitee differs: /user/repos vs /users/{owner}/repos.
            let path = match &owner {
                Some(o) => format!("/users/{o}/repos"),
                None => "/user/repos".to_string(),
            };
            let items: Vec<RepoDetails> = ctx.client.get_paged(&path, &[], limit)?;
            ctx.out.render(&items, || out::repo_table(&items));
        }
        RepoCmd::Clone { spec, dir, ssh } => {
            let rr = Repo::from_spec(&spec)?;
            let owner = &rr.owner;
            let name = &rr.name;
            let det: RepoDetails =
                ctx.client.get(&format!("/repos/{owner}/{name}"), &[])?;
            let url = preferred_url(&det, ssh);
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
        RepoCmd::Fork { remote } => {
            let o = ctx.repo.owner.as_str();
            let r = ctx.repo.name.as_str();
            let forked: RepoDetails =
                ctx.client.post(&format!("/repos/{o}/{r}/forks"), &[])?;
            println!("Forked to {}", forked.full_name);
            ctx.out.render(&forked, || out::one_repo(&forked));
            if let Some(name) = remote {
                let url = preferred_url(&forked, true);
                let status = std::process::Command::new("git")
                    .args(["remote", "add", &name, &url])
                    .status()
                    .map_err(|e| GiteeError::Usage(format!("git remote add: {e}")))?;
                if status.success() {
                    println!("Added remote '{name}' -> {url}");
                }
            }
        }
    }
    Ok(())
}

fn preferred_url(det: &RepoDetails, ssh: bool) -> String {
    if ssh {
        if let Some(u) = &det.ssh_url {
            if !u.is_empty() {
                return u.clone();
            }
        }
    } else if let Some(u) = &det.clone_url {
        if !u.is_empty() {
            return u.clone();
        }
    }
    det.html_url.clone()
}
