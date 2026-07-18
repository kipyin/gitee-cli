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
            println!("set {key}={value}");
        }
    }
    Ok(())
}
