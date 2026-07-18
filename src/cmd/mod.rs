use std::cell::OnceCell;
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
pub mod release;
pub mod repo;

pub struct Ctx {
    pub client: Client,
    pub out: Output,
    repo_arg: Option<String>,
    remote_arg: Option<String>,
    repo: OnceCell<Repo>,
}

impl Ctx {
    pub fn repo(&self) -> Result<&Repo> {
        if let Some(r) = self.repo.get() {
            return Ok(r);
        }
        let r = Repo::resolve(self.repo_arg.as_deref(), self.remote_arg.as_deref())?;
        let _ = self.repo.set(r);
        Ok(self.repo.get().expect("repo just initialized"))
    }

    pub fn repo_arg(&self) -> Option<&str> {
        self.repo_arg.as_deref()
    }
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
        Command::Release(c) => {
            let ctx = build(&cli)?;
            release::execute(&ctx, c.clone())
        }
        Command::Repo(c) => {
            let ctx = build(&cli)?;
            repo::execute(&ctx, c.clone())
        }
        Command::Completions { shell } => completions(shell.clone()),
    }
}

/// HTTP client with no repo resolution.
fn core(cli: &Cli) -> Result<Client> {
    let token = Config::token(&cli.host)?;
    let mut client = Client::for_host(&cli.host, token);
    client.set_debug(cli.debug);
    Ok(client)
}

fn build(cli: &Cli) -> Result<Ctx> {
    Ok(Ctx {
        client: core(cli)?,
        out: Output {
            json: cli.json.clone(),
        },
        repo_arg: cli.repo.clone(),
        remote_arg: cli.remote.clone(),
        repo: OnceCell::new(),
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
    // Generate into a buffer first: clap_complete panics on write errors,
    // and a closed pipe (`gitee completions bash | head`) must exit quietly.
    let mut cmd: clap::Command = crate::cli::Cli::command();
    let mut buf = Vec::new();
    generate(shell, &mut cmd, "gitee", &mut buf);
    use std::io::Write;
    let mut out = std::io::stdout().lock();
    match out.write_all(&buf).and_then(|()| out.flush()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => Ok(()),
        Err(e) => Err(e.into()),
    }
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
