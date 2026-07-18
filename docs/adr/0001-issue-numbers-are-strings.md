# ADR-0001: Issue numbers are strings

## Status

Accepted

## Date

2026-07-18

## Context

An architecture review (2026-07-18) suggested unifying `Issue.number` with `PullRequest.number` as `i64` for consistency across commands.

Gitee issue numbers are **alphanumeric identifiers** (e.g. `I6D3AV`), not sequential integers. They appear in issue URLs and API path segments such as `/repos/{owner}/{repo}/issues/{number}`. Pull request numbers are ordinary integers (`i64`) in both the API and URLs.

The codebase already reflects this split:

- `Issue.number: String` in `src/models.rs`
- `PullRequest.number: i64` in `src/models.rs`
- `Issues::get`, `set_state`, `comment`, and `link` take `number: &str` in `src/api/issues.rs`
- Issue CLI handlers pass `&number` from clap (`src/cmd/issue.rs`)

## Decision

**Keep `Issue.number` as `String`.** Do not unify issue and PR number types.

Issue-facing CLI arguments and `api::issues` methods continue to accept `&str`. PR-facing APIs keep `i64`.

## Consequences

- Issue commands accept alphanumeric identifiers without parsing or coercion.
- Type signatures document the Gitee API distinction at compile time.
- Future reviews should not re-propose unifying issue and PR number types unless Gitee changes its issue identifier format.
- Cross-linking or shared helpers must not assume a numeric issue id.
