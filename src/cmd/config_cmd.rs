use std::io::Write;

use crate::cli::ConfigCmd;
use crate::config::Config;
use crate::error::{GiteeError, Result};

pub fn execute(cmd: ConfigCmd) -> Result<()> {
    match cmd {
        ConfigCmd::List => {
            let items = Config::list_keys()?;
            let mut out = std::io::stdout().lock();
            if items.is_empty() {
                writeln!(out, "(no config keys set)")?;
            } else {
                for (k, v) in items {
                    writeln!(out, "{k}={v}")?;
                }
            }
        }
        ConfigCmd::Get { key } => {
            match Config::get_key(&key)? {
                Some(v) => println!("{v}"),
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
