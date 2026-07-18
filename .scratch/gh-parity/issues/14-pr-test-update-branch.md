# pr test / pr update-branch (Gitee-specific)

Status: ready-for-agent

## Context
Gitee PRs have a 审查/测试 dual gate: `pr approve` already covers 审查通过;
测试通过 is its own endpoint. Beyond-parity vs gh.

## Scope
- `pr test <number> [--force]` — POST /repos/{owner}/{repo}/pulls/{number}/test.
- `pr update-branch <number>` — PUT /repos/{owner}/{repo}/pulls/{number}/update-branch
  (verify exact method/path in swagger before coding).
- Both respect --json; print a one-line confirmation otherwise.

## Acceptance
- On a gated test PR: `pr approve` + `pr test` together make it mergeable;
  `pr update-branch` refreshes from base (visible as new head sha in `pr view`).

## Non-goals
- Auto-merge polling.
