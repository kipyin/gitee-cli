# Extension system (design decision needed)

Status: needs-triage

## Context
`gh extension` lets third parties ship `gh-<name>` binaries installed from git
repos. High value long-term, large design surface.

## Open questions
- Discovery/install: clone from a Gitee repo into ~/.local/share/gitee/extensions
  like gh, or simpler PATH-based discovery only (no install command) for v1?
- Naming: `gitee-<name>` on PATH executed via `gitee <name>` passthrough
  (arg forwarding, env: GITEE_TOKEN, GITEE_HOST).
- Upgrade/remove semantics; Windows support.

## Suggested direction
v1 = PATH passthrough + `extension list` only (no install/upgrade) — small,
unblocks third-party extensions; install/upgrade as v2. Maintainer to confirm.

## Non-goals
- A curated extension registry.
