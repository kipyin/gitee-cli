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

Token lookup order: `$GITEE_TOKEN` → OS keyring → `~/.config/gitee/<host>.token`.
Useful commands:

```bash
gitee auth status    # where am I logged in / where's the token from?
gitee auth token     # print the active token (for piping)
gitee auth logout    # forget the stored token
```

## Everyday commands

Run from inside a repo, or point anywhere with `--repo owner/name`.

```bash
# pull requests
gitee pr list
gitee pr status                      # created / assigned / awaiting my test
gitee pr view 42
gitee pr diff 42
gitee pr checkout 42                 # fetch into local branch pr-42
gitee pr create --title "Fix" --head my-branch
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
gitee issue create --title "Bug" --body "steps…"
gitee issue edit I88 --label bug --milestone v1.0
gitee issue comment I88 -m "looking into it"
gitee issue close I88

# cross-repo dashboard
gitee status                         # assigned / created open issues

# search
gitee search repos gitee --language Rust
gitee search issues login --state open
gitee search users kip

# releases
gitee release list
gitee release view v1.0.0
gitee release create --tag v1.0.0 --notes "changelog…"
gitee release upload v1.0.0 dist/*.tar.xz
gitee release download v1.0.0 --dir ./dist
gitee release edit v1.0.0 --notes "updated notes"

# repositories
gitee repo view oschina/git
gitee repo list
gitee repo clone oschina/git
gitee repo fork
gitee repo create my-tool --private
gitee repo edit --description "…"
gitee repo rename new-slug

# labels / milestones
gitee label list
gitee label create bug --color ff0000
gitee milestone list
gitee milestone create --title v1.0 --due-on 2026-12-31

# gists
gitee gist list
gitee gist create notes.md --desc "scratch"
gitee gist view <id> --raw

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
| `view <n>` | Show pull request details |
| `diff <n>` | Show pull request diff |
| `checkout <n>` | Fetch and check out a pull request locally |
| `create` | Open a PR (`--title` or `--fill`; `--body`, `--head`, `--base`, `--assignee`, `--tester`, `--label`, `--milestone`, `--close-issue`) |
| `edit <n>` | Edit metadata (`--title`, `--body`, `--assignee`, `--tester`, `--label`, `--milestone`) |
| `merge <n>` | Merge (`--squash`, `--rebase`, `--no-close-issue`) |
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
| `view <n>` | Show issue details (Gitee issue idents are strings, e.g. `I88`) |
| `create` | Create (`--title` required; `--body`, `--assignee`, `--labels`, `--milestone`, `--security-hole`) |
| `edit <n>` | Edit metadata (`--title`, `--body`, `--assignee`, `--label`, `--milestone`, `--security-hole`) |
| `close <n>` / `reopen <n>` | Change state |
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
| `view <tag>` | Show release details |
| `create` | Create (`--tag` required; `--name`, `--notes`, `--target`, `--prerelease`) |
| `upload <tag> <files…>` | Attach files to an existing release |
| `download <tag>` | Download assets (`--dir`, `--pattern`) |
| `edit <tag>` | Edit (`--name`, `--notes`, `--prerelease`) |
| `delete <tag>` | Delete (`--yes` to skip confirmation) |

</details>

<details>
<summary><strong>gitee repo</strong> — repositories</summary>

| Subcommand | Description |
|------------|-------------|
| `view [repo]` | Show repository details |
| `list [owner]` | List repos (yours, or a user/org's public repos) (`--limit`) |
| `clone <spec> [dir]` | Clone via git (`--ssh`) |
| `fork` | Fork the resolved repository (`--add-remote <name>`) |
| `create <name>` | Create under your account or `--org` (`--private`, `--description`, `--homepage`, `--gitignore`, `--license`) |
| `edit` | Edit settings (`--description`, `--homepage`, `--private`/`--public`, `--default-branch`) |
| `rename <path>` | Rename the URL slug |
| `delete` | Delete (`--yes` to skip confirmation) |

</details>

<details>
<summary><strong>gitee label</strong> — labels</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List labels (`--limit`) |
| `create <name>` | Create (`--color` required, hex without `#`) |
| `edit <name>` | Edit (`--name`, `--color`) |
| `delete <name>` | Delete (`--yes` to skip confirmation) |

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

</details>

<details>
<summary><strong>gitee auth</strong> — authentication</summary>

| Subcommand | Description |
|------------|-------------|
| `login` | Store a token (`--token`, `--force` to skip validation) |
| `status` | Show login status and token source |
| `token` | Print the active token |
| `logout` | Forget the stored token for the current host |

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

## Gitee-specific (Gitee 特色)

Features beyond GitHub CLI parity:

| Feature | Status | Notes |
|---------|--------|-------|
| `pr test` | shipped | 测试通过 gate; pairs with `pr approve` (审查通过) |
| `issue --security-hole` | shipped | mark an issue as a security hole on create/edit |
| `milestone` | shipped | full list/view/create/edit; Gitee requires `--due-on` on create |
| `pr` assignees **and** testers | shipped | dual review/test roles on create/edit/status |
| `repo star` / `watch` | planned | ticket 22 |
| `webhook` | planned | ticket 20 |

## License

MIT — see [Cargo.toml](Cargo.toml).
