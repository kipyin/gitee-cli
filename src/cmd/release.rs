use std::io::Write;

use super::Ctx;
use crate::api::releases::CreateRelease;
use crate::cli::ReleaseCmd;
use crate::error::Result;
use crate::out;

pub fn execute(ctx: &Ctx, cmd: ReleaseCmd) -> Result<()> {
    match cmd {
        ReleaseCmd::List { limit } => {
            let repo = ctx.repo()?;
            let items = ctx.client.releases(repo).list(limit.limit)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::release_table(w, &items))?;
        }
        ReleaseCmd::View { tag } => {
            let repo = ctx.repo()?;
            let release = ctx.client.releases(repo).get_by_tag(&tag)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &release, |w| out::one_release(w, &release))?;
        }
        ReleaseCmd::Create {
            tag,
            name,
            notes,
            target,
            prerelease,
        } => {
            let repo = ctx.repo()?;
            let req = CreateRelease {
                tag: &tag,
                name: name.as_deref(),
                notes: notes.as_deref(),
                target: target.as_deref(),
                prerelease,
            };
            let release = ctx.client.releases(repo).create(&req)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &release, |w| out::one_release(w, &release))?;
        }
        ReleaseCmd::Upload { tag, files } => {
            let repo = ctx.repo()?;
            let releases = ctx.client.releases(repo);
            let mut out = std::io::stdout().lock();
            for file_path in files {
                let asset = releases.upload(&tag, &file_path)?;
                writeln!(out, "{}", asset.name)?;
            }
        }
    }
    Ok(())
}
