use std::io::Write;

use crate::cli::AliasCmd;
use crate::config::Config;
use crate::error::Result;

pub fn execute(cmd: AliasCmd) -> Result<()> {
    match cmd {
        AliasCmd::List => {
            let items = Config::alias_list()?;
            let mut out = std::io::stdout().lock();
            if items.is_empty() {
                writeln!(out, "(no aliases)")?;
            } else {
                for (name, expansion) in items {
                    writeln!(out, "alias {name}='{expansion}'")?;
                }
            }
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
