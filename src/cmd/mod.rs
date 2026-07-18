use std::cell::OnceCell;
use std::str::FromStr;

use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::api::client::Client;
use crate::cli::{Cli, Command};
use crate::config::Config;
use crate::error::{GiteeError, Result};
use crate::models::UserBasic;
use crate::out::Output;
use crate::repo::Repo;

pub mod api;
pub mod alias;
pub mod extension;
pub mod auth;
pub mod browse;
pub mod collaborator;
pub mod config_cmd;
pub mod gist;
pub mod issue;
pub mod label;
pub mod org;
pub mod pr;
pub mod milestone;
pub mod release;
pub mod search;
pub mod ssh_key;
pub mod status;
pub mod repo;
pub mod webhook;

pub struct Ctx {
    pub client: Client,
    pub out: Output,
    pub host: String,
    repo_arg: Option<String>,
    remote_arg: Option<String>,
    repo: OnceCell<Repo>,
    me: OnceCell<UserBasic>,
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

    /// The authenticated user, fetched once per invocation and cached.
    pub fn me(&self) -> Result<&UserBasic> {
        if let Some(u) = self.me.get() {
            return Ok(u);
        }
        let u = self.client.users().me()?;
        let _ = self.me.set(u);
        Ok(self.me.get().expect("user just initialized"))
    }
}

pub fn run(cli: Cli) -> Result<()> {
    match &cli.cmd {
        Command::Auth(c) => auth::execute(c.clone(), &cli.host),
        Command::Config(c) => {
            let ctx = build_inner(&cli, false)?;
            config_cmd::execute(&ctx, c.clone())
        }
        Command::Alias(c) => {
            let ctx = build_inner(&cli, false)?;
            alias::execute(&ctx, c.clone())
        }
        Command::Browse => {
            let ctx = build_inner(&cli, false)?;
            browse::execute(&ctx)
        }
        Command::Api(a) => {
            let client = core(&cli)?;
            api::execute(&client, a.clone())
        }
        Command::Gist(c) => {
            let ctx = build(&cli)?;
            gist::execute(&ctx, c.clone())
        }
        Command::Pr(c) => {
            let require_auth = !matches!(c, crate::cli::PrCmd::View { web: true, .. });
            let ctx = build_inner(&cli, require_auth)?;
            pr::execute(&ctx, c.clone())
        }
        Command::Issue(c) => {
            let require_auth = !matches!(c, crate::cli::IssueCmd::View { web: true, .. });
            let ctx = build_inner(&cli, require_auth)?;
            issue::execute(&ctx, c.clone())
        }
        Command::Search(c) => {
            let ctx = build(&cli)?;
            search::execute(&ctx, c.clone())
        }
        Command::Status { limit } => {
            let ctx = build(&cli)?;
            status::execute(&ctx, limit.clone())
        }
        Command::Release(c) => {
            let require_auth = !matches!(c, crate::cli::ReleaseCmd::View { web: true, .. });
            let ctx = build_inner(&cli, require_auth)?;
            release::execute(&ctx, c.clone())
        }
        Command::Label(c) => {
            let ctx = build(&cli)?;
            label::execute(&ctx, c.clone())
        }
        Command::Repo(c) => {
            let require_auth = !matches!(c, crate::cli::RepoCmd::View { web: true, .. });
            let ctx = build_inner(&cli, require_auth)?;
            repo::execute(&ctx, c.clone())
        }
        Command::Milestone(c) => {
            let ctx = build(&cli)?;
            milestone::execute(&ctx, c.clone())
        }
        Command::Org(c) => {
            let ctx = build(&cli)?;
            org::execute(&ctx, c.clone())
        }
        Command::SshKey(c) => {
            let ctx = build(&cli)?;
            ssh_key::execute(&ctx, c.clone())
        }
        Command::Collaborator(c) => {
            let ctx = build(&cli)?;
            collaborator::execute(&ctx, c.clone())
        }
        Command::Webhook(c) => {
            let ctx = build(&cli)?;
            webhook::execute(&ctx, c.clone())
        }
        Command::Extension(c) => {
            let ctx = build_inner(&cli, false)?;
            extension::execute(&ctx, c.clone())
        }
        Command::External(args) => {
            let Some(name) = args.first().and_then(|s| s.to_str()) else {
                return Err(crate::error::GiteeError::Usage(
                    "extension command name required".into(),
                ));
            };
            crate::extension::exec(name, &args[1..], &cli.host)
        }
        Command::Completions { shell } => completions(shell.clone()),
    }
}

