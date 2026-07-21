# gitee

A `gh`-like command-line client for [Gitee](https://gitee.com). Manage pull
requests, issues, releases, gists, and more from your terminal.

## Quick start

**1 — Install** (pick one):

```bash
cargo install gitee-cli-rs      # build from crates.io
cargo binstall gitee-cli-rs     # pre-built binary (no compile)
brew install kipyin/tap/gitee   # macOS via Homebrew
```

Or grab a binary from [GitHub Releases](https://github.com/kipyin/gitee-cli/releases).

**2 — Log in** with a Gitee [personal access token](https://gitee.com/profile/personal_access_tokens):

```bash
gitee auth login
```

**3 — Use it.** Run inside any git clone (or add `--repo owner/name`):

```bash
gitee pr list        # pull requests
gitee issue list     # issues
gitee status         # your open issues across repos
gitee release list   # releases
gitee repo view      # repo info
```

That's it. Everything below is detail.

> **Scripting gitee-cli?** See [docs/scripting.md](docs/scripting.md) for exit
> codes, `--preview`, idempotent mutating verbs, and CI/agent patterns.

## Install

| Method | Command | Notes |
|--------|---------|-------|
| crates.io | `cargo install gitee-cli-rs` | builds from source |
| cargo-binstall | `cargo binstall gitee-cli-rs` | same binary, no compile |
| Homebrew (macOS) | `brew install kipyin/tap/gitee` | arm64 + x86_64 |
| Direct download | [Releases](https://github.com/kipyin/gitee-cli/releases) | `gitee-<target>-v<ver>.tar.xz` |
| From source | `cargo install --path .` | after `git clone` |

> The crates.io package is `gitee-cli-rs`, but the installed command is `gitee`.

Shell completions (bash/zsh/fish/powershell/elvish):

```bash
gitee completions zsh > "${fpath[1]}/_gitee"
```

## Configure

The CLI needs a Gitee personal access token. Create one at
<https://gitee.com/profile/personal_access_tokens> (default scopes are fine for
reading; check **pull_requests**, **issues**, and **projects** for write actions).

```bash
gitee auth login                  # prompts for the token, validates it, stores it
```

For CI / scripts, use an environment variable instead:

```bash
export GITEE_TOKEN=your_token
```

Token lookup order: `$GITEE_TOKEN` → OS keyring → `~/.config/gitee/<host>/…`
(per-user token files after login; legacy `~/.config/gitee/<host>.token` is migrated).

Useful commands:

```bash
gitee auth status                 # where am I logged in / who's active?
gitee auth token                  # print the active token (for piping)
gitee auth switch --user alice    # pick among saved accounts on this host
gitee auth setup-git              # use gitee as git credential helper
gitee auth logout                 # forget the stored token
gitee config set editor nvim      # defaults: host / remote / editor
gitee alias set co pr checkout    # expand `gitee co 42` → `gitee pr checkout 42`
```

## Everyday commands

Run from inside a repo, or point anywhere with `--repo owner/name`.

```bash
# pull requests
gitee pr list
gitee pr status                      # created / assigned / awaiting my test
gitee pr view 42
gitee pr view 42 --web               # open in browser
gitee pr diff 42
gitee pr checkout 42                 # fetch into local branch pr-42
gitee pr create --title "Fix" --head my-branch
gitee pr create                      # interactive title/body on a TTY
gitee pr create --fill               # title/body from commits
gitee pr edit 42 --title "Retitle" --label bug
gitee pr comment 42 -m "LGTM"
gitee pr approve 42                  # 审查通过
gitee pr test 42                     # 测试通过 (Gitee-specific)
gitee pr merge 42 --squash

# issues
gitee issue list
gitee issue status                   # created / assigned to me
gitee issue view I88
gitee issue view I88 --web
gitee issue create --title "Bug" --body "steps…"
gitee issue create                   # interactive title/body on a TTY
gitee issue edit I88 --label bug --milestone v1.0
gitee issue edit I88 --state progressing   # open|progressing|closed|rejected
gitee issue comment I88 -m "looking into it"
gitee issue close I88

# cross-repo dashboard / browser
gitee status                         # assigned / created open issues
gitee browse                         # open the resolved repo in a browser

# search
gitee search repos gitee --language Rust
gitee search issues login --state open
gitee search users kip

# releases
gitee release list
gitee release view v1.0.0
gitee release view v1.0.0 --web
gitee release create --tag v1.0.0 --notes "changelog…"
gitee release upload v1.0.0 dist/*.tar.xz
gitee release download v1.0.0 --dir ./dist
gitee release edit v1.0.0 --notes "updated notes"

# repositories
gitee repo view oschina/git
gitee repo view --web
gitee repo list
gitee repo clone oschina/git
gitee repo fork
gitee repo create my-tool --private
gitee repo edit --description "…"
gitee repo rename new-slug
gitee repo star                      # also: unstar / watch / unwatch

# labels / milestones
gitee label list
gitee label create bug --color ff0000
gitee milestone list
gitee milestone create --title v1.0 --due-on 2026-12-31

# org / access / hooks
gitee org list
gitee ssh-key list
gitee ssh-key add ~/.ssh/id_ed25519.pub --title laptop
gitee collaborator list
gitee collaborator add alice --permission push
gitee webhook list
gitee webhook create --url https://example.com/hook --events push_events

# gists
gitee gist list
gitee gist create notes.md --desc "scratch"
gitee gist view <id> --raw

# config / aliases / extensions
gitee config list
gitee alias list
gitee extension list                 # gitee-* binaries on PATH + managed dir
gitee extension install owner/my-ext # clone + (optionally) build, into managed dir
gitee extension create demo         # scaffold a new extension in cwd
gitee extension remove demo         # delete an installed extension
gitee extension upgrade [name]      # git pull + rebuild (all if no name)
# unknown commands also exec `gitee-<name>` from PATH (gh-style)

# raw API escape hatch
gitee api user
gitee api repos/oschina/git/releases --paginate
```

Handy global flags:

```bash
gitee pr list --json number,title          # JSON field projection
gitee pr list --json --jq '.[].title'      # jq after projection
gitee --debug pr list                      # HTTP trace on stderr
gitee --repo owner/name pr list            # target a repo without cd'ing
gitee --remote gitee pr status             # resolve via a non-origin remote
gitee --host git.example.com ...           # self-hosted Gitee
```

## Command reference

<details>
<summary><strong>gitee pr</strong> — pull requests</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List PRs (`--state`, `--author`, `--limit`) |
| `status` | Open PRs relevant to you: created, assigned, awaiting your test (`--limit`) |
| `view <n>` | Show pull request details (`--web` opens in browser); `--json` includes a `files` array with per-file `path` / `additions` / `deletions` / `changes` |
| `diff <n>` | Show pull request diff |
| `checkout <n>` | Fetch and check out a pull request locally |
| `create` | Open a PR (`--title` / `--fill`, or interactive on a TTY; `--body`, `--head`, `--base`, `--assignee`, `--tester`, `--label`, `--milestone`, `--close-issue`) |
| `edit <n>` | Edit metadata (`--title`, `--body`, `--assignee`, `--tester`, `--label`, `--milestone`) |
| `merge <n>` | Merge (`--squash`, `--rebase`, `--no-close-issue`) — **idempotent**: already-merged exits `0` |
| `comment <n>` | Add a comment (`-m/--body`) |
| `approve <n>` | Approve / 审查通过 (`--force`) |
| `test <n>` | Mark tested / 测试通过 (`--force`) — Gitee-specific |
| `close <n>` / `reopen <n>` | Change state |
| `link <n> <issue>` | Link a pull request to an issue |

</details>

<details>
<summary><strong>gitee issue</strong> — issues</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List issues (`--state`, `--assignee`, `--limit`) |
| `status` | Open issues relevant to you: created, assigned (`--limit`) |
| `view <n>` | Show issue details (`--web`; Gitee issue idents are strings, e.g. `I88`) |
| `create` | Create (`--title` or interactive on a TTY; `--body`, `--assignee`, `--labels`, `--milestone`, `--security-hole`) |
| `edit <n>` | Edit metadata (`--title`, `--body`, `--assignee`, `--label`, `--milestone`, `--security-hole`, `--state`) |
| `close <n>` / `reopen <n>` | Change state — **idempotent**: already-closed/open exits `0` (`open`/`closed` shortcuts; prefer `edit --state` for `progressing`/`rejected`) |
| `link <n> <pr>` | Link an issue to a pull request |
| `comment <n>` | Add a comment (`-m/--body`) |

</details>

<details>
<summary><strong>gitee status</strong> — cross-repo dashboard</summary>

| Flag | Description |
|------|-------------|
| `--limit` | Cap each section (default 30) |

Shows open issues assigned to you and created by you across all repos. PR
sections are omitted — Gitee v5 has no user-level pulls endpoint.

</details>

<details>
<summary><strong>gitee search</strong> — search</summary>

| Subcommand | Description |
|------------|-------------|
| `repos <query>` | Search repositories (`--owner`, `--language`, `--fork`, `--sort`, `--order`, `--limit`) |
| `issues <query>` | Search issues (`--state`, `--author`, `--assignee`, `--label`, `--language`, `--sort`, `--order`, `--limit`; scoped with global `--repo`) |
| `users <query>` | Search users (`--sort`, `--order`, `--limit`) |

</details>

<details>
<summary><strong>gitee release</strong> — releases</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List releases (`--limit`) |
| `view <tag>` | Show release details (`--web` opens in browser) |
| `create` | Create (`--tag` required; `--name`, `--notes`, `--target`, `--prerelease`) |
| `upload <tag> <files…>` | Attach files to an existing release |
| `download <tag>` | Download assets (`--dir`, `--pattern`) |
| `edit <tag>` | Edit (`--name`, `--notes`, `--prerelease`) |
| `delete <tag>` | Delete (`--yes` to skip confirmation) — deleting a missing release exits `4` |

</details>

<details>
<summary><strong>gitee repo</strong> — repositories</summary>

| Subcommand | Description |
|------------|-------------|
| `view [repo]` | Show repository details (`--web` opens in browser) |
| `list [owner]` | List repos (yours, or a user/org's public repos) (`--limit`) |
| `clone <spec> [dir]` | Clone via git (`--ssh`) |
| `fork` | Fork the resolved repository (`--add-remote <name>`) |
| `create <name>` | Create under your account or `--org` (`--private`, `--description`, `--homepage`, `--gitignore`, `--license`) |
| `edit` | Edit settings (`--description`, `--homepage`, `--private`/`--public`, `--default-branch`) |
| `rename <path>` | Rename the URL slug |
| `star` / `unstar` | Star or unstar the resolved repository |
| `watch` / `unwatch` | Watch or unwatch the resolved repository |
| `delete` | Delete (`--yes` to skip confirmation) |

</details>

<details>
<summary><strong>gitee label</strong> — labels</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List labels (`--limit`) |
| `create <name>` | Create (`--color` required, hex without `#`) — **idempotent**: same name + same color exits `0`; same name + different color exits `1` with `gitee label edit <name> --color <c>` hint |
| `edit <name>` | Edit (`--name`, `--color`) |
| `delete <name>` | Delete (`--yes` to skip confirmation) — deleting a missing label exits `4` |

</details>

<details>
<summary><strong>gitee milestone</strong> — milestones</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List milestones (`--state`, `--limit`) |
| `view <n>` | Show milestone details |
| `create` | Create (`--title` and `--due-on YYYY-MM-DD` required; `--description`, `--state`) |
| `edit <n>` | Edit (`--title`, `--due-on`, `--description`, `--state`) |

</details>

<details>
<summary><strong>gitee gist</strong> — gists</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List your gists (`--limit`) |
| `view <id>` | Show a gist (`--raw` prints file contents) |
| `create <files…>` | Create (`--desc`, `--public`, `--filename` when reading `-` from stdin) |
| `edit <id> <file>` | Replace one file's contents |
| `delete <id>` | Delete (`--yes` to skip confirmation) |

</details>

<details>
<summary><strong>gitee api</strong> — raw REST passthrough</summary>

Like `gh api`. Pass an endpoint path, optional `-X` method, `-F`/`-f` fields,
`-H` headers, `--input`, and `--paginate` for array paging.

```bash
gitee api user
gitee api /repos/oschina/git/issues -X POST -F title=Bug -F body=steps
gitee api repos/oschina/git/releases --paginate
```

Issue state changes are a common footgun: use
`PATCH /repos/{owner}/issues/{number}` with a **JSON** body
`{"repo":"<name>","title":"<current title>","state":"progressing"}`
(title must be echoed or Gitee blanks it). The
`/repos/{owner}/{repo}/issues/{number}` path with form `-f state=…` often
returns `404 {"message":"project or enterprise"}`. Prefer
`gitee issue edit <n> --state …` when you can.

</details>

<details>
<summary><strong>gitee auth</strong> — authentication</summary>

| Subcommand | Description |
|------------|-------------|
| `login` | Store a token (`--token`, `--force` to skip validation) |
| `status` | Show login status, active user, and token source |
| `token` | Print the active token |
| `logout` | Forget the stored token for the current host |
| `switch --user <name>` | Switch the active saved account for this host |
| `setup-git` | Configure git to use `gitee` as credential helper for this host |
| `git-credential` | Git credential-helper protocol (`get` / `store` / `erase`; usually invoked by git) |

</details>

<details>
<summary><strong>gitee browse</strong> — open in browser</summary>

Opens the resolved repository in your default browser. Prefer `view --web` when you already know the PR / issue / release / repo target.

</details>

<details>
<summary><strong>gitee org</strong> — organizations</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List organizations for the authenticated user (`--limit`) |

</details>

<details>
<summary><strong>gitee ssh-key</strong> — SSH keys</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List your SSH public keys (`--limit`) |
| `add <pubkey-file>` | Upload a public key (`--title`) |
| `delete <id>` | Delete a key (`--yes` to skip confirmation) |

</details>

<details>
<summary><strong>gitee collaborator</strong> — repository collaborators</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List collaborators on the resolved repo (`--limit`) |
| `add <username>` | Add a collaborator (`--permission` pull\|push\|admin, default `push`) |
| `remove <username>` | Remove a collaborator (`--yes` to skip confirmation) |

</details>

<details>
<summary><strong>gitee webhook</strong> — repository webhooks</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List webhooks (`--limit`) |
| `create` | Create (`--url` required; `--events` push_events/tag_push_events/issues_events/merge_requests_events/note_events; `--password`) |
| `delete <id>` | Delete (`--yes` to skip confirmation) |

</details>

<details>
<summary><strong>gitee config</strong> — defaults</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | Show configured keys |
| `get <key>` | Read one key (`host`, `remote`, `editor`) |
| `set <key> <value>` | Write one key |

Stored in `~/.config/gitee/config.json`. CLI flags still win over these defaults.

</details>

<details>
<summary><strong>gitee alias</strong> — command aliases</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | Show aliases |
| `set <name> <expansion…>` | Define an alias (shell-quote multi-word expansions) |
| `delete <name>` | Remove an alias |

Example: `gitee alias set co pr checkout` → `gitee co 42` expands to `gitee pr checkout 42`.

</details>

<details>
<summary><strong>gitee extension</strong> — PATH extensions</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List `gitee-*` executables discovered on `PATH` and in the managed dir |
| `install <owner/repo> [--build cargo\|npm] [-y]` | Clone the repo into the managed dir and (optionally) build it |
| `create <name> [--cargo]` | Scaffold a new extension project in the current directory |
| `remove <name> [-y]` | Delete an installed extension from the managed dir |
| `upgrade [name]` | `git pull` (and rebuild, if needed) one or all installed extensions |

Unknown top-level commands also exec `gitee-<name>` from `PATH` (same model as `gh`).

### Extensions

Installed extensions live in a managed dir (no shell `PATH` mutation):

- Linux/macOS: `~/.local/share/gitee/extensions/<name>/`
- Windows: `%LOCALAPPDATA%\gitee\extensions\<name>\`

The CLI's extension resolver scans this managed dir **before** `PATH`, so an
installed extension shadows a same-named binary elsewhere. The directory layout
is `<name>/gitee-<name>` — the entry point must be a `gitee-<name>` executable at
the repo root (no build step) unless `--build cargo` or `--build npm` is given.

**Trust model.** `gitee extension install` downloads and runs arbitrary code.
Before cloning it prints the repo URL and last commit short SHA and asks for
confirmation (`--yes` skips). There is no signature verification — install only
from repos you trust.

**Build systems.**

- `--build cargo`: runs `cargo build --release`, copies the resulting binary
  (named after the crate, or `gitee-<name>`) to the extension dir root.
- `--build npm`: runs `npm install` and `npm run build` (if a `build` script
  exists); the `gitee-<name>` script at the repo root is the entry point.
- Default (no `--build`): the repo must already contain a `gitee-<name>`
  executable at the root.

**Environment contract** (forwarded to every extension child process):

- `GITEE_TOKEN` — the active personal access token (or your own `$GITEE_TOKEN`).
- `GITEE_HOST` — the active Gitee host (e.g. `gitee.com`), unless already exported.
- All trailing argv, forwarded verbatim.

</details>

### Global flags

| Flag | Description |
|------|-------------|
| `--repo <owner/name>` | Target repository (default: resolved from git remote) |
| `--remote <name>` | Git remote to resolve the repo from (default: `origin`) |
| `--host <host>` | Gitee host (default: `gitee.com`) |
| `--json [fields]` | JSON output; `--json number,title` projects fields |
| `--jq <expr>` | jq expression on `--json` output (requires `--json`) |
| `--debug` | Log HTTP requests/responses to stderr |
| `--preview` | Print what would happen and exit 0 (no HTTP call); mutating verbs only |

## Exit codes

`gitee` exits with a stable, documented code so scripts can switch on `$?`
instead of parsing stderr. See [docs/scripting.md](docs/scripting.md) for
the full table and patterns.

| Code | Meaning |
|------|---------|
| `0` | success (including idempotent no-ops) |
| `1` | generic failure |
| `2` | usage error (missing flag, bad arg, non-TTY prompt attempted) |
| `3` | auth error (no token / invalid / expired) |
| `4` | not found (repo / issue / PR / release) |
| `5` | rate limited (HTTP 429) |
| `6` | network error (host unreachable) |

When `--json` is set, errors print to stderr as
`{"code":"not_found","message":"…","exit_code":4}` — see
[docs/scripting.md](docs/scripting.md).

## Repository resolution

Most commands operate on a repository resolved two ways:

1. `--repo owner/name`, or
2. from the current directory's git remote (`--remote`, default `origin`).

So either `cd` into a clone, or pass `--repo` / `--remote` explicitly.

## Compared to `gh` (与 gh 的差异)

Gitee OpenAPI v5 does not expose everything GitHub CLI can reach. These are
**not planned** (no public API / no Gitee equivalent):

- `gh workflow` / `gh run` / `gh pr checks` / secrets / variables — Gitee Go has
  no public v5 REST API; PR 门禁 is only available via third-party apps
- `issue transfer` / `pin` / `lock` / `delete` — no Gitee API
- `repo archive` — no Gitee API
- codespaces / projects / attestations / discussions — no Gitee equivalent

Also omitted after swagger verification: `repo sync`, `pr update-branch`.

An MCP server is intentionally **not** built into this CLI — Gitee already
ships an official one ([`mcp-gitee`](https://gitee.com/oschina/mcp-gitee), Go).
For AI agent runtimes that prefer native MCP integration, use the official
server. For shell-out / CI patterns, see
[Scripting gitee-cli for agents](#scripting-gitee-cli-for-agents) below.

## Gitee-specific (Gitee 特色)

Features beyond GitHub CLI parity:

| Feature | Status | Notes |
|---------|--------|-------|
| `pr test` | shipped | 测试通过 gate; pairs with `pr approve` (审查通过) |
| `issue --security-hole` | shipped | mark an issue as a security hole on create/edit |
| `milestone` | shipped | full list/view/create/edit; Gitee requires `--due-on` on create |
| `pr` assignees **and** testers | shipped | dual review/test roles on create/edit/status |
| `repo star` / `watch` | shipped | `star` / `unstar` / `watch` / `unwatch` |
| `webhook` | shipped | list / create / delete repository hooks |

## Scripting gitee-cli for agents

AI agents and CI scripts can drive this CLI directly — no MCP required.

- **Native MCP integration** (for agent runtimes like Claude Code, Cursor,
  opencode): use Gitee's official
  [`mcp-gitee`](https://gitee.com/oschina/mcp-gitee) (Go). We do not ship our
  own MCP server; the official one is active and covers the read/write surface.
- **Shell-out patterns** (for agents that exec commands, and for CI): every
  verb supports `--json` + `--jq` for structured output, stable exit codes
  (0–6), structured JSON errors on stderr in `--json` mode, idempotent
  mutating verbs (already-closed / already-merged is exit 0), and `--preview`
  to dry-run a mutation. See [`docs/scripting.md`](docs/scripting.md) for the
  PR→issue closure loop, batch operations, CI status gates, and error
  handling by exit code.
- **Extensions as agent tools**: a MCP server can also be installed as a
  `gitee` extension via `gitee extension install <owner/repo> --build cargo`
  (see [Extensions](#extensions) above) if a Rust implementation you trust
  ever emerges.

## License

MIT — see [Cargo.toml](Cargo.toml).
