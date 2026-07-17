# gitee

A `gh`-like command-line client for [Gitee](https://gitee.com). Manage pull requests, issues, and repositories from your terminal.

> **Note:** The crates.io crate name may differ from `gitee` if the name is already taken. Publishing to crates.io is TBD.

## Install

**From source (today):**

```bash
cargo install --path .
```

**Pre-built binaries (once published):**

```bash
cargo binstall gitee
```

Binaries are distributed via GitHub Releases. See [`.github/workflows/release.yml`](.github/workflows/release.yml) for the release build matrix.

## Authentication

The CLI needs a Gitee personal access token for API calls.

### Interactive login

```bash
gitee auth login
```

You will be prompted to paste a token. The token is validated against the Gitee API unless you pass `--force`.

You can also pass the token directly:

```bash
gitee auth login --token "$MY_TOKEN"
```

### CI / headless

Set the `GITEE_TOKEN` environment variable:

```bash
export GITEE_TOKEN=your_personal_access_token
```

This is the recommended approach for CI pipelines and scripts.

### Token storage

When you run `gitee auth login`, the token is stored using this precedence for **reading**:

1. **`$GITEE_TOKEN` environment variable** — highest priority; ideal for CI
2. **OS keyring** — default for interactive use (service: `gitee-cli`, account: host name e.g. `gitee.com`)
3. **Plaintext file fallback** — used when the keyring is unavailable, at `~/.config/gitee/<host>.token` (Unix)

When **writing** via `auth login`, the CLI tries the OS keyring first; if that fails, it falls back to the file with mode `0600` on Unix.

Check where your token comes from:

```bash
gitee auth status
```

Print the active token (e.g. to pipe into another tool):

```bash
gitee auth token
```

### Security

- Prefer the OS keyring for interactive machines; tokens are not written to disk when keyring storage succeeds.
- The file fallback is restricted to owner-read/write only (`0600`) on Unix.
- To remove stored credentials for a host, run `gitee auth logout`. This clears both keyring and file entries (it does not unset `$GITEE_TOKEN`; unset that in your shell or CI config separately).

## Global flags

These flags apply to all subcommands:

| Flag | Description |
|------|-------------|
| `--repo <owner/name>` | Target repository. If omitted, resolved from the current git remote. |
| `--remote <name>` | Git remote used to resolve the repo (default: `origin`). |
| `--host <host>` | Gitee host (default: `gitee.com`). |
| `--json`, `-j` | Emit JSON output. Bare `--json` prints full objects. |
| `--json <fields>` | Project output to comma-separated fields (e.g. `--json number,title`). Arrays project per element. |
| `--debug` | Log HTTP requests and responses to stderr. |

## Command reference

### Pull requests — `gitee pr`

| Subcommand | Description |
|------------|-------------|
| `list` | List pull requests |
| `view <number>` | Show pull request details |
| `diff <number>` | Show pull request diff |
| `checkout <number>` | Fetch and check out a pull request locally |
| `create` | Open a new pull request |
| `merge <number>` | Merge a pull request |
| `comment <number>` | Add a comment |
| `approve <number>` | Approve a pull request |
| `close <number>` | Close a pull request |
| `reopen <number>` | Reopen a closed pull request |
| `link <number> <issue>` | Link a pull request to an issue |

**Examples:**

```bash
# List open PRs (default limit 30)
gitee pr list

# Filter by state or author
gitee pr list --state merged --author alice --limit 10

# View PR #42
gitee pr view 42

# Show unified diff for PR #42
gitee pr diff 42

# Check out PR #42 as local branch pr-42
gitee pr checkout 42

# Create a PR
gitee pr create --title "Fix login bug" --body "Details here" --head feature-branch --base master

# Merge PR #42 (squash)
gitee pr merge 42 --squash

# Merge with rebase, leave linked issue open
gitee pr merge 42 --rebase --no-close-issue

# Comment on a PR
gitee pr comment 42 -m "LGTM"

# Approve (use --force if required by repo settings)
gitee pr approve 42
gitee pr approve 42 --force

# Close and reopen
gitee pr close 42
gitee pr reopen 42

# Link PR to issue
gitee pr link 42 ISSUE-123

# JSON output with selected fields
gitee pr list --json number,title,state
```

#### `pr list` flags

| Flag | Description |
|------|-------------|
| `--state <state>` | Filter by state |
| `--author <login>` | Filter by author |
| `--limit <n>` | Maximum results (default: `30`) |

#### `pr create` flags

| Flag | Description |
|------|-------------|
| `--title <title>` | **Required.** PR title |
| `--body <text>` | PR body |
| `--head <branch>` | Head branch |
| `--base <branch>` | Base branch |

#### `pr merge` flags

| Flag | Description |
|------|-------------|
| `--squash` | Squash merge |
| `--rebase` | Rebase merge |
| `--no-close-issue` | Do not close linked issues |

#### `pr comment` flags

| Flag | Description |
|------|-------------|
| `-m`, `--body <text>` | **Required.** Comment body |

#### `pr approve` flags

| Flag | Description |
|------|-------------|
| `--force` | Force approval |

---

### Issues — `gitee issue`

| Subcommand | Description |
|------------|-------------|
| `list` | List issues |
| `view <number>` | Show issue details |
| `create` | Create an issue |
| `close <number>` | Close an issue |
| `reopen <number>` | Reopen a closed issue |
| `link <number> <pr>` | Link an issue to a pull request |
| `comment <number>` | Add a comment |

**Examples:**

```bash
# List issues
gitee issue list

# Filter by state, assignee, or limit
gitee issue list --state open --assignee bob --limit 20

# View issue (number may be alphanumeric on some repos)
gitee issue view 17

# Create an issue
gitee issue create --title "Bug report" --body "Steps to reproduce…"
gitee issue create --title "Task" --assignee alice --labels "bug,priority-high"

# Close and reopen
gitee issue close 17
gitee issue reopen 17

# Link issue to PR #42
gitee issue link 17 42

# Comment on an issue
gitee issue comment 17 -m "Still investigating"

# JSON field projection
gitee issue view 17 --json title,state,body
```

#### `issue list` flags

| Flag | Description |
|------|-------------|
| `--state <state>` | Filter by state |
| `--assignee <login>` | Filter by assignee |
| `--limit <n>` | Maximum results (default: `30`) |

#### `issue create` flags

| Flag | Description |
|------|-------------|
| `--title <title>` | **Required.** Issue title |
| `--body <text>` | Issue body |
| `--assignee <login>` | Assignee |
| `--labels <list>` | Comma-separated labels |

#### `issue comment` flags

| Flag | Description |
|------|-------------|
| `-m`, `--body <text>` | **Required.** Comment body |

---

### Repositories — `gitee repo`

| Subcommand | Description |
|------------|-------------|
| `view [repo]` | Show repository details |
| `list [owner]` | List repositories |
| `clone <spec> [dir]` | Clone via git |
| `fork` | Fork the resolved repository |

**Examples:**

```bash
# View the repo resolved from git remote
gitee repo view

# View a specific repo
gitee repo view oschina/gitee

# List your authenticated user's repos
gitee repo list

# List a user or org's public repos
gitee repo list oschina --limit 50

# Clone by owner/name or full Gitee URL
gitee repo clone oschina/gitee
gitee repo clone oschina/gitee ./my-gitee-clone
gitee repo clone oschina/gitee --ssh

# Fork the current repo and add remote named "fork"
gitee repo fork --remote fork
```

#### `repo list` flags

| Flag | Description |
|------|-------------|
| `--limit <n>` | Maximum results (default: `30`) |

#### `repo clone` flags

| Flag | Description |
|------|-------------|
| `--ssh` | Use the SSH URL instead of HTTPS |

#### `repo fork` flags

| Flag | Description |
|------|-------------|
| `--remote <name>` | After forking, add the new repo as a git remote with this name |

---

### Authentication — `gitee auth`

| Subcommand | Description |
|------------|-------------|
| `login` | Store a personal access token |
| `status` | Show login status and token source |
| `token` | Print the active token |
| `logout` | Forget the stored token for the current host |

**Examples:**

```bash
# Interactive login (validates token)
gitee auth login

# Login with token flag
gitee auth login --token "$GITEE_TOKEN"

# Skip API validation (offline or restricted token)
gitee auth login --token "$GITEE_TOKEN" --force

# Check status
gitee auth status

# Print token for scripting
gitee auth token

# Log out of gitee.com (default host)
gitee auth logout

# Log out of a self-hosted instance
gitee --host git.example.com auth logout
```

#### `auth login` flags

| Flag | Description |
|------|-------------|
| `--token <token>` | Token to store (otherwise prompted) |
| `--force` | Skip token validation probe |

---

### Shell completions — `gitee completions`

Print a shell completion script for bash, zsh, fish, powershell, or elvish.

**Examples:**

```bash
# Detect shell from $SHELL
gitee completions bash >> ~/.bashrc
gitee completions zsh  > "${fpath[1]}/_gitee"
gitee completions fish > ~/.config/fish/completions/gitee.fish

# Explicit shell (when $SHELL cannot be detected)
gitee completions powershell
gitee completions elvish
```

If `$SHELL` cannot be detected, pass the shell name explicitly.

## Repository resolution

Most commands target a repository resolved from your current git directory:

1. Use `--repo owner/name` to override, or
2. Resolve `owner/name` from the `--remote` git remote URL (default remote: `origin`).

Run commands from inside a git clone, or pass `--repo` explicitly.

## License

MIT — see [Cargo.toml](Cargo.toml).
