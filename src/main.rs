use std::ffi::OsString;

use clap::Parser;
use gitee_cli_rs::config::{self, Config};

fn main() {
    let settings = Config::load_settings().unwrap_or_else(|_| Default::default());
    let args_os: Vec<OsString> = std::env::args_os().collect();
    let debug = args_os.iter().any(|a| a == "--debug");

    let cli = if let Some(raw) = args_to_utf8(&args_os) {
        let with_defaults = config::apply_defaults(raw.clone(), &settings);
        match config::expand_aliases(with_defaults.clone(), &settings.aliases) {
            Ok(expanded) => {
                if debug && expanded != with_defaults {
                    eprintln!("alias expanded: {}", expanded.join(" "));
                }
                gitee_cli_rs::cli::Cli::parse_from(expanded)
            }
            Err(e) => {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
    } else {
        let with_defaults = config::apply_defaults_os(args_os, &settings);
        gitee_cli_rs::cli::Cli::parse_from(with_defaults)
    };

    if let Err(e) = gitee_cli_rs::cmd::run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn args_to_utf8(args: &[OsString]) -> Option<Vec<String>> {
    args.iter()
        .map(|a| a.clone().into_string().ok())
        .collect()
}
