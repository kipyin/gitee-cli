# gitee

A `gh`-like command-line client for [Gitee](https://gitee.com). Manage pull requests, issues, and repositories from your terminal.

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
gitee pr list                        # open PRs
gitee pr view 42                     # details
gitee pr diff 42                     # unified diff
gitee pr checkout 42                 # fetch into local branch pr-42
gitee pr create --title "Fix" --head my-branch --base master
gitee pr comment 42 -m "LGTM"
gitee pr approve 42
gitee pr merge 42 --squash

# issues
gitee issue list
gitee issue view 17
gitee issue create --title "Bug" --body "steps…"
gitee issue comment 17 -m "looking into it"
gitee issue close 17

# repositories
gitee repo view oschina/git         # any repo, no clone needed
gitee repo list                     # your repos
gitee repo clone oschina/git        # clone via git
gitee repo fork                     # fork current repo
```

Handy global flags:

```bash
gitee pr list --json number,title   # JSON, selected fields (bare --json = full)
gitee --debug pr list               # show HTTP requests on stderr
gitee --repo owner/name pr list     # target a repo without cd'ing into it
gitee --host git.example.com ...    # self-hosted Gitee
```

## Command reference

<details>
<summary><strong>gitee pr</strong> — pull requests</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List pull requests (`--state`, `--author`, `--limit`) |
| `view <n>` | Show pull request details |
| `diff <n>` | Show pull request diff |
| `checkout <n>` | Fetch and check out a pull request locally |
| `create` | Open a PR (`--title` required, `--body`, `--head`, `--base`) |
| `merge <n>` | Merge (`--squash`, `--rebase`, `--no-close-issue`) |
| `comment <n>` | Add a comment (`-m/--body`) |
| `approve <n>` | Approve (`--force`) |
| `close <n>` / `reopen <n>` | Change state |
| `link <n> <issue>` | Link a pull request to an issue |

</details>

<details>
<summary><strong>gitee issue</strong> — issues</summary>

| Subcommand | Description |
|------------|-------------|
| `list` | List issues (`--state`, `--assignee`, `--limit`) |
| `view <n>` | Show issue details |
| `create` | Create an issue (`--title` required, `--body`, `--assignee`, `--labels`) |
| `close <n>` / `reopen <n>` | Change state |
| `link <n> <pr>` | Link an issue to a pull request |
| `comment <n>` | Add a comment (`-m/--body`) |

</details>

<details>
<summary><strong>gitee repo</strong> — repositories</summary>

| Subcommand | Description |
|------------|-------------|
| `view [repo]` | Show repository details |
| `list [owner]` | List repos (yours, or a user/org's public repos) (`--limit`) |
| `clone <spec> [dir]` | Clone via git (`--ssh`) |
| `fork` | Fork the resolved repository (`--add-remote <name>`) |

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
| `--debug` | Log HTTP requests/responses to stderr |

## Repository resolution

Most commands operate on a repository resolved two ways:

1. `--repo owner/name`, or
2. from the current directory's git remote (`--remote`, default `origin`).

So either `cd` into a clone, or pass `--repo` explicitly.

## License

MIT — see [Cargo.toml](Cargo.toml).
