# 28 — MCP server (wontfix)

Status: wontfix
Resolved: 2026-07-21

## Decision

Will not build an MCP server into or alongside `gitee-cli`.

## Why

Gitee already ships an official MCP server written in Go —
`gitee.com/oschina/mcp-gitee` — active and maintained. A third-party Rust
implementation (`zymaio/gitee-rs`) also exists. Building our own (option C:
feature-gated `gitee mcp` subcommand; or option D: installable extension)
would duplicate an already-solved surface and add ongoing maintenance against
API drift, for marginal value.

The distinction between an extension and an MCP server is real but not in our
favor here:

- **Extension** = subprocess, argv + stdout/stderr + exit code. Useful for
  humans and shell scripts.
- **MCP server** = schema'd JSON-RPC over stdio/SSE. Useful for agent
  runtimes (Claude Code, Cursor, opencode) that want native tool integration.

Agent runtimes that want native Gitee integration should use the official
`mcp-gitee`. Agent orchestration via shell-out to our CLI is already
well-supported by ticket #29's scripting ergonomics (stable exit codes,
structured `--json` errors, idempotent mutating verbs, `--preview`).

## What we did instead

- README gains an "AI agent 集成" / "Scripting gitee-cli for agents" section
  pointing at the official `mcp-gitee` for native MCP integration and at
  `docs/scripting.md` for shell-out patterns.
- Ticket #29 covers the agent-friendly CLI surface.
- The extension system (tickets #24/#27) can still install a future MCP
  server as an extension — `gitee extension install owner/gitee-mcp
  --build cargo` — no code change needed.

## Reopen if

- The official `mcp-gitee` stagnates or is abandoned.
- A clear gap opens that only a Rust implementation sharing our API client
  can fill (e.g. the official server lacks a tool we need and won't accept
  contributions).