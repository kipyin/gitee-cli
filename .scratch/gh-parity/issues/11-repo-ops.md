# repo create / edit / rename / delete

Status: ready-for-agent

## Context
`gh repo` parity for repo lifecycle.

## Scope
- `repo create <name> [--org <org>] [--private] [--description] [--homepage]
  [--gitignore <t>] [--license <spdx>]` — POST /user/repos or /orgs/{org}/repos.
- `repo edit [--description] [--homepage] [--private|--public] [--default-branch]`
  — PATCH /repos/{owner}/{repo}.
- `repo rename <new-path>` — PATCH path field (verify path vs name semantics).
- `repo delete` (--yes to skip confirm) — DELETE /repos/{owner}/{repo}.
- Fork sync: verify whether a synchronize endpoint exists; if yes add `repo sync`,
  else omit and note in file.

## Acceptance
- Round-trip create → edit → rename → delete on a scratch repo.

## Non-goals
- repo archive — no Gitee API (spec.md blocked list).

## Implementation notes (2026-07-18)

Implemented: create(--org/--private/--description/--homepage/--gitignore/--license)/edit(--description/--homepage/--private|--public/--default-branch)/rename/delete(--yes).

- `repo sync` OMITTED: no fork-synchronize endpoint in the v5 swagger (verified 2026-07-18); noted on RepoCmd.
- PATCH /repos/{o}/{r} requires `name` on every call → edit/rename always send the current name; rename sets `path` (URL slug).
- Platform quirk: after rename, GET on the OLD slug still resolves (alias → canonical), unlike GitHub (documented in rename()).
- New repos are PRIVATE by default (Gitee platform default); no --auto-init flag (not in ticket scope) — release create needs an initialized branch.
