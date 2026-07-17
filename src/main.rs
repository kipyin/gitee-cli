mod api;
mod cli;
mod cmd;
mod config;
mod error;
mod models;
mod out;
mod repo;
use crate::cli::Cli;
use clap::Parser;
fn main() {
    let cli = Cli::parse();
    if let Err(e) = cmd::run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
