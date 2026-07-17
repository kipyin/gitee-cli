use super::Ctx;
use crate::cli::IssueCmd;
use crate::error::Result;
use crate::models::{Comment, Issue};
use crate::out;

pub fn execute(ctx: &Ctx, cmd: IssueCmd) -> Result<()> {
    let o = ctx.repo.owner.as_str();
    let r = ctx.repo.name.as_str();
    match cmd {
        IssueCmd::List {
            state,
            assignee,
            limit,
        } => {
            let mut q: Vec<(&str, String)> = Vec::new();
            if let Some(s) = state {
                q.push(("state", s));
            }
            if let Some(a) = assignee {
                q.push(("assignee", a));
            }
            let qref: Vec<(&str, &str)> = q.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let path = format!("/repos/{o}/{r}/issues");
            let items: Vec<Issue> = ctx.client.get_paged(&path, &qref, limit)?;
            ctx.out.render(&items, || out::issue_table(&items));
        }
        IssueCmd::View { number } => {
            let issue: Issue = ctx
                .client
                .get(&format!("/repos/{o}/{r}/issues/{number}"), &[])?;
            ctx.out.render(&issue, || out::one_issue(&issue));
        }
        IssueCmd::Create {
            title,
            body,
            assignee,
            labels,
        } => {
            // Gitee quirk: issue creation is POST /repos/{owner}/issues with `repo` as a form field.
            let mut f: Vec<(&str, String)> =
                vec![("repo", ctx.repo.name.clone()), ("title", title)];
            if let Some(b) = body {
                f.push(("body", b));
            }
            if let Some(a) = assignee {
                f.push(("assignee", a));
            }
            if let Some(l) = labels {
                f.push(("labels", l));
            }
            let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let issue: Issue = ctx.client.post(&format!("/repos/{o}/issues"), &form)?;
            ctx.out.render(&issue, || out::one_issue(&issue));
        }
        IssueCmd::Close { number } => {
            let issue = set_state(ctx, &number, "closed")?;
            ctx.out.render(&issue, || out::one_issue(&issue));
        }
        IssueCmd::Reopen { number } => {
            let issue = set_state(ctx, &number, "open")?;
            ctx.out.render(&issue, || out::one_issue(&issue));
        }
        IssueCmd::Link { number, pr } => {
            let cur: Issue = ctx
                .client
                .get(&format!("/repos/{o}/{r}/issues/{number}"), &[])?;
            let old = cur.body.clone().unwrap_or_default();
            let tag = format!("!{pr}");
            if old.contains(tag.as_str()) {
                println!("Issue #{number} already references {tag}");
            } else {
                let new = format!("{old}\n\nLinked: {tag}");
                let body = serde_json::json!({
                    "repo": ctx.repo.name,
                    "title": cur.title,
                    "body": new,
                });
                let _issue: Issue = ctx
                    .client
                    .patch_json(&format!("/repos/{o}/issues/{number}"), &body)?;
                println!("Linked pull request {tag} on issue #{number}");
            }
        }
        IssueCmd::Comment { number, body } => {
            let f: Vec<(&str, String)> = vec![("body", body)];
            let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let c: Comment = ctx
                .client
                .post(&format!("/repos/{o}/{r}/issues/{number}/comments"), &form)?;
            ctx.out.render(&c, || out::comment_line(&c));
        }
    }
    Ok(())
}

/// Gitee quirk: issue state changes are PATCH /repos/{owner}/issues/{number}
/// with a JSON body `{repo, title, state}`. The current title must be echoed
/// back or Gitee blanks it.
fn set_state(ctx: &Ctx, number: &str, state: &str) -> Result<Issue> {
    let o = ctx.repo.owner.as_str();
    let name = &ctx.repo.name;
    let cur: Issue = ctx
        .client
        .get(&format!("/repos/{o}/{name}/issues/{number}"), &[])?;
    let body = serde_json::json!({
        "repo": ctx.repo.name,
        "title": cur.title,
        "state": state,
    });
    ctx.client
        .patch_json(&format!("/repos/{o}/issues/{number}"), &body)
}
