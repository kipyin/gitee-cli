# browse + --web flags

Status: ready-for-agent

## Context
`gh browse` parity; pure client-side (open URL with the `open` crate or
platform fallback).

## Scope
- `gitee browse` — open resolved repo home.
- `--web` on: repo view, pr view, issue view, release view — open instead of printing.
- URL shapes: https://{host}/{owner}/{repo}, /pulls/{n}, /issues/{ident},
  /releases/{tag}. Verify release URL shape against the live site.
- Headless/no-browser: print the URL, exit 0 (don't error).

## Acceptance
- Each --web command opens the matching page (manual check) and headless prints URL.

## Non-goals
- browse --settings/--wiki/--branch path tricks.
