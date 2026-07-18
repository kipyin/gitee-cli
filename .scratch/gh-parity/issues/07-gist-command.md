# gist command group

Status: ready-for-agent

## Context
`gh gist` parity; Gitee 代码片段 has full v5 CRUD.

## Scope
- `gist list [--limit]`, `gist view <id> [--json] [--raw]`, `gist create <files...|->`
  (--desc, --public), `gist edit <id> <file>`, `gist delete <id>` (confirm or --yes).
- API: GET/POST /gists, GET/PATCH/DELETE /gists/{id}.
- create reads stdin when file is `-` (filename from --filename flag, gh-style).

## Acceptance
- Round-trip: create → view → edit → delete via CLI only.

## Non-goals
- Gist comments/star/fork subcommands (API exists; later ticket if wanted).

## Implementation notes (2026-07-18)

Implemented: list/view(--raw)/create(files|-stdin with --filename)/edit/delete(--yes).

- Gist create/edit form encoding is Rails-style nested fields `files[<name>][content]` (verified live, 201); description is API-required (defaults to first filename).
- Live round-trip verified (create → view --raw → edit → delete) on merged build.
