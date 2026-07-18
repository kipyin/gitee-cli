# org list

Status: ready-for-agent

## Context
`gh org list` parity; trivial.

## Scope
- `org list [--limit]` — GET /user/orgs; table of login/name/role; --json.

## Acceptance
- Lists the authed user's orgs.

## Non-goals
- Org member management (v5 has org member endpoints; separate ticket if wanted).
