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
            let items: Vec<ExtensionEntry> = extension::list_on_path()
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
    }
    Ok(())
}
