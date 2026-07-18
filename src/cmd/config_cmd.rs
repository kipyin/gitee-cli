use std::io::Write;

use super::Ctx;
use crate::cli::ConfigCmd;
use crate::config::Config;
use crate::error::{GiteeError, Result};
use serde::Serialize;

#[derive(Serialize)]
struct ConfigEntry {
    key: String,
    value: String,
}

pub fn execute(ctx: &Ctx, cmd: ConfigCmd) -> Result<()> {
    match cmd {
        ConfigCmd::List => {
            let items: Vec<ConfigEntry> = Config::list_keys()?
                .into_iter()
                .map(|(key, value)| ConfigEntry { key, value })
                .collect();
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &items, |w| {
                if items.is_empty() {
                    writeln!(w, "(no config keys set)")?;
                } else {
                    for item in &items {
                        writeln!(w, "{}={}", item.key, item.value)?;
                    }
                }
                Ok(())
            })?;
        }
        ConfigCmd::Get { key } => {
            match Config::get_key(&key)? {
                Some(value) => {
                    let entry = ConfigEntry {
                        key: key.clone(),
                        value: value.clone(),
                    };
                    let mut out = std::io::stdout().lock();
                    ctx.out.render(&mut out, &entry, |w| {
                        writeln!(w, "{value}")?;
                        Ok(())
                    })?;
                }
                None => {
                    return Err(GiteeError::Usage(format!(
                        "config key '{key}' is not set"
                    )))
                }
            }
        }
        ConfigCmd::Set { key, value } => {
            Config::set_key(&key, &value)?;
            let entry = ConfigEntry {
                key: key.clone(),
                value: value.clone(),
            };
            let mut out = std::io::stdout().lock();
            ctx.out.render(&mut out, &entry, |w| {
                writeln!(w, "set {key}={value}")?;
                Ok(())
            })?;
        }
    }
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ConfigCmd;
    use clap::Parser;
    #[test]
    fn config_set_uses_output_render() {
        let _env = crate::config::test_config_env_lock();
        let dir = std::env::temp_dir().join(format!(
            "gitee-cli-config-set-json-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::config::set_test_dir(Some(dir.clone()));
        let cli = crate::cli::Cli::try_parse_from(["gitee", "--json=", "config", "set", "editor", "vim"]).unwrap();
        let ctx = super::super::build_inner(&cli, false).unwrap();
        execute(&ctx, ConfigCmd::Set {
            key: "editor".into(),
            value: "vim".into(),
        })
        .unwrap();
        crate::config::set_test_dir(None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
