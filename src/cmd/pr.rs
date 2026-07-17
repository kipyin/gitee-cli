use super::Ctx;
use crate::cli::PrCmd;
use crate::error::{GiteeError, Result};
use crate::models::{Comment, PullRequest, RepoInfo};
use crate::out;
use serde_json::Value;

pub fn execute(ctx: &Ctx, cmd: PrCmd) -> Result<()> {
    let o = ctx.repo.owner.as_str();
    let r = ctx.repo.name.as_str();
    match cmd {
        PrCmd::List {
            state,
            author,
            limit,
        } => {
            let mut q: Vec<(&str, String)> = Vec::new();
            if let Some(s) = state {
                q.push(("state", s));
            }
            if let Some(a) = author {
                q.push(("author", a));
            }
            let qref: Vec<(&str, &str)> = q.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let path = format!("/repos/{o}/{r}/pulls");
            let items: Vec<PullRequest> = ctx.client.get_paged(&path, &qref, limit)?;
            if ctx.out.json {
                out::json(&items);
            } else {
                out::pr_table(&items);
            }
        }
        PrCmd::Create {
            title,
            body,
            head,
            base,
        } => {
            let head = match head {
                Some(h) => h,
                None => current_branch()?,
            };
            let base = match base {
                Some(b) => b,
                None => {
                    let info: RepoInfo = ctx.client.get(&format!("/repos/{o}/{r}"), &[])?;
                    info.default_branch.unwrap_or_else(|| "master".to_string())
                }
            };
            let mut f: Vec<(&str, String)> = vec![("title", title), ("head", head), ("base", base)];
            if let Some(b) = body {
                f.push(("body", b));
            }
            let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let pr: PullRequest = ctx.client.post(&format!("/repos/{o}/{r}/pulls"), &form)?;
            if ctx.out.json {
                out::json(&pr);
            } else {
                out::one_pr(&pr);
            }
        }
        PrCmd::Merge {
            number,
            squash,
            rebase,
            no_close_issue,
        } => {
            let method = if rebase {
                "rebase"
            } else if squash {
                "squash"
            } else {
                "merge"
            };
            let close = if no_close_issue { "false" } else { "true" };
            let f: Vec<(&str, String)> = vec![
                ("merge_method", method.to_string()),
                ("close_related_issue", close.to_string()),
            ];
            let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let _v: Value = ctx
                .client
                .put(&format!("/repos/{o}/{r}/pulls/{number}/merge"), &form)?;
            println!("Merged pull request !{number}");
        }
        PrCmd::Comment { number, body } => {
            let f: Vec<(&str, String)> = vec![("body", body)];
            let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let c: Comment = ctx
                .client
                .post(&format!("/repos/{o}/{r}/pulls/{number}/comments"), &form)?;
            if ctx.out.json {
                out::json(&c);
            } else {
                out::comment_line(&c);
            }
        }
        PrCmd::Approve { number, force } => {
            let mut f: Vec<(&str, String)> = Vec::new();
            if force {
                f.push(("force", "true".to_string()));
            }
            let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let _pr: PullRequest = ctx
                .client
                .post(&format!("/repos/{o}/{r}/pulls/{number}/review"), &form)?;
            println!("Approved pull request !{number}");
        }
        PrCmd::Close { number } => {
            let f: Vec<(&str, String)> = vec![("state", "closed".to_string())];
            let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let pr: PullRequest = ctx
                .client
                .patch(&format!("/repos/{o}/{r}/pulls/{number}"), &form)?;
            if ctx.out.json {
                out::json(&pr);
            } else {
                out::one_pr(&pr);
            }
        }
        PrCmd::Link { number, issue } => {
            let pr: PullRequest = ctx
                .client
                .get(&format!("/repos/{o}/{r}/pulls/{number}"), &[])?;
            let cur = pr.body.clone().unwrap_or_default();
            let tag = format!("#{issue}");
            if cur.contains(tag.as_str()) {
                println!("Pull request !{number} already references {tag}");
            } else {
                let new = format!("{cur}\n\nLinked: {tag}");
                let f: Vec<(&str, String)> = vec![("body", new)];
                let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
                let _pr: PullRequest = ctx
                    .client
                    .patch(&format!("/repos/{o}/{r}/pulls/{number}"), &form)?;
                println!("Linked issue {tag} on pull request !{number}");
            }
        }
    }
    Ok(())
}

fn current_branch() -> Result<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| GiteeError::Usage(format!("git: {e}")))?;
    if !out.status.success() {
        return Err(GiteeError::Usage(
            "could not determine current branch (pass --head)".into(),
        ));
    }
    let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if b.is_empty() || b == "HEAD" {
        return Err(GiteeError::Usage(
            "detached HEAD; pass --head <branch>".into(),
        ));
    }
    Ok(b)
}
