use clap::Parser;

fn main() {
    let cli = gitee_cli_rs::cli::Cli::parse();
    if let Err(e) = gitee_cli_rs::cmd::run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
