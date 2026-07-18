# pr edit

Status: ready-for-agent

## Context
`gh pr edit` parity. Today PRs can only be created and state-flipped; any
metadata change needs the web UI.

## Scope
- `gitee pr edit <number>` with flags: --title --body --base --assignee <user>
  --tester <user> --label <name[,name]> --milestone <id|title>.
- API: PATCH /repos/{owner}/{repo}/pulls/{number}. Param names per swagger
  (title, body, state, base, assignees, testers, milestone_number, labels) —
  verify round-trip behavior, especially whether labels/assignees replace or merge.
- Repeatable --label/--assignee accumulate, gh-style.

## Acceptance
- Each flag's effect is visible via `gitee pr view <number> --json` afterwards.
- No flags: usage error (do NOT open an editor — interactive is ticket 23).

## Non-goals
- Reviewers interplay with 审查/测试 gates beyond setting assignees/testers.
