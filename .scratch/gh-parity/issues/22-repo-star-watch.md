# repo star / watch (beyond-parity)

Status: ready-for-agent

## Context
gh has no star command to this day; Gitee v5 exposes starring and subscriptions.

## Scope
- `repo star` / `repo unstar` — PUT/DELETE /user/starred/{owner}/{repo}.
- `repo watch` / `repo unwatch` — PUT/DELETE /user/subscriptions/{owner}/{repo}.
- Print current state after toggle (`repo view` already shows counts — add
  starred/watching booleans to its JSON if cheap via the check endpoints).

## Acceptance
- Toggle on a public repo, state confirmed via web UI.

## Non-goals
- Listing my starred repos (GET /user/starred exists; small add if trivial — note in file).
