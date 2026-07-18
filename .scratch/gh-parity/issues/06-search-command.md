# search command group

Status: ready-for-agent

## Context
`gh search` parity; Gitee v5 exposes /search/repositories, /search/issues,
/search/code, /search/commits, /search/users.

## Scope
- `gitee search repos|issues|code|commits|users <query>` with --limit, --json,
  and --sort/--order where the endpoint supports it.
- Code search requires auth — surface a clear error if the token lacks it.
- PR search: verify whether /search/issues can filter type=pr or a dedicated
  endpoint exists; if yes add `search prs`, otherwise omit (note in code comment).

## Acceptance
- `gitee search repos gitee --limit 5` prints a table; --json works.
- Empty result prints nothing, exit 0.

## Non-goals
- Query DSL beyond what v5 accepts in q.

## Implementation notes (2026-07-18)

Implemented: `search repos|issues|users` with --limit/--json/--sort/--order + endpoint-specific filters; issues repo filter comes from the global --repo (no git-remote resolution for search).

- `search code` / `search commits` DROPPED: no /search/code or /search/commits in the v5 swagger (verified 2026-07-18).
- `search prs` OMITTED: /search/issues has no PR type filter (verified 2026-07-18); noted in src/api/search.rs.
- Platform quirk: /search/repositories live index returns [] for every query tried (incl. popular terms) — the command is correct; empty prints nothing, exit 0. issues/users search return results normally.
