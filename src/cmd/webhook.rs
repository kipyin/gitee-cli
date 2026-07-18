use std::io::Write;

use super::{confirm, join_flags, Ctx};
use crate::api::webhooks::CreateWebhook;
use crate::cli::WebhookCmd;
use crate::error::{GiteeError, Result};
use crate::out;

const EVENT_FLAGS: &[&str] = &[
    "push_events",
    "tag_push_events",
    "issues_events",
    "merge_requests_events",
    "pull_requests_events", // alias → merge_requests_events
    "note_events",
];

pub fn execute(ctx: &Ctx, cmd: WebhookCmd) -> Result<()> {
    match cmd {
        WebhookCmd::List { limit } => {
            let repo = ctx.repo()?;
            let items = ctx.client.webhooks(repo).list(limit.limit)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::webhook_table(w, &items))?;
        }
        WebhookCmd::Create {
            url,
            events,
            password,
        } => {
            let repo = ctx.repo()?;
            let selected = parse_events(&events)?;
            let hook = ctx.client.webhooks(repo).create(&CreateWebhook {
                url: &url,
                password: password.as_deref(),
                push_events: selected.iter().any(|e| e == "push_events"),
                tag_push_events: selected.iter().any(|e| e == "tag_push_events"),
                issues_events: selected.iter().any(|e| e == "issues_events"),
                merge_requests_events: selected.iter().any(|e| {
                    e == "merge_requests_events" || e == "pull_requests_events"
                }),
                note_events: selected.iter().any(|e| e == "note_events"),
            })?;
            let items = [hook];
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::webhook_table(w, &items))?;
        }
        WebhookCmd::Delete { id, yes } => {
            let repo = ctx.repo()?;
            confirm(&format!("Delete webhook {id}"), yes)?;
            ctx.client.webhooks(repo).delete(id)?;
            writeln!(std::io::stdout().lock(), "Deleted webhook {id}")?;
        }
    }
    Ok(())
}

fn parse_events(events: &[String]) -> Result<Vec<String>> {
    let joined = join_flags(events).unwrap_or_default();
    if joined.is_empty() {
        // Default to push when nothing specified (common webhook default).
        return Ok(vec!["push_events".into()]);
    }
    let mut out = Vec::new();
    for ev in joined.split(',') {
        let ev = ev.trim();
        if !EVENT_FLAGS.contains(&ev) {
            return Err(GiteeError::Usage(format!(
                "unknown webhook event '{ev}'; expected one of {}",
                EVENT_FLAGS.join(", ")
            )));
        }
        if !out.iter().any(|e| e == ev) {
            out.push(ev.to_string());
        }
    }
    Ok(out)
}
