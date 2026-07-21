use std::io::Write;

use super::Ctx;
use crate::cli::ExtensionCmd;
use crate::error::Result;
use crate::extension;
use serde::Serialize;

#[derive(Serialize)]
struct ExtensionEntry {
    name: String,
}

pub fn execute(ctx: &Ctx, cmd: ExtensionCmd) -> Result<()> {
    match cmd {
        ExtensionCmd::List => {
            let mut names = extension::list_on_path();
            for n in extension::list_installed()? {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
            names.sort();
            names.dedup();
            let items: Vec<ExtensionEntry> = names
                .into_iter()
                .map(|name| ExtensionEntry { name })
                .collect();
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &items, |w| {
                if items.is_empty() {
                    writeln!(w, "(no extensions)")?;
                } else {
                    for item in &items {
                        writeln!(w, "{}", item.name)?;
                    }
                }
                Ok(())
            })?;
        }
        ExtensionCmd::Install { repo, build, yes } => {
            extension::install(&repo, build.as_deref(), yes, &ctx.host)?;
        }
        ExtensionCmd::Create { name, cargo } => {
            extension::create(&name, cargo)?;
        }
        ExtensionCmd::Remove { name, yes } => {
            extension::remove(&name, yes)?;
        }
        ExtensionCmd::Upgrade { name } => {
            extension::upgrade(name.as_deref())?;
        }
    }
    Ok(())
}
