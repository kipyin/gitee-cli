# auth setup-git / auth switch / git-credential

Status: ready-for-agent

## Context
`gh auth setup-git` configures git to use the CLI as a credential helper;
`gh auth switch` juggles multiple accounts per host.

## Scope
- `auth git-credential get|store|erase` implementing the git credential protocol
  (token as password, username = gitee username or oauth2).
- `auth setup-git`: writes credential.https://<host>.helper pointing at the
  current exe (use std::env::current_exe, gh-style `!path auth git-credential`).
- `auth switch --user <name>`: token store becomes per-user-per-host;
  migrate existing single-token config transparently (treat as default user).
- `auth status` shows all stored users, marking active.

## Acceptance
- `git clone https://gitee.com/...` on a private repo works with zero prompts
  after setup-git; switch changes which account `auth token` returns.

## Non-goals
- OAuth device flow (Gitee PAT model stays).
