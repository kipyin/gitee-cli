# label command group

Status: ready-for-agent

## Context
`gh label` parity; labels are needed by tickets 03/05 flag validation anyway.

## Scope
- `label list [--limit]`, `label create <name> --color <hex> [--description]`,
  `label edit <name> [--name] [--color]`, `label delete <name>` (confirm or --yes).
- API: GET/POST /repos/{owner}/{repo}/labels, PATCH/DELETE .../labels/{name}.
- Color: strip leading '#', validate 6-hex.

## Acceptance
- Round-trip create/edit/delete on a test repo.

## Non-goals
- `gh label clone` cross-repo copy (nice-to-have, note as follow-up in file).

## Implementation notes (2026-07-18)

Implemented: list/create/edit/delete(--yes); color normalized (strip '#', validate 6-hex, lowercase).

- `--description` DROPPED: v5 POST /labels has no description param (verified 2026-07-18).
- GET /labels has no page/per_page params → --limit is client-side truncation.
- Follow-up noted: `gh label clone` cross-repo copy not implemented (ticket non-goal).