/// HTTP client with no repo resolution.
fn core(cli: &Cli) -> Result<Client> {
    core_inner(cli, true)
}

fn core_inner(cli: &Cli, require_auth: bool) -> Result<Client> {
    let token = match Config::token(&cli.host) {
        Ok(t) => t,
        Err(GiteeError::NotLoggedIn) if !require_auth => String::new(),
        Err(e) => return Err(e),
    };
    let mut client = Client::for_host(&cli.host, token);
    client.set_debug(cli.debug);
    Ok(client)
}

/// Guard a destructive operation behind explicit confirmation. `--yes` skips
/// the prompt; an interactive terminal must type `yes`; anything else (piped
/// stdin, no TTY) is a usage error, so scripts can't delete by accident.
pub fn confirm(action: &str, yes: bool) -> Result<()> {
    use std::io::IsTerminal;
    if yes {
        return Ok(());
    }
    if !std::io::stdin().is_terminal() {
        return Err(GiteeError::Usage(format!(
            "{action}: pass --yes to confirm (stdin is not a terminal)"
        )));
    }
    eprintln!("{action}? Type 'yes' to confirm: ");
    let mut line = String::new();
    std::io::stdin().read_line(&mut line).ok();
    if line.trim() == "yes" {
        Ok(())
    } else {
        Err(GiteeError::Usage("aborted".into()))
    }
}

/// Flatten repeatable, comma-splittable flag values (e.g. `--label a,b --label c`)
/// into one comma-joined string; `None` when nothing was given.
pub(crate) fn join_flags(values: &[String]) -> Option<String> {
    let parts: Vec<&str> = values
        .iter()
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    (!parts.is_empty()).then(|| parts.join(","))
}

/// Resolve a `--milestone` value: bare integers pass through; anything else is
/// matched against the repo's milestone titles (one extra API call).
pub(crate) fn resolve_milestone(ctx: &Ctx, repo: &Repo, id_or_title: &str) -> Result<i64> {
    if let Ok(n) = id_or_title.trim().parse::<i64>() {
        return Ok(n);
    }
    let list = ctx
        .client
        .repos()
        .list_milestones(&repo.owner, &repo.name)?;
    crate::models::Milestone::resolve(&list, id_or_title).ok_or_else(|| {
        let known = list
            .iter()
            .map(|m| m.title.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        GiteeError::Usage(format!(
            "no milestone titled '{id_or_title}' (available: {known})"
        ))
    })
}

/// Optional variant of [`resolve_milestone`]: `None` stays `None`.
pub(crate) fn resolve_milestone_opt(
    ctx: &Ctx,
    repo: &Repo,
    id_or_title: Option<&str>,
) -> Result<Option<i64>> {
    match id_or_title {
        Some(m) => Ok(Some(resolve_milestone(ctx, repo, m)?)),
        None => Ok(None),
    }
}

fn build(cli: &Cli) -> Result<Ctx> {
    build_inner(cli, true)
}

fn build_inner(cli: &Cli, require_auth: bool) -> Result<Ctx> {
    Ok(Ctx {
        client: core_inner(cli, require_auth)?,
        out: Output {
            json: cli.json.clone(),
            jq: cli.jq.clone(),
        },
        host: cli.host.clone(),
        repo_arg: cli.repo.clone(),
        remote_arg: cli.remote.clone(),
        repo: OnceCell::new(),
        me: OnceCell::new(),
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

#[cfg(test)]
mod auth_free_tests {
    use super::*;
    use crate::cli::Cli;

    #[test]
    fn builds_without_auth_for_local_commands() {
        use clap::Parser;

        let dir = std::env::temp_dir().join(format!(
            "gitee-cli-authfree-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("config.json"), "{}
").unwrap();
        std::env::set_var("GITEE_CONFIG_DIR", &dir);
        for args in ["gitee config list", "gitee alias list", "gitee browse"] {
            let cli = Cli::try_parse_from(args.split_whitespace()).expect("parse");
            build_inner(&cli, false).expect("build without auth");
        }
        std::env::remove_var("GITEE_CONFIG_DIR");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod flag_tests {
    #[test]
    fn join_flags_flattens_repeatable_and_comma_split() {
        let v = vec!["a,b".to_string(), " c ".to_string()];
        assert_eq!(super::join_flags(&v).as_deref(), Some("a,b,c"));
    }

    #[test]
    fn join_flags_empty_is_none() {
        assert_eq!(super::join_flags(&[]), None);
        assert_eq!(super::join_flags(&["  ".to_string()]), None);
    }
}
