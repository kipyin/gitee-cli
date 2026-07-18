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
            let entry = AliasEntry {
                name: name.clone(),
                expansion: expansion.clone(),
            };
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &entry, |w| {
                writeln!(w, "alias {name}='{expansion}'")?;
                Ok(())
            })?;
        }
        AliasCmd::Delete { name } => {
            Config::alias_delete(&name)?;
            let entry = AliasEntry {
                name: name.clone(),
                expansion: String::new(),
            };
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &entry, |w| {
                writeln!(w, "Deleted alias {name}")?;
                Ok(())
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{AliasCmd, Cli};
    use clap::Parser;

    #[test]
    fn alias_set_and_delete_use_output_render() {
        let _env = crate::config::test_config_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "gitee-cli-alias-json-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::config::set_test_dir(Some(dir.clone()));
        let cli = Cli::try_parse_from([
            "gitee",
            "--json=",
            "alias",
            "set",
            "co",
            "pr",
            "checkout",
        ])
        .unwrap();
        let ctx = super::super::build_inner(&cli, false).unwrap();
        execute(
            &ctx,
            AliasCmd::Set {
                name: "co".into(),
                expansion: vec!["pr".into(), "checkout".into()],
            },
        )
        .unwrap();
        execute(&ctx, AliasCmd::Delete { name: "co".into() }).unwrap();
        crate::config::set_test_dir(None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
