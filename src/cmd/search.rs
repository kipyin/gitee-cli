use super::Ctx;
use crate::api::search::{SearchIssuesFilter, SearchReposFilter, SearchUsersFilter};
use crate::cli::SearchCmd;
use crate::error::Result;
use crate::out;

pub fn execute(ctx: &Ctx, cmd: SearchCmd) -> Result<()> {
    match cmd {
        SearchCmd::Repos {
            query,
            owner,
            language,
            fork,
            sort,
            order,
            limit,
        } => {
            let filter = SearchReposFilter {
                q: &query,
                owner: owner.as_deref(),
                language: language.as_deref(),
                fork,
                sort: sort.as_deref(),
                order: order.as_deref(),
                limit: limit.limit,
            };
            let items = ctx.client.search().repos(&filter)?;
            if items.is_empty() && ctx.out.json.is_none() {
                return Ok(());
            }
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::repo_table(w, &items))?;
        }
        SearchCmd::Issues {
            query,
            state,
            author,
            assignee,
            label,
            language,
            sort,
            order,
            limit,
        } => {
            let filter = SearchIssuesFilter {
                q: &query,
                repo: ctx.repo_arg(),
                language: language.as_deref(),
                label: label.as_deref(),
                state: state.as_deref(),
                author: author.as_deref(),
                assignee: assignee.as_deref(),
                sort: sort.as_deref(),
                order: order.as_deref(),
                limit: limit.limit,
            };
            let items = ctx.client.search().issues(&filter)?;
            if items.is_empty() && ctx.out.json.is_none() {
                return Ok(());
            }
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::issue_table(w, &items))?;
        }
        SearchCmd::Users {
            query,
            sort,
            order,
            limit,
        } => {
            let filter = SearchUsersFilter {
                q: &query,
                sort: sort.as_deref(),
                order: order.as_deref(),
                limit: limit.limit,
            };
            let items = ctx.client.search().users(&filter)?;
            if items.is_empty() && ctx.out.json.is_none() {
                return Ok(());
            }
            let mut out = std::io::stdout().lock();
            ctx.out
                .render(&mut out, &items, |w| out::user_table(w, &items))?;
        }
    }
    Ok(())
}
