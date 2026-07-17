use crate::api::client::Client;
use crate::cli::RepoCmd;
use crate::error::{GiteeError, Result};
use crate::models::RepoDetails;
use crate::out::{self, Output};
use crate::repo::Repo;

/// Repository commands. Unlike pr/issue, these resolve their own target so that
/// `list`, `clone`, and `view <owner/name>` work without a local git remote.
pub fn execute(
    client: &Client,
    out: &Output,
    cmd: RepoCmd,
    repo_arg: Option<String>,
    remote_arg: Option<String>,
) -> Result<()> {
    match cmd {
        RepoCmd::View { target } => {
            let rr = match target {
                Some(s) => Repo::from_spec(&s)?,
                None => resolve(&repo_arg, &remote_arg)?,
            };
            let owner = &rr.owner;
            let name = &rr.name;
            let details: RepoDetails =
                client.get(&format!("/repos/{owner}/{name}"), &[])?;
            out.render(&details, || out::one_repo(&details));
        }
        RepoCmd::List { owner, limit } => {
            // Bare lists the authenticated user's repos; with an arg, that
            // user's public repos. Gitee differs: /user/repos vs /users/{owner}/repos.
            let path = match &owner {
                Some(o) => format!("/users/{o}/repos"),
                None => "/user/repos".to_string(),
            };
            let items: Vec<RepoDetails> = client.get_paged(&path, &[], limit)?;
            out.render(&items, || out::repo_table(&items));
        }
        RepoCmd::Clone { spec, dir, ssh } => {
            let rr = Repo::from_spec(&spec)?;
            let owner = &rr.owner;
            let name = &rr.name;
            let det: RepoDetails = client.get(&format!("/repos/{owner}/{name}"), &[])?;
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
        RepoCmd::Fork { add_remote } => {
            let rr = resolve(&repo_arg, &remote_arg)?;
            let o = rr.owner.as_str();
            let r = rr.name.as_str();
            let forked: RepoDetails =
                client.post(&format!("/repos/{o}/{r}/forks"), &[])?;
            println!("Forked to {}", forked.full_name);
            out.render(&forked, || out::one_repo(&forked));
            if let Some(name) = add_remote {
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

/// Resolve a repo from the global --repo/--remote (for View-without-arg and Fork).
fn resolve(repo_arg: &Option<String>, remote_arg: &Option<String>) -> Result<Repo> {
    Repo::resolve(repo_arg.as_deref(), remote_arg.as_deref())
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
