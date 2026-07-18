use clap::Parser;
use gitee_cli_rs::config::{self, Config};

fn main() {
    let settings = Config::load_settings().unwrap_or_else(|_| Default::default());
    let debug = std::env::args_os().any(|a| a == "--debug");
    let raw: Vec<String> = std::env::args().collect();
    let with_defaults = config::apply_defaults(raw.clone(), &settings);
    match config::expand_aliases(with_defaults.clone(), &settings.aliases) {
        Ok(expanded) => {
            if debug && expanded != with_defaults {
                eprintln!("alias expanded: {}", expanded.join(" "));
            }
            let cli = gitee_cli_rs::cli::Cli::parse_from(expanded);
            if let Err(e) = gitee_cli_rs::cmd::run(cli) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
