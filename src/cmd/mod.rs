use std::str::FromStr;

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::api::client::Client;
use crate::cli::{Cli, Command};
use crate::config::Config;
use crate::error::{GiteeError, Result};
use crate::out::Output;
use crate::repo::Repo;

pub mod auth;
pub mod issue;
pub mod pr;
pub mod repo;

pub struct Ctx {
    pub client: Client,
    pub repo: Repo,
    pub out: Output,
}

pub fn run(cli: Cli) -> Result<()> {
    match &cli.cmd {
        Command::Auth(c) => auth::execute(c.clone(), &cli.host),
        Command::Pr(c) => {
            let ctx = build(&cli)?;
            pr::execute(&ctx, c.clone())
        }
        Command::Issue(c) => {
            let ctx = build(&cli)?;
            issue::execute(&ctx, c.clone())
        }
        Command::Repo(c) => {
            let ctx = build(&cli)?;
            repo::execute(&ctx, c.clone())
        }
        Command::Completions { shell } => completions(shell.clone()),
    }
}

fn build(cli: &Cli) -> Result<Ctx> {
    let token = Config::token(&cli.host)?;
    let mut client = Client::new(format!("https://{}/api/v5", cli.host), token);
    client.set_debug(cli.debug);
    let repo = Repo::resolve(cli.repo.as_deref(), cli.remote.as_deref())?;
    Ok(Ctx {
        client,
        repo,
        out: Output {
            json: cli.json.clone(),
        },
    })
}

fn completions(shell: Option<String>) -> Result<()> {
    let shell = match shell.as_deref() {
        Some(s) => Shell::from_str(s).map_err(|_| {
            GiteeError::Usage(format!(
                "unknown shell '{s}'; use one of: bash, zsh, fish, powershell, elvish"
            ))
        })?,
        None => detect_shell()?,
    };
    let mut cmd: clap::Command = crate::cli::Cli::command();
    generate(shell, &mut cmd, "gitee", &mut std::io::stdout());
    Ok(())
}

fn detect_shell() -> Result<Shell> {
    let shell = std::env::var("SHELL").unwrap_or_default();
    let name = shell.rsplit('/').next().unwrap_or("bash");
    Shell::from_str(name).map_err(|_| {
        GiteeError::Usage(format!(
            "could not detect shell from $SHELL='{shell}'; pass it explicitly (bash|zsh|fish|...)"
        ))
    })
}
