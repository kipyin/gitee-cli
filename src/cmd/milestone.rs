
use super::Ctx;
use crate::api::milestones::{CreateMilestone, EditMilestone, MilestoneFilter};
use crate::cli::MilestoneCmd;
use crate::error::Result;
use crate::out;

pub fn execute(ctx: &Ctx, cmd: MilestoneCmd) -> Result<()> {
    match cmd {
        MilestoneCmd::List { list } => {
            let repo = ctx.repo()?;
            let filter = MilestoneFilter {
                state: list.state.as_deref(),
                limit: list.limit,
            };
            let items = ctx.client.milestones(repo).list(&filter)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::milestone_table(w, &items))?;
        }
        MilestoneCmd::View { number } => {
            let repo = ctx.repo()?;
            let milestone = ctx.client.milestones(repo).get(number)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &milestone, |w| out::one_milestone(w, &milestone))?;
        }
        MilestoneCmd::Create {
            title,
            due_on,
            description,
            state,
        } => {
            let repo = ctx.repo()?;
            let req = CreateMilestone {
                title: &title,
                due_on: &due_on,
                description: description.as_deref(),
                state: state.as_deref(),
            };
            let milestone = ctx.client.milestones(repo).create(&req)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &milestone, |w| out::one_milestone(w, &milestone))?;
        }
        MilestoneCmd::Edit {
            number,
            title,
            due_on,
            description,
            state,
        } => {
            let repo = ctx.repo()?;
            let req = EditMilestone {
                title: title.as_deref(),
                due_on: due_on.as_deref(),
                description: description.as_deref(),
                state: state.as_deref(),
            };
            let milestone = ctx.client.milestones(repo).edit(number, &req)?;
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &milestone, |w| out::one_milestone(w, &milestone))?;
        }
    }
    Ok(())
}
