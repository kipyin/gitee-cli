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
            if ctx.out.json {
                out::json(&items);
            } else {
                out::issue_table(&items);
            }
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
            if ctx.out.json {
                out::json(&issue);
            } else {
                out::one_issue(&issue);
            }
        }
        IssueCmd::Close { number } => {
            let f: Vec<(&str, String)> = vec![("state", "closed".to_string())];
            let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let issue: Issue = ctx
                .client
                .patch(&format!("/repos/{o}/{r}/issues/{number}"), &form)?;
            if ctx.out.json {
                out::json(&issue);
            } else {
                out::one_issue(&issue);
            }
        }
        IssueCmd::Link { number, pr } => {
            let issue: Issue = ctx
                .client
                .get(&format!("/repos/{o}/{r}/issues/{number}"), &[])?;
            let cur = issue.body.clone().unwrap_or_default();
            let tag = format!("!{pr}");
            if cur.contains(tag.as_str()) {
                println!("Issue #{number} already references {tag}");
            } else {
                let new = format!("{cur}\n\nLinked: {tag}");
                let f: Vec<(&str, String)> = vec![("body", new)];
                let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
                let _issue: Issue = ctx
                    .client
                    .patch(&format!("/repos/{o}/{r}/issues/{number}"), &form)?;
                println!("Linked pull request {tag} on issue #{number}");
            }
        }
        IssueCmd::Comment { number, body } => {
            let f: Vec<(&str, String)> = vec![("body", body)];
            let form: Vec<(&str, &str)> = f.iter().map(|(k, v)| (*k, v.as_str())).collect();
            let c: Comment = ctx
                .client
                .post(&format!("/repos/{o}/{r}/issues/{number}/comments"), &form)?;
            if ctx.out.json {
                out::json(&c);
            } else {
                out::comment_line(&c);
            }
        }
    }
    Ok(())
}
