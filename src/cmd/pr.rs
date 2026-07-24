use std::io::Write;

use super::{join_flags, resolve_milestone_opt, Ctx};
use crate::api::pulls::{CreatePr, EditPr, PrFilter};
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
                ..Default::default()
            };
            let items = ctx.client.pulls(repo).list(&filter)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::pr_table(w, &items))?;
        }
        PrCmd::Status { limit } => {
            let repo = ctx.repo()?;
            let me = ctx.me()?;
            let login = me.login.as_str();
            let open = Some("open");
            let created = ctx.client.pulls(repo).list(&PrFilter {
                state: open,
                author: Some(login),
                limit: limit.limit,
                ..Default::default()
            })?;
            let assigned = ctx.client.pulls(repo).list(&PrFilter {
                state: open,
                assignee: Some(login),
                limit: limit.limit,
                ..Default::default()
            })?;
            let awaiting_test = ctx.client.pulls(repo).list(&PrFilter {
                state: open,
                tester: Some(login),
                limit: limit.limit,
                ..Default::default()
            })?;
            let status = out::PrStatus {
                created,
                assigned,
                awaiting_test,
            };
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &status, |w| out::pr_status(w, &status))?;
        }
        PrCmd::View { number, web } => {
            let repo = ctx.repo()?;
            if web {
                let url = crate::web::pull_url(&ctx.host, repo, number);
                return crate::web::open_or_print(&url);
            }
            let mut pr = ctx.client.pulls(repo).get(number)?;
            let files = ctx.client.pulls(repo).files(number)?;
            pr.files = Some(files);
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
        PrCmd::Edit {
            number,
            title,
            body,
            assignee,
            tester,
            label,
            milestone,
        } => {
            let repo = ctx.repo()?;
            let milestone_number = resolve_milestone_opt(ctx, repo, milestone.as_deref())?;
            let labels = join_flags(&label);
            let assignees = join_flags(&assignee);
            let testers = join_flags(&tester);
            let req = EditPr {
                title: title.as_deref(),
                body: body.as_deref(),
                labels: labels.as_deref(),
                assignees: assignees.as_deref(),
                testers: testers.as_deref(),
                milestone_number,
            };
            let pr = ctx.client.pulls(repo).edit(number, &req)?;
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &pr, |w| out::one_pr(w, &pr))?;
        }
        PrCmd::Create {
            title,
            body,
            head,
            base,
            fill,
            assignee,
            tester,
            label,
            milestone,
            close_issue,
        } => {
            // `--preview` short-circuits before any git/API work: print intent and exit 0.
            if ctx.preview {
                let repo = ctx.repo()?;
                let head = head.clone().unwrap_or_else(|| "<current-branch>".into());
                let base = base.clone().unwrap_or_else(|| repo.name.clone() + " default branch");
                let title = title.clone().unwrap_or_default();
                let body = body.clone().unwrap_or_default();
                let repo_str = format!("{}/{}", repo.owner, repo.name);
                let mut details: Vec<(&str, &str)> = vec![
                    ("repo", &repo_str),
                    ("title", &title),
                    ("head", &head),
                    ("base", &base),
                ];
                if !body.is_empty() {
                    details.push(("body", &body));
                }
                if fill {
                    details.push(("fill", "true"));
                }
                println!("{}", super::preview_line("create pull request", &details));
                return Ok(());
            }
            if super::interactive::should_run_interactive_create(title.as_deref(), fill)
                && !super::interactive::stdin_is_tty()
            {
                return Err(super::interactive::missing_title_usage("pr create", true));
            }
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
            let mut title = title;
            let mut body = body;
            if fill {
                let subjects = git_log_subjects(&base, &head)?;
                let (ft, fb) = fill_from_subjects(&subjects)?;
                if title.is_none() {
                    title = Some(ft);
                }
                if body.is_none() {
                    body = Some(fb);
                }
            }
            if super::interactive::should_run_interactive_create(title.as_deref(), fill) {
                if !super::interactive::stdin_is_tty() {
                    return Err(super::interactive::missing_title_usage("pr create", true));
                }
                title = Some(super::interactive::prompt_title(None)?);
                if body.is_none() {
                    let initial =
                        fetch_pr_template(ctx, repo, &base)?.unwrap_or_default();
                    let editor = super::interactive::resolve_editor_from_env_and_config()?;
                    body = super::interactive::edit_body_in_editor(&initial, &editor)?;
                }
            }
            let title = title.ok_or_else(|| {
                super::interactive::missing_title_usage("pr create", true)
            })?;
            if body.is_none() {
                body = fetch_pr_template(ctx, repo, &base)?;
            }
            let milestone_number = resolve_milestone_opt(ctx, repo, milestone.as_deref())?;
            let labels = join_flags(&label);
            let assignees = join_flags(&assignee);
            let testers = join_flags(&tester);
            let req = CreatePr {
                title: &title,
                head: &head,
                base: &base,
                body: body.as_deref(),
                labels: labels.as_deref(),
                assignees: assignees.as_deref(),
                testers: testers.as_deref(),
                milestone_number,
                issue: close_issue.as_deref(),
                close_related_issue: close_issue.is_some(),
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
            if ctx.preview {
                println!("{}", super::preview_line(
                    &format!("merge pull request !{number} ({})", method.as_str()),
                    &[
                        ("repo", &format!("{}/{}", repo.owner, repo.name)),
                        ("close_related_issue", if no_close_issue { "false" } else { "true" }),
                    ],
                ));
                return Ok(());
            }
            let change = ctx
                .client
                .pulls(repo)
                .merge_idempotent(number, method, !no_close_issue)?;
            let mut out = std::io::stdout().lock();
            match change {
                crate::api::StateChange::Changed(()) => {
                    writeln!(out, "Merged pull request !{number}")?;
                }
                crate::api::StateChange::Already(()) => {
                    writeln!(out, "Pull request !{number} already merged")?;
                }
            }
        }
        PrCmd::Comment(crate::cli::PrCommentCmd::Create { number, body }) => {
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
        PrCmd::Test { number, force } => {
            let repo = ctx.repo()?;
            ctx.client.pulls(repo).test(number, force)?;
            let confirm = serde_json::json!({
                "number": number,
                "tested": true,
                "force": force,
            });
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &confirm, |w| {
                writeln!(w, "Marked pull request !{number} as tested")
            })?;
        }
        PrCmd::Close { number } => {
            let repo = ctx.repo()?;
            if ctx.preview {
                println!("{}", super::preview_line(
                    &format!("close pull request !{number}"),
                    &[("repo", &format!("{}/{}", repo.owner, repo.name))],
                ));
                return Ok(());
            }
            let change = ctx
                .client
                .pulls(repo)
                .set_state_idempotent(number, PrState::Closed)?;
            render_idempotent_pr(ctx, change, "closed", number)?;
        }
        PrCmd::Reopen { number } => {
            let repo = ctx.repo()?;
            if ctx.preview {
                println!("{}", super::preview_line(
                    &format!("reopen pull request !{number}"),
                    &[("repo", &format!("{}/{}", repo.owner, repo.name))],
                ));
                return Ok(());
            }
            let change = ctx
                .client
                .pulls(repo)
                .set_state_idempotent(number, PrState::Open)?;
            render_idempotent_pr(ctx, change, "open", number)?;
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

/// Commit subjects in `base..head`, oldest first, for --fill.
fn git_log_subjects(base: &str, head: &str) -> Result<Vec<String>> {
    let range = format!("{base}..{head}");
    let out = std::process::Command::new("git")
        .args(["log", "--reverse", "--format=%s", &range])
        .output()
        .map_err(|e| GiteeError::Usage(format!("git: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(GiteeError::Usage(format!(
            "git log {range} failed: {}",
            stderr.trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect())
}

/// gh --fill semantics: title = first (oldest) commit subject, body = the
/// commit list as markdown bullets.
fn fill_from_subjects(subjects: &[String]) -> Result<(String, String)> {
    let first = subjects.first().ok_or_else(|| {
        GiteeError::Usage("no commits in base..head range; nothing to --fill from".into())
    })?;
    let body = subjects
        .iter()
        .map(|s| format!("- {s}"))
        .collect::<Vec<_>>()
        .join("\n");
    Ok((first.clone(), body))
}

/// Template prefill, tried in order on the base ref; missing template = no body.
fn fetch_pr_template(ctx: &Ctx, repo: &crate::repo::Repo, base: &str) -> Result<Option<String>> {
    for path in [
        ".gitee/PULL_REQUEST_TEMPLATE.md",
        "PULL_REQUEST_TEMPLATE.md",
    ] {
        if let Some(c) = ctx
            .client
            .repos()
            .file_contents(&repo.owner, &repo.name, path, base)?
        {
            return Ok(Some(c));
        }
    }
    Ok(None)
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

/// Render an idempotent PR close/reopen result. On `Already`, print a human
/// "already <state>" line (or a structured `--json` envelope). On `Changed`,
/// render the updated PR through the normal renderer.
fn render_idempotent_pr(
    ctx: &Ctx,
    change: crate::api::StateChange<crate::models::PullRequest>,
    state_word: &str,
    number: i64,
) -> Result<()> {
    use crate::api::StateChange;
    let mut out = std::io::stdout().lock();
    match change {
        StateChange::Changed(pr) => {
            ctx.out.render(&mut out, &pr, |w| out::one_pr(w, &pr))?;
        }
        StateChange::Already(pr) => {
            if ctx.out.json.is_some() {
                let envelope = serde_json::json!({
                    "number": pr.number,
                    "state": pr.state.as_str(),
                    "message": format!("already {state_word}"),
                });
                writeln!(out, "{}", serde_json::to_string_pretty(&envelope).unwrap())?;
            } else {
                writeln!(out, "Pull request !{number} already {state_word}")?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod fill_tests {
    #[test]
    fn fill_from_subjects_single_commit() {
        let (title, body) = super::fill_from_subjects(&["Add paging".to_string()]).unwrap();
        assert_eq!(title, "Add paging");
        assert_eq!(body, "- Add paging");
    }

    #[test]
    fn fill_from_subjects_uses_oldest_subject_as_title() {
        let subjects = vec!["First commit".to_string(), "Second commit".to_string()];
        let (title, body) = super::fill_from_subjects(&subjects).unwrap();
        assert_eq!(title, "First commit");
        assert_eq!(body, "- First commit\n- Second commit");
    }

    #[test]
    fn fill_from_subjects_empty_is_usage_error() {
        assert!(super::fill_from_subjects(&[]).is_err());
    }
}
