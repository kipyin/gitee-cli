use std::io::Write;

use super::Ctx;
use crate::api::pulls::{CreatePr, PrFilter};
use crate::cli::PrCmd;
use crate::error::{GiteeError, Result};
use crate::models::{MergeMethod, PrState};
use crate::out;

pub fn execute(ctx: &Ctx, cmd: PrCmd) -> Result<()> {
    match cmd {
        PrCmd::List { list, author } => {
            let repo = ctx.repo()?;
            let filter = PrFilter {
                state: list.state.as_deref(),
                author: author.as_deref(),
                limit: list.limit,
            };
            let items = ctx.client.pulls(repo).list(&filter)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::pr_table(w, &items))?;
        }
        PrCmd::View { number } => {
            let repo = ctx.repo()?;
            let pr = ctx.client.pulls(repo).get(number)?;
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &pr, |w| out::one_pr(w, &pr))?;
        }
        PrCmd::Diff { number } => {
            let repo = ctx.repo()?;
            let files = ctx.client.pulls(repo).files(number)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &files, |w| out::pr_diff(w, &files))?;
        }
        PrCmd::Checkout { number } => checkout_pr(ctx, number)?,
        PrCmd::Create {
            title,
            body,
            head,
            base,
        } => {
            let repo = ctx.repo()?;
            let head = match head {
                Some(h) => h,
                None => current_branch()?,
            };
            let base = match base {
                Some(b) => b,
                None => {
                    let o = repo.owner.as_str();
                    let r = repo.name.as_str();
                    ctx.client
                        .repos()
                        .get(o, r)?
                        .default_branch
                        .unwrap_or_else(|| "master".to_string())
                }
            };
            let req = CreatePr {
                title: &title,
                head: &head,
                base: &base,
                body: body.as_deref(),
            };
            let pr = ctx.client.pulls(repo).create(&req)?;
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &pr, |w| out::one_pr(w, &pr))?;
        }
        PrCmd::Merge {
            number,
            squash,
            rebase,
            no_close_issue,
        } => {
            let repo = ctx.repo()?;
            let method = if rebase {
                MergeMethod::Rebase
            } else if squash {
                MergeMethod::Squash
            } else {
                MergeMethod::Merge
            };
            ctx.client
                .pulls(repo)
                .merge(number, method, !no_close_issue)?;
            let mut out = std::io::stdout().lock();
            writeln!(out, "Merged pull request !{number}")?;
        }
        PrCmd::Comment { number, body } => {
            let repo = ctx.repo()?;
            let c = ctx.client.pulls(repo).comment(number, &body.body)?;
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &c, |w| out::comment_line(w, &c))?;
        }
        PrCmd::Approve { number, force } => {
            let repo = ctx.repo()?;
            ctx.client.pulls(repo).approve(number, force)?;
            let mut out = std::io::stdout().lock();
            writeln!(out, "Approved pull request !{number}")?;
        }
        PrCmd::Close { number } => {
            let repo = ctx.repo()?;
            let pr = ctx.client.pulls(repo).set_state(number, PrState::Closed)?;
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &pr, |w| out::one_pr(w, &pr))?;
        }
        PrCmd::Reopen { number } => {
            let repo = ctx.repo()?;
            let pr = ctx.client.pulls(repo).set_state(number, PrState::Open)?;
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &pr, |w| out::one_pr(w, &pr))?;
        }
        PrCmd::Link { number, issue } => {
            let repo = ctx.repo()?;
            let tag = format!("#{issue}");
            let linked = ctx.client.pulls(repo).link(number, &tag)?;
            let mut out = std::io::stdout().lock();
            if linked {
                writeln!(out, "Linked issue {tag} on pull request !{number}")?;
            } else {
                writeln!(out, "Pull request !{number} already references {tag}")?;
            }
        }
    }
    Ok(())
}

fn checkout_pr(ctx: &Ctx, number: i64) -> Result<()> {
    let repo = ctx.repo()?;
    let pr = ctx.client.pulls(repo).get(number)?;
    let head_ref = pr.head.git_ref.trim();
    if head_ref.is_empty() {
        return Err(GiteeError::Usage(format!(
            "pull request !{number} has no head ref"
        )));
    }

    let o = repo.owner.as_str();
    let r = repo.name.as_str();
    let base = ctx.client.repos().get(o, r)?;
    let fetch_url = pr
        .head
        .repo
        .as_ref()
        .and_then(|hr| hr.fetch_url())
        .unwrap_or_else(|| base.preferred_url(true));
    let branch = format!("pr-{number}");
    let refspec = format!("+{head_ref}:{branch}");

    let fetch = std::process::Command::new("git")
        .args(["fetch", &fetch_url, &refspec])
        .output()
        .map_err(|e| GiteeError::Usage(format!("git fetch: {e}")))?;
    if !fetch.status.success() {
        let stderr = String::from_utf8_lossy(&fetch.stderr);
        return Err(GiteeError::Usage(format!(
            "git fetch failed: {}",
            stderr.trim()
        )));
    }

    let checkout = std::process::Command::new("git")
        .args(["checkout", &branch])
        .output()
        .map_err(|e| GiteeError::Usage(format!("git checkout: {e}")))?;
    if !checkout.status.success() {
        let stderr = String::from_utf8_lossy(&checkout.stderr);
        return Err(GiteeError::Usage(format!(
            "git checkout failed: {}",
            stderr.trim()
        )));
    }

    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "Checked out branch '{branch}' for pull request !{number}"
    )?;
    writeln!(
        out,
        "Hint: run `git log --oneline {}..{}` to see PR commits",
        pr.base.git_ref, branch
    )?;
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
