# ssh-key command group

Status: ready-for-agent

## Context
`gh ssh-key` parity; v5 has /user/keys.

## Scope
- `ssh-key list`, `ssh-key add <pubkey-file> [--title]`, `ssh-key delete <id>`
  (confirm or --yes).
- API: GET /user/keys, POST /user/keys (key, title), DELETE /user/keys/{id}.
- add defaults title to the key comment, else hostname+date.

## Acceptance
- Round-trip with a throwaway keypair; ssh clone works after add.

## Non-goals
- GPG keys (no Gitee API).
