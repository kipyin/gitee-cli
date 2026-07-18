# --jq: arbitrary jq expression on --json output

Status: ready-for-agent

## Context
gh pairs `--json` with `--jq` for arbitrary filtering; gitee-cli currently only
has field projection (`--json number,title`). Power users lean on --jq as much as
on `gh api` (ticket 01).

## Scope
- Global flag `--jq <expr>`; requires --json (error otherwise, mirroring gh).
- Evaluation order: fetch → field projection (if fields given) → jq expression.
- Implement with a pure-Rust jq engine (e.g. the jaq crate); avoid linking C libjq.
- Output stays raw-JSON (strings not auto-unquoted unless expression yields scalar —
  match gh behavior as closely as jaq allows; document divergence in --help if any).

## Acceptance
- `gitee pr list --json --jq '.[0].title'` prints the first PR title.
- `gitee issue list --json number,title --jq 'map(.number)'` works post-projection.
- Invalid expression: exit non-zero with a clear message.
- --jq without --json: usage error.

## Non-goals
- Streaming mode, YAML output.
