use std::io::Write;

use super::{confirm, Ctx};
use crate::cli::CollaboratorCmd;
use crate::error::Result;
use crate::out;

pub fn execute(ctx: &Ctx, cmd: CollaboratorCmd) -> Result<()> {
    match cmd {
        CollaboratorCmd::List { limit } => {
            let repo = ctx.repo()?;
            let items = ctx.client.collaborators(repo).list(limit.limit)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::collaborator_table(w, &items))?;
        }
        CollaboratorCmd::Add {
            username,
            permission,
        } => {
            let repo = ctx.repo()?;
            ctx.client
                .collaborators(repo)
                .add(&username, &permission)?;
            writeln!(
                std::io::stdout().lock(),
                "Added collaborator {username} with permission {permission}"
            )?;
        }
        CollaboratorCmd::Remove { username, yes } => {
            let repo = ctx.repo()?;
            confirm(&format!("Remove collaborator {username}"), yes)?;
            ctx.client.collaborators(repo).remove(&username)?;
            writeln!(
                std::io::stdout().lock(),
                "Removed collaborator {username}"
            )?;
        }
    }
    Ok(())
}
