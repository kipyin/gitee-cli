use std::io::Write;

use super::{confirm, Ctx};
use crate::api::labels::{CreateLabel, EditLabel};
use crate::cli::LabelCmd;
use crate::error::Result;
use crate::out;

// Follow-up (ticket 08 non-goal): `gh label clone` cross-repo copy is not implemented.
pub fn execute(ctx: &Ctx, cmd: LabelCmd) -> Result<()> {
    match cmd {
        LabelCmd::List { limit } => {
            let repo = ctx.repo()?;
            let items = ctx.client.labels(repo).list(limit.limit)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::label_table(w, &items))?;
        }
        LabelCmd::Create { name, color } => {
            let repo = ctx.repo()?;
            if ctx.preview {
                println!("{}", super::preview_line(
                    &format!("create label {name}"),
                    &[
                        ("repo", &format!("{}/{}", repo.owner, repo.name)),
                        ("color", &color),
                    ],
                ));
                return Ok(());
            }
            let change = ctx.client.labels(repo).create_idempotent(&CreateLabel {
                name: &name,
                color: &color,
            })?;
            let mut out = std::io::stdout().lock();
            match change {
                crate::api::StateChange::Already(label) => {
                    writeln!(out, "Label {name} already exists (color {})", label.color.as_deref().unwrap_or("(none)"))?;
                }
                crate::api::StateChange::Changed(label) => {
                    let items = [label];
                    ctx.out
                        .render(&mut out, &items, |w| out::label_table(w, &items))?;
                }
            }
        }
        LabelCmd::Edit {
            name,
            new_name,
            color,
        } => {
            let repo = ctx.repo()?;
            let label = ctx.client.labels(repo).edit(
                &name,
                &EditLabel {
                    name: new_name.as_deref(),
                    color: color.as_deref(),
                },
            )?;
            let items = [label];
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::label_table(w, &items))?;
        }
        LabelCmd::Delete { name, yes } => {
            let repo = ctx.repo()?;
            confirm(&format!("Delete label {name}"), yes)?;
            ctx.client.labels(repo).delete(&name)?;
            writeln!(std::io::stdout().lock(), "Deleted label {name}")?;
        }
    }
    Ok(())
}
