# Interactive create flows (design decision needed)

Status: needs-triage

## Context
`gh issue create` / `gh pr create` with no flags enter a prompt flow
(title → body editor → metadata → confirm). Big UX gap, but real design choices:

## Open questions
- Prompt crate: dialoguer vs inquire (both maintained; inquire is prettier,
  dialoguer is simpler).
- TTY detection + CI behavior: non-TTY with missing required flags must error,
  never hang.
- Editor handoff for body ($VISUAL/$EDITOR, tempfile), template prefill from
  ticket 04 as the initial buffer.
- Interaction with --fill: prompts prefilled but editable?

## Suggested direction
inquire + `atty`/`is_terminal` gating; --fill prefills; body step skipped when
--body given. Maintainer to confirm crate + flow before implementation.

## Non-goals
- Full TUI dashboards.
