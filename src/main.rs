use std::ffi::OsString;

use clap::Parser;
use gitee_cli_rs::config::{self, Config};
use gitee_cli_rs::error::GiteeError;

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
                std::process::exit(2);
            }
        }
    } else {
        let with_defaults = config::apply_defaults_os(args_os, &settings);
        gitee_cli_rs::cli::Cli::parse_from(with_defaults)
    };

    let json = cli.json.is_some();
    let debug = cli.debug;
    if let Err(e) = gitee_cli_rs::cmd::run(cli) {
        print_error(&e, json, debug);
        std::process::exit(e.exit_code());
    }
}

/// Print an error to stderr. When `--json` is in effect, emit a structured
/// JSON envelope; otherwise a human-readable `error: …` line.
fn print_error(e: &GiteeError, json: bool, debug: bool) {
    if json {
        let envelope = serde_json::json!({
            "code": e.code_slug(),
            "message": e.to_string(),
            "exit_code": e.exit_code(),
        });
        eprintln!("{}", serde_json::to_string(&envelope).unwrap_or_else(|_| format!("{{\"code\":\"error\",\"message\":\"{}\"}}", e)));
    } else if debug {
        eprintln!("error: {e:?}");
    } else {
        eprintln!("error: {e}");
    }
}

fn args_to_utf8(args: &[OsString]) -> Option<Vec<String>> {
    args.iter()
        .map(|a| a.clone().into_string().ok())
        .collect()
}
