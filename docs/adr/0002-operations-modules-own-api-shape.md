# ADR-0002: Operations modules own API shape

## Status

Accepted

## Date

2026-07-18

## Context

Before the 2026-07-18 architecture review, Gitee API shape was scattered across `cmd` handlers: path templates, form field names, JSON-vs-form encoding, and host-specific quirks lived beside argument parsing and printing. Handlers were hard to test in isolation and duplicated knowledge when commands grew.

Gitee v5 is inconsistent. Examples now centralized in operations modules:

- **Asymmetric issue paths** — create/state-change use `/repos/{owner}/issues/…` with `repo` in the body or form, not always `/repos/{owner}/{repo}/issues/…` (`src/api/issues.rs`).
- **Title echo on issue state change** — PATCH must include the current `title` or Gitee blanks it (`Issues::set_state`).
- **Mixed encodings** — PR state uses form PATCH; issue updates require JSON (`Client::patch` vs `patch_json`).
- **Release form quirks** — `prerelease` always sent as `"true"`/`"false"` strings (`src/api/releases.rs`).

## Decision

**All API shape knowledge lives in operations modules** — `src/api/pulls.rs`, `src/api/issues.rs`, `src/api/releases.rs`, `src/api/repos.rs` — behind typed methods (`Pulls`, `Issues`, `Releases`, `Repos`).

Layering:

| Layer | Location | Responsibility |
|-------|----------|----------------|
| Models | `src/models.rs` | Response types; state vocabulary (`PrState`, `IssueState`, `MergeMethod`) |
| Operations | `src/api/{pulls,issues,releases,repos}.rs` | Paths, query keys, form/JSON bodies, Gitee quirks |
| Transport | `src/api/client.rs` | HTTP verbs, auth header, paging, error mapping |
| Cmd | `src/cmd/*.rs` | Parse args → call ops → render via `out` |
| Output | `src/out.rs` | Tables, colors, `--json` projection |

**Cmd handlers must not format API paths or field names.** New commands follow: parse → `ctx.client.<ops>(repo).method(…)` → `ctx.out.render(…)`.

## Consequences

- Integration tests cross the operations seam (mock HTTP via mockito) instead of exercising handlers with embedded path strings.
- Adding a command is mostly parse + one ops call + one printer; API changes touch one operations file.
- `Client` stays verb-level; it does not grow domain-specific methods or path builders.
- State enums remain in `models`, not in operations or cmd — operations accept/return typed states, not raw strings, where the API allows.
- Duplicating path templates or form keys outside `src/api/{pulls,issues,releases,repos}.rs` is a review failure.
