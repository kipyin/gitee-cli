# gh-parity spec

Close the functional gap between gitee-cli and GitHub CLI (`gh`) wherever the
Gitee OpenAPI v5 allows it, and add Gitee-specific features gh lacks.

Baseline (src/cli.rs @ 2026-07-18): auth(login/status/token/logout),
repo(view/list/clone/fork), issue(list/view/create/close/reopen/link/comment),
pr(list/view/diff/checkout/create/merge/comment/approve/close/reopen/link),
release(list/view/create/upload), completions.
Globals: --repo --remote --host --json(field projection) --debug.

## Ticket order = suggested priority

- 01–02  power-user escape hatches (highest leverage)
- 03–05  PR/issue CRUD completeness
- 06–09  new command groups (search, gist, label, milestone)
- 10–11  release & repo CRUD completeness
- 12–14  status views + Gitee-specific PR ops
- 15–22  UX layer + small command groups
- 23–25  design decisions + docs

## Platform-blocked — do NOT schedule

- `gh workflow` / `gh run` / `gh pr checks` / secrets / variables:
  Gitee Go has no public v5 REST API; PR 门禁 only via third-party apps.
- `issue transfer/pin/lock/delete`: no Gitee API.
- `repo archive`: no Gitee API.
- codespaces / projects / attestations / discussions: no Gitee equivalent.

## Conventions for implementers

- Follow existing patterns: subcommands in src/cli.rs, handlers in src/cmd/<area>.rs,
  endpoint fns in src/api/<area>.rs.
- Every list command must respect --limit and --json.
- Endpoints marked "verify" are inferred from the v5 surface; confirm against
  https://gitee.com/api/v5/swagger before coding, and drop the subcommand if absent.
- Smoke-test against the live API (`gitee auth login`) before marking done.
