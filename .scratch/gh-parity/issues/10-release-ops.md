# release download / edit / delete

Status: ready-for-agent

## Context
`gh release` parity for everything except create/upload which exist.

## Scope
- `release download <tag> [--dir <d>] [--pattern <glob>]`: enumerate attach_files
  on the release, download each asset (send token if the URL 401s publicly).
- `release edit <tag>`: --name --notes --prerelease/--latest. API: PATCH
  /repos/{owner}/{repo}/releases/{id} (resolve id via get_by_tag).
- `release delete <tag>` (--yes to skip confirm). API: DELETE .../releases/{id}.

## Acceptance
- Round-trip on a test repo: upload → download (bytes identical) → edit → delete.

## Non-goals
- delete-asset granularity if the API lacks it (verify; note in file).

## Implementation notes (2026-07-18)

Implemented: download(--dir/--pattern)/edit/delete(--yes).

- `--latest` DROPPED: PATCH /releases/{id} has no latest param (verified 2026-07-18).
- PATCH requires tag_name, name, body on every call → edit GETs by tag then sends flag-or-current for all three.
- Download uses browser_download_url, token retry on 401/403 (per ticket). Gitee rejects the gitee-cli UA on authenticated asset redirects → get_bytes sends a curl UA (documented in client.rs). Private-repo auto archive assets (zip/tar.gz) may 403; uploaded attach_files download correctly.
- Follow-up available: asset-level DELETE /releases/{rid}/attach_files/{aid} exists in v5 (out of scope here).
