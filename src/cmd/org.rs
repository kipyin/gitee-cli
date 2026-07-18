use super::Ctx;
use crate::cli::OrgCmd;
use crate::error::Result;
use crate::out;

pub fn execute(ctx: &Ctx, cmd: OrgCmd) -> Result<()> {
    match cmd {
        OrgCmd::List { limit } => {
            let items = ctx.client.users().orgs(limit.limit)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::org_table(w, &items))?;
        }
    }
    Ok(())
}
