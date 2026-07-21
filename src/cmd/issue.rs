use std::io::Write;

use super::{join_flags, resolve_milestone_opt, Ctx};
use crate::api::issues::{CreateIssue, EditIssue, IssueFilter};
use crate::cli::IssueCmd;
use crate::error::{GiteeError, Result};
use crate::models::IssueState;
use crate::out;

/// Clap already restricts `--state` to known values; map them onto `IssueState`.
fn parse_issue_state(raw: &str) -> Result<IssueState> {
    match raw {
        "open" => Ok(IssueState::Open),
        "progressing" => Ok(IssueState::Progressing),
        "closed" => Ok(IssueState::Closed),
        "rejected" => Ok(IssueState::Rejected),
        other => Err(GiteeError::Usage(format!(
            "unsupported issue state '{other}' (want open|progressing|closed|rejected)"
        ))),
    }
}

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
            if ctx.preview {
                let repo = ctx.repo()?;
                let t = title.clone().unwrap_or_default();
                let b = body.clone().unwrap_or_default();
                let a = assignee.clone().unwrap_or_default();
                let l = labels.clone().unwrap_or_default();
                let m = milestone.clone().unwrap_or_default();
                let repo_str = format!("{}/{}", repo.owner, repo.name);
                let sh = if security_hole { "true" } else { "false" };
                let details: Vec<(&str, &str)> = vec![
                    ("repo", &repo_str),
                    ("title", &t),
                    ("body", &b),
                    ("assignee", &a),
                    ("labels", &l),
                    ("milestone", &m),
                    ("security_hole", sh),
                ];
                println!("{}", super::preview_line("create issue", &details));
                return Ok(());
            }
            if super::interactive::should_run_interactive_create(title.as_deref(), false)
                && !super::interactive::stdin_is_tty()
            {
                return Err(super::interactive::missing_title_usage(
                    "issue create",
                    false,
                ));
            }
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
            state,
        } => {
            let repo = ctx.repo()?;
            let milestone_number = resolve_milestone_opt(ctx, repo, milestone.as_deref())?;
            let labels = join_flags(&label);
            let state = state
                .as_deref()
                .map(parse_issue_state)
                .transpose()?;
            let req = EditIssue {
                title: title.as_deref(),
                body: body.as_deref(),
                assignee: assignee.as_deref(),
                labels: labels.as_deref(),
                milestone_number,
                security_hole: security_hole.then_some(true),
                state,
            };
            let issue = ctx.client.issues(repo).edit(&number, &req)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &issue, |w| out::one_issue(w, &issue))?;
        }
        IssueCmd::Close { number } => {
            let repo = ctx.repo()?;
            if ctx.preview {
                println!("{}", super::preview_line(
                    &format!("close issue {number}"),
                    &[("repo", &format!("{}/{}", repo.owner, repo.name))],
                ));
                return Ok(());
            }
            let change = ctx
                .client
                .issues(repo)
                .set_state_idempotent(&number, IssueState::Closed)?;
            render_idempotent_issue(ctx, change, "closed", &number)?;
        }
        IssueCmd::Reopen { number } => {
            let repo = ctx.repo()?;
            if ctx.preview {
                println!("{}", super::preview_line(
                    &format!("reopen issue {number}"),
                    &[("repo", &format!("{}/{}", repo.owner, repo.name))],
                ));
                return Ok(());
            }
            let change = ctx
                .client
                .issues(repo)
                .set_state_idempotent(&number, IssueState::Open)?;
            render_idempotent_issue(ctx, change, "open", &number)?;
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

/// Render an idempotent close/reopen result. On the `Already` branch print a
/// human "already <state>" message (or a structured `--json` object); on the
/// `Changed` branch render the updated issue through the normal renderer.
fn render_idempotent_issue(
    ctx: &Ctx,
    change: crate::api::StateChange<crate::models::Issue>,
    state_word: &str,
    number: &str,
) -> Result<()> {
    use crate::api::StateChange;
    let mut out = std::io::stdout().lock();
    match change {
        StateChange::Changed(issue) => {
            ctx.out
                .render(&mut out, &issue, |w| out::one_issue(w, &issue))?;
        }
        StateChange::Already(issue) => {
            let line = format_already_message(&issue, state_word, number, ctx.out.json.is_some());
            writeln!(out, "{line}")?;
        }
    }
    Ok(())
}

/// Build the "already <state>" line. JSON mode emits a structured envelope
/// `{"number","state","message"}`; human mode emits `issue <n> already <state>`.
fn format_already_message(
    issue: &crate::models::Issue,
    state_word: &str,
    number: &str,
    json: bool,
) -> String {
    if json {
        let envelope = serde_json::json!({
            "number": issue.number,
            "state": issue.state.as_str(),
            "message": format!("already {state_word}"),
        });
        serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| envelope.to_string())
    } else {
        format!("issue {number} already {state_word}")
    }
}

#[cfg(test)]
mod idempotent_message_tests {
    use super::format_already_message;
    use crate::models::{Issue, IssueState};

    fn closed_issue() -> Issue {
        Issue {
            number: "I88".into(),
            title: "Bug".into(),
            state: IssueState::Closed,
            ..Default::default()
        }
    }

    #[test]
    fn human_message_names_issue_and_state() {
        let line = format_already_message(&closed_issue(), "closed", "I88", false);
        assert_eq!(line, "issue I88 already closed");
    }

    #[test]
    fn json_message_emits_structured_envelope() {
        let line = format_already_message(&closed_issue(), "closed", "I88", true);
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["number"], "I88");
        assert_eq!(v["state"], "closed");
        assert_eq!(v["message"], "already closed");
    }
}
