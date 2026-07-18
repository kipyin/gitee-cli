# milestone command group

Status: ready-for-agent

## Context
gh has NO milestone command — this is beyond-parity. Gitee v5 has full CRUD;
tickets 03/04/05 want to reference milestones.

## Scope
- `milestone list [--state] [--limit]`, `milestone view <number>`,
  `milestone create --title <t> [--due-on YYYY-MM-DD] [--description] [--state open|closed]`,
  `milestone edit <number>` (same fields).
- API: GET/POST /repos/{owner}/{repo}/milestones,
  PATCH /repos/{owner}/{repo}/milestones/{number}.
- Accept milestone by title in 03/04/05 by resolving via list (exact match).

## Acceptance
- Round-trip; `issue create --milestone v1.0` resolves by title.

## Non-goals
- Milestone progress burndown.

## Implementation notes (2026-07-18)

Implemented: list(--state)/view/create/edit.

- `due_on` is REQUIRED by the API on create (verified live: POST without it → 400) → `--due-on YYYY-MM-DD` is a required flag.
- PATCH requires title AND due_on on every call → edit does GET-then-merge (sends current values for unset flags).
- Title→number resolution for issue/pr --milestone already existed (resolve_milestone); smoke-verified `issue create --milestone v1.0` attaches the milestone.
