# issue edit + create parity fields

Status: ready-for-agent

## Context
No way to modify an issue after creation; create lacks Gitee-specific fields.

## Scope
- `gitee issue edit <ident>`: --title --body --assignee --label --milestone
  --priority --security-hole (bool).
- Extend `issue create` with: --milestone --priority --security-hole.
- API: PATCH /repos/{owner}/issues/{ident} and the existing POST. Verify priority
  vocabulary (严重/主要/次要/不重要) and security_hole flag naming against live responses.

## Acceptance
- Edit round-trips via `gitee issue view`.
- Create with --priority/--security-hole reflected in view --json.

## Non-goals
- issue transfer/pin/lock/delete — no Gitee API (see spec.md blocked list).
