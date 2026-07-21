# Research: How GitHub CLI (`gh`) implements its update notifier

Date: 2026-07-21  
Sources pinned to: `cli/cli` trunk `64bc042c65c1`, `cli/go-gh` trunk `a89b68a441b2`, Homebrew formula `Formula/g/gh.rb` (live main as of research).

## Verdict (short)

On **Homebrew-built** `gh` only (Go build tag `updateable`), a background goroutine may `GET https://api.github.com/repos/cli/cli/releases/latest` at most once per 24h, caching the result in `~/.local/state/gh/state.yml` (XDG/Windows variants apply). After a **successful** command, if a newer release was found, `gh` prints a yellow/cyan tip on **stderr** (plus `brew upgrade gh` when the binary lives under Homebrew’s prefix). Opt-out is env-only: `GH_NO_UPDATE_NOTIFIER`. Official release/MSI/deb/rpm builds leave the checker compiled out.

---

## 1. Where it learns the latest version

**API:** unauthenticated (relative to this call site) HTTP GET:

```
https://api.github.com/repos/{repo}/releases/latest
```

with `{repo}` = `"cli/cli"` when the `updateable` build tag is present.

- Implementation: [`internal/update/update.go`](https://github.com/cli/cli/blob/trunk/internal/update/update.go) `getLatestReleaseInfo` builds that URL and JSON-decodes into `ReleaseInfo` (`tag_name` → `Version`, `html_url` → `URL`, `published_at` → `PublishedAt`).
- First-party API docs: [Get the latest release](https://docs.github.com/en/rest/releases/releases#get-the-latest-release).
- The HTTP client comes from `cmdFactory.HttpClient()` ([`internal/ghcmd/cmd.go`](https://github.com/cli/cli/blob/trunk/internal/ghcmd/cmd.go) `checkForUpdate`), i.e. the normal authenticated factory client ([`pkg/cmd/factory/default.go`](https://github.com/cli/cli/blob/trunk/pkg/cmd/factory/default.go) `HttpClientFunc` → `api.NewHTTPClient` with auth config). The request itself does not special-case “no auth”; whatever the factory attaches is used.
- Comparison: `versionGreaterThan(latest, current)` via `hashicorp/go-version`, with a `git describe` suffix rewrite for source builds (`\d+-\d+-g[a-f0-9]{8}$`).

**Build-time gate (critical):** checking is compiled in only when `updaterEnabled != ""`.

| File | Build tag | `updaterEnabled` |
|------|-----------|------------------|
| [`internal/ghcmd/update_enabled.go`](https://github.com/cli/cli/blob/trunk/internal/ghcmd/update_enabled.go) | `updateable` | `"cli/cli"` |
| [`internal/ghcmd/update_disabled.go`](https://github.com/cli/cli/blob/trunk/internal/ghcmd/update_disabled.go) | `!updateable` | `""` |

Homebrew sets the tag:

```ruby
"GO_BUILDTAGS" => "updateable",
```

in [`Homebrew/homebrew-core` `Formula/g/gh.rb`](https://github.com/Homebrew/homebrew-core/blob/master/Formula/g/gh.rb) (passed through `script/build.go` as `go build -tags …`).

Comments in `update_enabled.go` state development builds do not notify by default, and point at the Homebrew formula + [cli/cli#11024 discussion](https://github.com/cli/cli/pull/11024#discussion_r2107597618).

**Historical intent:** [cli/cli#6977](https://github.com/cli/cli/pull/6977) (“Disable gh update checker in our precompiled binaries”) turned the checker off for official precompiled artifacts so package managers own updates; Homebrew remained the exception by injecting the enable flag (later the `updateable` tag in #11024).

---

## 2. Cache / TTL and file location

**TTL:** 24 hours. `CheckForUpdate` reads the state file; if `time.Since(checked_for_update_at).Hours() < 24`, it returns `(nil, nil)` without hitting the network ([`internal/update/update.go`](https://github.com/cli/cli/blob/trunk/internal/update/update.go)).

**Path:** `filepath.Join(config.StateDir(), "state.yml")` ([`cmd.go` `checkForUpdate`](https://github.com/cli/cli/blob/trunk/internal/ghcmd/cmd.go)).

`StateDir()` (via `cli/go-gh` [`pkg/config/config.go`](https://github.com/cli/go-gh/blob/trunk/pkg/config/config.go)):

1. `$XDG_STATE_HOME/gh` if `XDG_STATE_HOME` is set  
2. else on Windows: `%LocalAppData%/GitHub CLI`  
3. else: `$HOME/.local/state/gh`

**File shape** (YAML, mode `0600`):

```yaml
checked_for_update_at: <RFC3339 timestamp>
latest_release:
  version: vX.Y.Z          # from tag_name
  url: https://github.com/cli/cli/releases/tag/vX.Y.Z
  publishedat: <timestamp> # yaml key from PublishedAt
```

Observed locally after a Homebrew `gh` run: `~/.local/state/gh/state.yml` matching that schema.

**Write timing:** state is written **after** a successful latest-release fetch (`setStateEntry` after `getLatestReleaseInfo`). If the goroutine is cancelled before that write, the next run may check again (known rough edge discussed in [cli/cli#12599](https://github.com/cli/cli/issues/12599); proposed “write timestamp first” PR [#12605](https://github.com/cli/cli/pull/12605) was closed unmerged as of this research).

Docs also state the 24h cadence: [`gh help environment` / `GH_NO_UPDATE_NOTIFIER`](https://cli.github.com/manual/gh_help_environment) and [`pkg/cmd/root/help_topic.go`](https://github.com/cli/cli/blob/trunk/pkg/cmd/root/help_topic.go).

---

## 3. Opt-out env / config knobs

### Core CLI notifier

| Knob | Effect | Source |
|------|--------|--------|
| `GH_NO_UPDATE_NOTIFIER` (any non-empty value) | Skip check entirely | [`ShouldCheckForUpdate`](https://github.com/cli/cli/blob/trunk/internal/update/update.go); [manual](https://cli.github.com/manual/gh_help_environment) |
| `CODESPACES` non-empty | Skip | same |
| CI heuristics (`CI`, `BUILD_NUMBER`, or `RUN_ID` set) | Skip | [`internal/ci/ci.go`](https://github.com/cli/cli/blob/trunk/internal/ci/ci.go) `IsCI()` |
| stdout or stderr not a TTY | Skip | `IsTerminal` (isatty / Cygwin) |

**No config-file / `gh config` key** disables the core update notifier. Help text and `ShouldCheckForUpdate` only mention the env var (plus implicit CI/TTY/Codespaces gates).

Also effectively disabled when `updaterEnabled == ""` (non-`updateable` builds) — not a user knob, a compile-time one.

### Related (extensions, out of ticket scope but adjacent)

`GH_NO_EXTENSION_UPDATE_NOTIFIER` gates extension update notices via `ShouldCheckForExtensionUpdate` (same TTY/CI/Codespaces pattern; separate 24h `state.yml` under the extension update dir).

---

## 4. Homebrew detection

Two layers:

### A. Compile-time (who even has a checker)

Only builds with `-tags updateable` set `updaterEnabled = "cli/cli"`. Homebrew’s formula does this; stock `make bin/gh` / official release pipelines do not set `GO_BUILDTAGS`, so they compile `update_disabled.go` and never check.

### B. Runtime (message copy + grace period)

[`isUnderHomebrew`](https://github.com/cli/cli/blob/trunk/internal/ghcmd/cmd.go):

1. `LookPath("brew")`  
2. `brew --prefix`  
3. true iff `gh` executable path has prefix `{brewPrefix}/bin/`

Used only **after** a newer release is known:

- If Homebrew **and** `published_at` is within the last 24h (`isRecentRelease`), **suppress** the notice (“do not notify Homebrew users before the version bump had a chance to get merged into homebrew-core”).
- If Homebrew (and not suppressed), append upgrade hint: `To upgrade, run: brew upgrade gh`.

Homebrew is **not** used as the source of “what’s latest” — that remains GitHub Releases. Detection only adjusts UX / timing.

---

## 5. Exact user-visible message shape

Printed to **stderr** (`ioStreams.ErrOut`) only when `newRelease != nil` on the success path:

```text

A new release of gh is available: <currentWithoutV> → <latestWithoutV>
To upgrade, run: brew upgrade gh          ← only if isUnderHomebrew
<release html_url>

```

Formatting ([`cmd.go`](https://github.com/cli/cli/blob/trunk/internal/ghcmd/cmd.go) ~lines 255–270):

- Leading `\n\n`, trailing `\n\n` around the block.
- `"A new release of gh is available:"` — ANSI yellow  
- current and latest versions — ANSI cyan; leading `v` stripped via `strings.TrimPrefix(..., "v")`  
- arrow is the Unicode `→`  
- release URL — ANSI yellow  
- Homebrew upgrade line — plain (no color wrapper)

Non-Homebrew `updateable` builds (rare/unofficial) would omit the `brew upgrade` line but still show the headline + URL.

---

## 6. When relative to command execution it prints

Lifecycle in [`ghcmd.Main`](https://github.com/cli/cli/blob/trunk/internal/ghcmd/cmd.go):

1. **Before** command work: spawn goroutine `checkForUpdate(updateCtx, …)` → sends on `updateMessageChan`.
2. Run `rootCmd.ExecuteContextC(ctx)`.
3. **On any command error / non-success exit path:** return immediately **without** reading the channel or printing a notice (`updateCancel` runs via `defer` as the process exits).
4. **On success only:** `updateCancel()` (abort in-flight check if still running), then `newRelease := <-updateMessageChan`, then maybe `fmt.Fprintf(stderr, …)`.
5. Return `exitOK`.

So the tip is **post-command, stderr, success-only**, and only if the background check finished with a newer version before/as it was cancelled. Official docs phrase this as: when any command is executed, gh checks once every 24 hours and displays an upgrade notice on standard error ([environment help](https://cli.github.com/manual/gh_help_environment)).

Debug: if `GH_DEBUG` / debug enabled and the check errors, a `warning: checking for update failed: …` may be printed from the goroutine; ordinary failures stay silent.

---

## End-to-end flow

```mermaid
sequenceDiagram
  participant User
  participant Main as ghcmd.Main
  participant BG as checkForUpdate goroutine
  participant API as api.github.com
  participant State as state.yml

  User->>Main: gh <cmd>
  Main->>BG: start (if updateable + ShouldCheckForUpdate)
  BG->>State: read; skip if <24h
  alt due for check
    BG->>API: GET /repos/cli/cli/releases/latest
    API-->>BG: tag_name, html_url, published_at
    BG->>State: write checked_for_update_at + latest_release
  end
  Main->>Main: Execute command
  alt command failed
    Main-->>User: error exit (no notice)
  else command ok
    Main->>BG: cancel ctx; recv result
    alt newer + (not Homebrew-within-24h-of-publish)
      Main-->>User: stderr tip (+ brew line if Homebrew)
    end
  end
```

---

## Implications for gitee-cli (non-normative)

Useful parallels: GitHub Releases `latest` as authority, ~24h state file, stderr post-command tip, env opt-out, TTY/CI suppression, silent network failure, Homebrew-specific upgrade copy without treating brew as version authority.

Deliberate differences already in the map (config opt-out, all install methods, etc.) are product choices — `gh` itself does **not** offer a config opt-out and only enables the checker for Homebrew `updateable` builds.

---

## Primary sources

| Claim area | Source |
|------------|--------|
| Check + cache + compare | https://github.com/cli/cli/blob/trunk/internal/update/update.go |
| Orchestration, message, Homebrew runtime | https://github.com/cli/cli/blob/trunk/internal/ghcmd/cmd.go |
| Build tag enable/disable | https://github.com/cli/cli/blob/trunk/internal/ghcmd/update_enabled.go , `update_disabled.go` |
| State directory | https://github.com/cli/go-gh/blob/trunk/pkg/config/config.go (`StateDir`) |
| Env docs | https://cli.github.com/manual/gh_help_environment ; https://github.com/cli/cli/blob/trunk/pkg/cmd/root/help_topic.go |
| Homebrew build tag | https://github.com/Homebrew/homebrew-core/blob/master/Formula/g/gh.rb |
| Why precompiled builds disabled | https://github.com/cli/cli/pull/6977 |
| `updateable` tag introduction | https://github.com/cli/cli/pull/11024 |
| Latest-release REST API | https://docs.github.com/en/rest/releases/releases#get-the-latest-release |
| CI detection | https://github.com/cli/cli/blob/trunk/internal/ci/ci.go |
