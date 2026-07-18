use std::io::Write;

use super::{join_flags, resolve_milestone_opt, Ctx};
use crate::api::issues::{CreateIssue, EditIssue, IssueFilter};
use crate::cli::IssueCmd;
use crate::error::Result;
use crate::models::IssueState;
use crate::out;

pub fn execute(ctx: &Ctx, cmd: IssueCmd) -> Result<()> {
    match cmd {
        IssueCmd::List { list, assignee } => {
            let repo = ctx.repo()?;
            let filter = IssueFilter {
                state: list.state.as_deref(),
                assignee: assignee.as_deref(),
                limit: list.limit,
            };
            let items = ctx.client.issues(repo).list(&filter)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::issue_table(w, &items))?;
        }
        IssueCmd::View { number } => {
            let repo = ctx.repo()?;
            let issue = ctx.client.issues(repo).get(&number)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &issue, |w| out::one_issue(w, &issue))?;
        }
        IssueCmd::Create {
            title,
            body,
            assignee,
            labels,
            milestone,
            security_hole,
        } => {
            let repo = ctx.repo()?;
            let milestone_number = resolve_milestone_opt(ctx, repo, milestone.as_deref())?;
            let req = CreateIssue {
                title: &title,
                body: body.as_deref(),
                assignee: assignee.as_deref(),
                labels: labels.as_deref(),
                milestone_number,
                security_hole,
            };
            let issue = ctx.client.issues(repo).create(&req)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &issue, |w| out::one_issue(w, &issue))?;
        }
        IssueCmd::Edit {
            number,
            title,
            body,
            assignee,
            label,
            milestone,
            security_hole,
        } => {
            let repo = ctx.repo()?;
            let milestone_number = resolve_milestone_opt(ctx, repo, milestone.as_deref())?;
            let labels = join_flags(&label);
            let req = EditIssue {
                title: title.as_deref(),
                body: body.as_deref(),
                assignee: assignee.as_deref(),
                labels: labels.as_deref(),
                milestone_number,
                security_hole: security_hole.then_some(true),
            };
            let issue = ctx.client.issues(repo).edit(&number, &req)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &issue, |w| out::one_issue(w, &issue))?;
        }
        IssueCmd::Close { number } => {
            let repo = ctx.repo()?;
            let issue = ctx
                .client
                .issues(repo)
                .set_state(&number, IssueState::Closed)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &issue, |w| out::one_issue(w, &issue))?;
        }
        IssueCmd::Reopen { number } => {
            let repo = ctx.repo()?;
            let issue = ctx
                .client
                .issues(repo)
                .set_state(&number, IssueState::Open)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &issue, |w| out::one_issue(w, &issue))?;
        }
        IssueCmd::Link { number, pr } => {
            let repo = ctx.repo()?;
            let tag = format!("!{pr}");
            let linked = ctx.client.issues(repo).link(&number, &tag)?;
            let mut out = std::io::stdout().lock();
            if linked {
                writeln!(out, "Linked pull request {tag} on issue #{number}")?;
            } else {
                writeln!(out, "Issue #{number} already references {tag}")?;
            }
        }
        IssueCmd::Comment { number, body } => {
            let repo = ctx.repo()?;
            let c = ctx.client.issues(repo).comment(&number, &body.body)?;
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &c, |w| out::comment_line(w, &c))?;
        }
    }
    Ok(())
}
