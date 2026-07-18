use std::io::Write;

use super::Ctx;
use crate::cli::AliasCmd;
use crate::config::Config;
use crate::error::Result;
use serde::Serialize;

#[derive(Serialize)]
struct AliasEntry {
    name: String,
    expansion: String,
}

pub fn execute(ctx: &Ctx, cmd: AliasCmd) -> Result<()> {
    match cmd {
        AliasCmd::List => {
            let items: Vec<AliasEntry> = Config::alias_list()?
                .into_iter()
                .map(|(name, expansion)| AliasEntry { name, expansion })
                .collect();
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &items, |w| {
                if items.is_empty() {
                    writeln!(w, "(no aliases)")?;
                } else {
                    for item in &items {
                        writeln!(w, "alias {}='{}'", item.name, item.expansion)?;
                    }
                }
                Ok(())
            })?;
        }
        AliasCmd::Set { name, expansion } => {
            let expansion = expansion.join(" ");
            Config::alias_set(&name, &expansion)?;
            println!("alias {name}='{expansion}'");
        }
        AliasCmd::Delete { name } => {
            Config::alias_delete(&name)?;
            println!("Deleted alias {name}");
        }
    }
    Ok(())
}
