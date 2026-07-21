# Scripting gitee-cli

`gitee` is designed to be safe to call from AI agents and CI scripts, not
just humans at a shell. This page documents the patterns that make that
work — no extra flags or surface, just the runtime behavior that matters.

## Exit codes

`gitee` exits with a stable, documented code so scripts can switch on `$?`
instead of parsing stderr.

| Code | Meaning                                   |
|------|-------------------------------------------|
| `0`  | success (including idempotent no-ops)     |
| `1`  | generic failure (API error, unexpected)  |
| `2`  | usage error (missing flag, bad arg, non-TTY prompt attempted) |
| `3`  | auth error (no token, invalid, expired)   |
| `4`  | not found (repo / issue / PR / release)   |
| `5`  | rate limited (HTTP 429)                   |
| `6`  | network error (host unreachable)         |

```bash
gitee issue close I88
case $? in
  0)   echo "closed (or was already closed)" ;;
  3)   echo "not logged in — set GITEE_TOKEN" ;;
  4)   echo "issue not found" ;;
  5)   echo "rate limited; retry later" ;;
  6)   echo "network error" ;;
  *)   echo "unexpected: $?" ;;
esac
```

## TTY detection — never hangs

Every verb that would normally prompt (issue/pr create without `--title`,
`auth login` without `--token`, deletes without `--yes`) checks
`stdin.is_terminal()` first. If stdin is **not** a TTY (piped, redirected,
or `/dev/null`), the verb **errors** with exit `2` and a message naming the
flag you need to pass. It never blocks waiting for input.

```bash
gitee issue create < /dev/null
# error: issue create needs --title
# (exit 2)

gitee issue create --title "Bug" --body "steps" < /dev/null
# works
```

## `--json` errors

When `--json` is set, errors print to **stderr** as a structured envelope:

```json
{"code":"not_found","message":"not found (HTTP 404): /repos/o/r/issues/I999","exit_code":4}
```

The `code` field is a stable slug (`auth`, `not_found`, `rate_limited`,
`usage`, `network`, `http`, `io`, `config`, `repo_resolve`, `error`).
Without `--json`, errors stay human-readable `error: …` lines on stderr.
The exit code is the same either way.

## Idempotent mutating verbs

Mutating verbs are safe to retry — "already in target state" is success,
not an error.

| Verb                              | Already-in-state behavior                        |
|-----------------------------------|--------------------------------------------------|
| `issue close <n>`                 | prints "issue <n> already closed", exit `0`       |
| `issue reopen <n>`                | prints "issue <n> already open", exit `0`         |
| `pr close <n>`                    | prints "Pull request !<n> already closed", exit `0` |
| `pr reopen <n>`                   | prints "Pull request !<n> already open", exit `0`  |
| `pr merge <n>`                    | prints "Pull request !<n> already merged", exit `0` |
| `label create <name> --color <c>` | same name + same color → exit `0` (no-op); same name + different color → exit `1` with `gitee label edit <name> --color <c>` hint |

With `--json`, the idempotent no-op returns a structured object on stdout:

```json
{"number":"I88","state":"closed","message":"already closed"}
```

## `--preview` on mutating verbs

`--preview` prints what would happen and exits `0` **without calling the
API**. Use it to verify intent before acting.

```bash
gitee issue close I88 --preview
# would close issue I88: repo=oschina/gitee-cli

gitee pr create --title "Fix" --head feature/x --preview
# would create pull request: repo=oschina/gitee-cli, title=Fix, head=feature/x, base=gitee-cli default branch
```

`--debug` confirms no HTTP request was made.

## Patterns

### PR → issue closure loop

Create a PR, link it to the issue, close the issue after merge — all from
`--json` output, no scraping.

```bash
set -e
PR=$(gitee pr create --title "Fix login" --head fix/login --json number | jq -r .number)
gitee issue comment I88 -m "Opening PR !$PR"
gitee pr merge "$PR" --squash
gitee issue close I88
```

### Batch close by label

```bash
gitee issue list --label bug --json number | jq -r '.[].number' \
  | xargs -n1 gitee issue close
```

### Idempotent retry

`gitee issue close I88` is safe to retry — already-closed is exit `0`, not
an error. Wrap a flaky network in a retry loop without state tracking:

```bash
for i in 1 2 3; do
  if gitee issue close I88; then break; fi
  echo "retry $i…"
  sleep 1
done
```

### Error handling by exit code

```bash
if ! gitee issue close I88; then
  case $? in
    4) echo "issue missing — already deleted?";;
    5) echo "rate limited; backing off"; sleep 60; gitee issue close I88;;
    *) exit $?;;
  esac
fi
```