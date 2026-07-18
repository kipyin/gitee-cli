use super::Ctx;
use crate::api::users::UserIssueFilter;
use crate::cli::LimitArgs;
use crate::error::Result;
use crate::out;

pub fn execute(ctx: &Ctx, limit: LimitArgs) -> Result<()> {
    let assigned = ctx.client.users().issues(&UserIssueFilter {
        filter: "assigned",
        state: Some("open"),
        limit: limit.limit,
    })?;
    let created = ctx.client.users().issues(&UserIssueFilter {
        filter: "created",
        state: Some("open"),
        limit: limit.limit,
    })?;

    let data = out::Dashboard { assigned, created };
    let mut out = std::io::stdout().lock();
    ctx.out
        .render(&mut out, &data, |w| out::dashboard(w, &data))?;
    Ok(())
}
