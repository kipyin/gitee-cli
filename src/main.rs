use clap::Parser;
use gitee_cli_rs::config::{self, Config};

fn main() {
    let settings = Config::load_settings().unwrap_or_else(|_| Default::default());
    let debug = std::env::args_os().any(|a| a == "--debug");
    let mut argv: Vec<String> = std::env::args().collect();
    argv = config::apply_defaults(argv, &settings);
    match config::expand_aliases(argv, &settings.aliases) {
        Ok(expanded) => {
            if debug && expanded != std::env::args().collect::<Vec<_>>() {
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
