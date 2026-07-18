use std::io::Write;

use super::{confirm, Ctx};
use crate::api::webhooks::{self, CreateWebhook};
use crate::cli::WebhookCmd;
use crate::error::Result;
use crate::out;

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
            let selected = webhooks::parse_events(&events)?;
            let (push_events, tag_push_events, issues_events, merge_requests_events, note_events) =
                webhooks::event_bools(&selected);
            let hook = ctx.client.webhooks(repo).create(&CreateWebhook {
                url: &url,
                password: password.as_deref(),
                push_events,
                tag_push_events,
                issues_events,
                merge_requests_events,
                note_events,
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
