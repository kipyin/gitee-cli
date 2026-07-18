# gitee api — raw API passthrough

Status: ready-for-agent

## Context
`gh api` is the single highest-leverage gap: it unblocks every feature not yet
covered by a dedicated subcommand.

## Scope
- New subcommand `gitee api <endpoint>` (e.g. `gitee api user`, `gitee api repos/oschina/gitfy/releases`).
- Flags: `-X/--method` (default GET; POST when fields given), `-F/--field k=v`
  (form field; repeatable), `-f/--raw-field k=v` (raw string), `-H/--header k:v`,
  `--input <file|->` (raw body), `--paginate` (walk page/per_page until empty page).
- Endpoint may be absolute path (`/user`) or relative (`user`); leading `/api/v5/`
  stripped or prepended consistently.
- Respects global --host, --debug; prints response body to stdout.

## Acceptance
- `gitee api user` prints the authenticated user JSON.
- `gitee api -X POST gists -F 'files[x.rs][content]=fn main(){}' -F description=t` creates a gist.
- Non-2xx: exit non-zero, error body on stderr.
- `--paginate` merges array pages into one JSON array.

## Non-goals
- Response caching, GraphQL, --template/--filter beyond ticket 02.
