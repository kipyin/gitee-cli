# config command + alias system

Status: ready-for-agent

## Context
`gh config`/`gh alias` parity. config.rs already persists per-host tokens;
expose it and add aliases.

## Scope
- `config list|get|set <key> [value]` for: host default, remote default, editor
  (used by future interactive flows). Keys stored in the existing config file.
- `alias set <name> <expansion>`, `alias list`, `alias delete <name>` — stored in config.
- Expansion: first-token replacement before clap parsing; expansion string split
  with shell-style quoting (shell-words crate); `gitee co 12` → alias co = "pr checkout"
  runs `pr checkout 12`. Recursive alias to self: error.

## Acceptance
- Round-trip config and alias; alias expansion visible with --debug.

## Non-goals
- Per-repo config overrides.
