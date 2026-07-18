# pr create: full parameter surface

Status: ready-for-agent

## Context
`pr create` currently takes only title/body/head/base. The v5 create endpoint
accepts much more, and gh muscle memory expects --fill and templates.

## Scope
- New flags: --assignee, --tester, --label, --milestone, --close-issue <ident>
  (maps to close_related_issue), --draft is NOT supported (no Gitee concept).
- `--fill`: derive title (first commit subject) and body (commit list) from the
  head..base range via local git.
- Template: when no --body and no --fill, fetch .gitee/PULL_REQUEST_TEMPLATE.md
  then PULL_REQUEST_TEMPLATE.md from the base repo default branch via the
  contents API; prefill body. Missing template = empty body.

## Acceptance
- `gitee pr create --fill --assignee me --label bug --close-issue I1AB2C` round-trips;
  linked issue closes on merge (existing link machinery).
- Template fetched and prefilled when present.

## Non-goals
- Interactive prompting (ticket 23).
