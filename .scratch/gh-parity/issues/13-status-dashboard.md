# gitee status — cross-repo dashboard

Status: ready-for-agent

## Context
`gh status` parity. Gitee has GET /user/issues (filter created/assigned/
participating, state, since) covering all repos; a cross-repo pulls endpoint is
unconfirmed.

## Scope
- `gitee status`: issues sections (assigned to me, created by me, open) via
  /user/issues; PR sections only if a cross-repo PR endpoint verifies — otherwise
  omit with a code comment and a note in --help.
- --json support.

## Acceptance
- Output matches the web 工作台 issue lists for the authed user.

## Non-goals
- Notifications (no v5 endpoint).
