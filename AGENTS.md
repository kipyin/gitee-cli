# gitee-cli

## Agent skills

### Issue tracker

Issues and specs live as local markdown files under `.scratch/<feature>/` (one spec + one file per ticket). See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`), recorded as a `Status:` line in each issue file. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at the repo root, created lazily by the skills when terms/decisions get resolved. See `docs/agents/domain.md`.
