# pr status / issue status

Status: ready-for-agent

## Context
`gh pr status` / `gh issue status` parity: repo-scoped "what's relevant to me".

## Scope
- `pr status`: sections — created by me (open), assigned to me, awaiting my
  review/test (assignees/testers contain me). Current user via GET /user (cache
  per invocation).
- `issue status`: sections — created by me, assigned to me (open only).
- Use list endpoints with server filters where available; client-side filter
  otherwise. Respect --limit, --json.

## Acceptance
- On a repo with mixed ownership, sections match the web UI's 我的 lists.

## Non-goals
- Cross-repo view — that's ticket 13.
