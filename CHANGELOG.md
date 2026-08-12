# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
(0.x: breaking changes may land in minor bumps).

## [0.2.2] — 2026-08-12

### Fixed

- Document and enforce Gitee v5 **issue state write** limits: only
  `open` | `progressing` | `closed` are accepted by the live API.
  `--state rejected` is no longer offered (it always returned HTTP 400
  `state does not have a valid value`). `closed` always maps to board
  state 已完成 — there is no API close-reason for 拒绝 / wontfix; use a
  repo label. See `gitee issue edit --help` and README `gitee api` notes.

## [0.2.1] — 2026-08-05

### Added

- `pr create --draft` and `pr ready <n>` / `pr ready <n> --undo` (gh-style draft
  PRs). List/view show open drafts as `draft`. Ready/undo are idempotent.

## [0.2.0] — 2026-07-24

### Breaking

- `issue comment` / `pr comment` are now subcommand groups. Create a comment
  with the new verb:

  - `gitee issue comment create <ident> -m "…"` (was `gitee issue comment <ident> -m "…"`)
  - `gitee pr comment create <n> -m "…"` (was `gitee pr comment <n> -m "…"`)

  The old flat form is removed. Siblings under `comment`: `list`, `edit`,
  `delete` (`--last` targets the current user's most-recent comment;
  `delete` on a missing id is idempotent exit `0`).

### Added

- `issue comment` / `pr comment` `list`, `edit`, and `delete`, including
  `--last` to target the current user's most-recent comment.
- `pr comment create` optional `--path` / `--position` / `--commit-id` for
  positional (diff-line) comments; create returns the richer `PrComment`
  model so positional fields survive `--json`.
- `issue label` / `pr label` `{add,remove,list}` for non-destructive label
  membership on an item (distinct from repo-level `gitee label`).
- `pr assignee` / `pr tester` `{add,remove,list}` for non-destructive
  reviewer (审查人) and tester (测试人) membership.
- `pr commits <n>` to list commits on a pull request.
- `pr view <n> --merged` for a scriptable merge-state exit code (0 merged /
  1 not); default `pr view` output also shows merged state.
