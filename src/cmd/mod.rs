use crate::api::client::Client;
use crate::cli::{Cli, Command};
use crate::config::Config;
use crate::error::Result;
use crate::out::Output;
use crate::repo::Repo;
pub mod auth;
pub mod issue;
pub mod pr;
pub struct Ctx {
    pub client: Client,
    pub repo: Repo,
    pub out: Output,
}
pub fn run(cli: Cli) -> Result<()> {
    match &cli.cmd {
        Command::Auth(c) => auth::execute(c.clone()),
        Command::Pr(c) => {
            let ctx = build(&cli)?;
            pr::execute(&ctx, c.clone())
        }
        Command::Issue(c) => {
            let ctx = build(&cli)?;
            issue::execute(&ctx, c.clone())
        }
    }
}
fn build(cli: &Cli) -> Result<Ctx> {
    let token = Config::token("gitee.com")?;
    let client = Client::new("https://gitee.com/api/v5".into(), token);
    let repo = Repo::resolve(cli.repo.as_deref(), cli.remote.as_deref())?;
    Ok(Ctx {
        client,
        repo,
        out: Output { json: cli.json },
    })
}
