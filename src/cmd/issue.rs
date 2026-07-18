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
                ..Default::default()
            };
            let items = ctx.client.issues(repo).list(&filter)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::issue_table(w, &items))?;
        }
        IssueCmd::Status { limit } => {
            let repo = ctx.repo()?;
            let me = ctx.me()?;
            let login = me.login.as_str();
            let open = Some("open");
            let created = ctx.client.issues(repo).list(&IssueFilter {
                state: open,
                creator: Some(login),
                limit: limit.limit,
                ..Default::default()
            })?;
            let assigned = ctx.client.issues(repo).list(&IssueFilter {
                state: open,
                assignee: Some(login),
                limit: limit.limit,
                ..Default::default()
            })?;
            let status = out::IssueStatus { created, assigned };
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &status, |w| out::issue_status(w, &status))?;
        }
        IssueCmd::View { number, web } => {
            let repo = ctx.repo()?;
            if web {
                let url = crate::web::issue_url(&ctx.host, repo, &number);
                return crate::web::open_or_print(&url);
            }
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
            let mut title = title;
            let mut body = body;
            if super::interactive::should_run_interactive_create(title.as_deref(), false) {
                if !super::interactive::stdin_is_tty() {
                    return Err(super::interactive::missing_title_usage(
                        "issue create",
                        false,
                    ));
                }
                title = Some(super::interactive::prompt_title(None)?);
                if body.is_none() {
                    let editor = super::interactive::resolve_editor_from_env_and_config()?;
                    body = super::interactive::edit_body_in_editor("", &editor)?;
                }
            }
            let title = title.ok_or_else(|| {
                super::interactive::missing_title_usage("issue create", false)
            })?;
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
