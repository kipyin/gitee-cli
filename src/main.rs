use clap::Parser;

fn main() {
    let cli = gitee::cli::Cli::parse();
    if let Err(e) = gitee::cmd::run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
