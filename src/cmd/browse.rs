use super::Ctx;
use crate::error::Result;
use crate::web;

pub fn execute(ctx: &Ctx) -> Result<()> {
    let repo = ctx.repo()?;
    let url = web::repo_url(&ctx.host, repo);
    web::open_or_print(&url)
}
