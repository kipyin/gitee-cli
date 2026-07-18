# webhook command group

Status: ready-for-agent

## Context
Repo WebHook CRUD; beyond-parity (gh has no built-in hook command).

## Scope
- `webhook list`, `webhook create --url <u> [--events push_events,tag_push_events,
  issues_events,pull_requests_events,note_events] [--password]`,
  `webhook delete <id>` (confirm or --yes).
- API: GET/POST /repos/{owner}/{repo}/hooks, DELETE /repos/{owner}/{repo}/hooks/{id}.
- Verify event flag names against swagger; --json supported.

## Acceptance
- Create a hook pointing at a request-bin URL, see it listed, delete it.

## Non-goals
- Hook delivery logs/replay (no v5 endpoint confirmed).
