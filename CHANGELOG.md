# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
(0.x: breaking changes may land in minor bumps).

## [0.2.0] — unreleased

### Breaking

- `issue comment` / `pr comment` are now subcommand groups. Create a comment
  with the new verb:

  - `gitee issue comment create <ident> -m "…"` (was `gitee issue comment <ident> -m "…"`)
  - `gitee pr comment create <n> -m "…"` (was `gitee pr comment <n> -m "…"`)

  The old flat form is removed. Siblings under `comment`: `list`, `edit`,
  `delete` (`--last` targets the current user's most-recent comment;
  `delete` on a missing id is idempotent exit `0`).
