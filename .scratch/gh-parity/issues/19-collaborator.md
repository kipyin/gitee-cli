# collaborator command group

Status: ready-for-agent

## Context
Repo member management; gh exposes this via api only — beyond-parity convenience.

## Scope
- `collaborator list`, `collaborator add <username> [--permission pull|push|admin]`,
  `collaborator remove <username>` (confirm or --yes).
- API: GET /repos/{owner}/{repo}/collaborators,
  PUT/DELETE /repos/{owner}/{repo}/collaborators/{username}.
- Verify permission vocabulary (pull/push/admin vs 中文枚举) against live responses.

## Acceptance
- Round-trip on a scratch repo with a second account.

## Non-goals
- Org team management.
