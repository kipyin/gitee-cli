use clap::{Parser, Subcommand};
use std::ffi::OsString;

#[derive(Parser)]
#[command(name = "gitee", version, about = "Gitee CLI (gh-like)")]
pub struct Cli {
    /// Target repository as owner/name (otherwise resolved from `git remote`).
    #[arg(long, global = true)]
    pub repo: Option<String>,
    /// Git remote to resolve the repo from (default: origin).
    #[arg(long, global = true)]
    pub remote: Option<String>,
    /// Gitee host (default: gitee.com).
    #[arg(long, global = true, default_value = "gitee.com")]
    pub host: String,
    /// Emit JSON. Bare `--json` prints full objects; `--json number,title`
    /// projects to those fields (arrays project per element).
    #[arg(long, short = 'j', global = true, num_args = 0..=1, default_missing_value = "")]
    pub json: Option<String>,
    /// jq expression applied to --json output (requires --json). Runs AFTER field
    /// projection. String scalars print unquoted; other results print as JSON;
    /// multiple results print one per line.
    #[arg(long, global = true, requires = "json")]
    pub jq: Option<String>,
    /// Log HTTP requests and responses to stderr.
    #[arg(long, global = true)]
    pub debug: bool,
    /// Print what would happen, then exit 0. No HTTP call is made. Works on
    /// mutating verbs (issue/pr create/close/reopen/merge, label create,
    /// release create/edit/delete, repo delete, gist delete, etc.).
    #[arg(long, global = true)]
    pub preview: bool,
    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(subcommand)]
    Pr(PrCmd),
    #[command(subcommand)]
    Issue(IssueCmd),
    #[command(subcommand)]
    Release(ReleaseCmd),
    #[command(subcommand)]
    Milestone(MilestoneCmd),
    #[command(subcommand)]
    Repo(RepoCmd),
    #[command(subcommand)]
    Label(LabelCmd),
    #[command(subcommand)]
    Auth(AuthCmd),
    #[command(subcommand)]
    Search(SearchCmd),
    /// Raw Gitee REST API request (like `gh api`).
    Api(ApiArgs),
    #[command(subcommand)]
    Gist(GistCmd),
    #[command(subcommand)]
    Org(OrgCmd),
    #[command(name = "ssh-key", subcommand)]
    SshKey(SshKeyCmd),
    #[command(subcommand)]
    Collaborator(CollaboratorCmd),
    #[command(subcommand)]
    Webhook(WebhookCmd),
    #[command(subcommand)]
    Config(ConfigCmd),
    #[command(subcommand)]
    Alias(AliasCmd),
    /// Open the repository in a browser.
    Browse,
    /// Cross-repo dashboard of your open issues. PR sections are omitted: Gitee v5 has no user-level pulls endpoint (swagger verified 2026-07-18).
    Status {
        #[command(flatten)]
        limit: LimitArgs,
    },
    /// Print a shell completion script (bash, zsh, fish, powershell, elvish).
    Completions { shell: Option<String> },
    #[command(subcommand)]
    Extension(ExtensionCmd),
    /// Extension or unknown command: exec `gitee-<name>` from PATH.
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

#[derive(clap::Args, Clone, Debug)]
pub struct ApiArgs {
    /// API path (e.g. `user`, `/repos/oschina/gitfy/releases`).
    pub endpoint: String,
    /// HTTP method (default: GET, or POST when fields or `--input` are given).
    #[arg(short = 'X', long = "method")]
    pub method: Option<String>,
    /// Form field (`key=value`). Repeatable. For urlencoded requests `-F` and
    /// `-f` behave the same (both pass string pairs); the typed-vs-raw
    /// distinction only matters for JSON bodies, which this CLI does not build.
    #[arg(short = 'F', long = "field")]
    pub fields: Vec<String>,
    /// Raw form field (`key=value`). Repeatable; same encoding as `-F` here.
    #[arg(short = 'f', long = "raw-field")]
    pub raw_fields: Vec<String>,
    /// Extra request header (`Name: value`). Repeatable.
    #[arg(short = 'H', long = "header")]
    pub headers: Vec<String>,
    /// Raw request body from a file or `-` for stdin (conflicts with `-F`/`-f`).
    #[arg(long, conflicts_with_all = ["fields", "raw_fields"])]
    pub input: Option<String>,
    /// Walk `page`/`per_page` until an empty page and merge JSON arrays.
    #[arg(long)]
    pub paginate: bool,
}

#[derive(clap::Args, Clone, Debug)]
pub struct ListArgs {
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long, default_value_t = 30)]
    pub limit: usize,
}

#[derive(clap::Args, Clone, Debug)]
pub struct LimitArgs {
    #[arg(long, default_value_t = 30)]
    pub limit: usize,
}

#[derive(clap::Args, Clone, Debug)]
pub struct CommentArgs {
    #[arg(long, short = 'm')]
    pub body: String,
}

#[derive(Subcommand, Clone)]
pub enum PrCmd {
    List {
        #[command(flatten)]
        list: ListArgs,
        #[arg(long)]
        author: Option<String>,
    },
    /// Show open PRs relevant to you: created, assigned, awaiting your test.
    Status {
        #[command(flatten)]
        limit: LimitArgs,
    },
    /// Show details of a pull request.
    View {
        number: i64,
        /// Open in a browser instead of printing.
        #[arg(long)]
        web: bool,
    },
    /// Show the unified diff of a pull request.
    Diff {
        number: i64,
    },
    /// Fetch and check out a pull request branch locally.
    Checkout {
        number: i64,
    },
    /// Create a pull request. --fill derives title/body from commits; without
    /// --body/--fill, the repo's pull request template prefills the body.
    Create {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        head: Option<String>,
        #[arg(long)]
        base: Option<String>,
        /// Fill title (first commit subject) and body (commit list) from the
        /// base..head commit range via local git.
        #[arg(long)]
        fill: bool,
        /// Assign a user (repeatable, gh-style).
        #[arg(long)]
        assignee: Vec<String>,
        /// Assign a tester (repeatable).
        #[arg(long)]
        tester: Vec<String>,
        /// Add labels (repeatable; each may be comma-separated).
        #[arg(long)]
        label: Vec<String>,
        /// Milestone by number or exact title (resolved via the milestones API).
        #[arg(long)]
        milestone: Option<String>,
        /// Link an issue (ident, e.g. I1AB2C) that merging this PR closes.
        #[arg(long = "close-issue")]
        close_issue: Option<String>,
    },
    /// Edit a pull request's metadata. At least one flag is required.
    #[command(group = clap::ArgGroup::new("edit_flags").required(true).multiple(true).args(["title", "body", "assignee", "tester", "label", "milestone"]))]
    Edit {
        number: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
        /// Assign a user (repeatable, gh-style). Sent as `assignees` (names per
        /// the PR create endpoint; not listed on the PATCH swagger).
        #[arg(long)]
        assignee: Vec<String>,
        /// Assign a tester (repeatable). Sent as `testers` (same caveat as --assignee).
        #[arg(long)]
        tester: Vec<String>,
        /// Set labels (repeatable; each may be comma-separated). Replaces existing labels.
        #[arg(long)]
        label: Vec<String>,
        /// Milestone by number or exact title (resolved via the milestones API).
        #[arg(long)]
        milestone: Option<String>,
    },
    Merge {
        number: i64,
        #[arg(long)]
        squash: bool,
        #[arg(long)]
        rebase: bool,
        #[arg(long = "no-close-issue")]
        no_close_issue: bool,
    },
    /// Comment on a pull request.
    #[command(subcommand)]
    Comment(PrCommentCmd),
    Approve {
        number: i64,
        #[arg(long)]
        force: bool,
    },
    /// Mark a pull request as tested (测试通过). Gitee-specific: approve covers 审查, test covers 测试.
    Test {
        number: i64,
        #[arg(long)]
        force: bool,
    },
    Close {
        number: i64,
    },
    /// Reopen a closed pull request.
    Reopen {
        number: i64,
    },
    Link {
        number: i64,
        issue: String,
    },
    // pr update-branch omitted: no such endpoint in the v5 swagger (verified 2026-07-18).
}

#[derive(Subcommand, Clone)]
pub enum PrCommentCmd {
    /// Create a comment on a pull request.
    Create {
        number: i64,
        #[command(flatten)]
        body: CommentArgs,
    },
    /// List comments on a pull request.
    List {
        number: i64,
        /// Filter by type: `diff` (line/positional) or `general` (maps to Gitee
        /// `diff_comment` / `pr_comment`).
        #[arg(long = "type", value_parser = ["diff", "general"])]
        comment_type: Option<String>,
        #[command(flatten)]
        limit: LimitArgs,
    },
    /// Edit a comment by id, or the current user's latest on a PR via `--last`.
    Edit {
        /// Comment id, or PR number when `--last` is set.
        target: i64,
        /// Edit the authenticated user's most-recent comment on the PR
        /// (`target` is the PR number).
        #[arg(long)]
        last: bool,
        #[arg(long, short = 'm')]
        body: Option<String>,
    },
}


#[derive(Subcommand, Clone)]
pub enum SearchCmd {
    /// Search repositories.
    Repos {
        query: String,
        #[arg(long)]
        owner: Option<String>,
        #[arg(long)]
        language: Option<String>,
        /// Only include forked repositories.
        #[arg(long)]
        fork: bool,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        order: Option<String>,
        #[command(flatten)]
        limit: LimitArgs,
    },
    /// Search issues (globally unless --repo is set).
    Issues {
        query: String,
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        label: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        order: Option<String>,
        #[command(flatten)]
        limit: LimitArgs,
    },
    /// Search users.
    Users {
        query: String,
        #[arg(long)]
        sort: Option<String>,
        #[arg(long)]
        order: Option<String>,
        #[command(flatten)]
        limit: LimitArgs,
    },
}

#[derive(Subcommand, Clone)]
pub enum IssueCmd {
    List {
        #[command(flatten)]
        list: ListArgs,
        #[arg(long)]
        assignee: Option<String>,
    },
    /// Show open issues relevant to you: created by me, assigned to me.
    Status {
        #[command(flatten)]
        limit: LimitArgs,
    },
    /// Show details of an issue.
    View {
        number: String,
        /// Open in a browser instead of printing.
        #[arg(long)]
        web: bool,
    },
    Create {
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        labels: Option<String>,
        /// Milestone by number or exact title (resolved via the milestones API).
        #[arg(long)]
        milestone: Option<String>,
        /// Mark the issue as a security hole (Gitee-specific).
        #[arg(long)]
        security_hole: bool,
    },
    /// Edit an issue's metadata. At least one flag is required.
    ///
    /// State changes use PATCH `/repos/{owner}/issues/{number}` with a JSON
    /// body `{repo, title, state}` (current title must be echoed). Do not use
    /// form fields on `/repos/{owner}/{repo}/issues/{number}` — that path
    /// often returns an opaque `project or enterprise` 404.
    #[command(group = clap::ArgGroup::new("edit_flags").required(true).multiple(true).args(["title", "body", "assignee", "label", "milestone", "security_hole", "state"]))]
    Edit {
        number: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        /// Set labels (repeatable; each may be comma-separated). Replaces existing labels.
        #[arg(long)]
        label: Vec<String>,
        /// Milestone by number or exact title (resolved via the milestones API).
        #[arg(long)]
        milestone: Option<String>,
        /// Mark the issue as a security hole (Gitee-specific).
        #[arg(long)]
        security_hole: bool,
        /// Lifecycle state: `open`, `progressing`, `closed`, or `rejected`.
        #[arg(long, value_parser = ["open", "progressing", "closed", "rejected"])]
        state: Option<String>,
    },
    Close {
        number: String,
    },
    /// Reopen a closed issue.
    Reopen {
        number: String,
    },
    Link {
        number: String,
        pr: i64,
    },
    /// Comment on an issue.
    #[command(subcommand)]
    Comment(IssueCommentCmd),
}

#[derive(Subcommand, Clone)]
pub enum IssueCommentCmd {
    /// Create a comment on an issue.
    Create {
        number: String,
        #[command(flatten)]
        body: CommentArgs,
    },
    /// List comments on an issue.
    List {
        number: String,
        #[command(flatten)]
        limit: LimitArgs,
    },
    /// Edit a comment by id, or the current user's latest on an issue via `--last`.
    Edit {
        /// Comment id, or issue number when `--last` is set.
        target: String,
        /// Edit the authenticated user's most-recent comment on the issue
        /// (`target` is the issue number).
        #[arg(long)]
        last: bool,
        #[arg(long, short = 'm')]
        body: Option<String>,
    },
}


#[derive(Subcommand, Clone)]
pub enum GistCmd {
    List {
        #[command(flatten)]
        limit: LimitArgs,
    },
    View {
        id: String,
        #[arg(long)]
        raw: bool,
    },
    /// Create a gist from one or more files. Use `-` to read from stdin
    /// (requires `--filename`). The Gitee API requires a description (1–30
    /// chars); when `--desc` is omitted it defaults to the first file name.
    Create {
        #[arg(long)]
        desc: Option<String>,
        #[arg(long)]
        public: bool,
        #[arg(long)]
        filename: Option<String>,
        #[arg(required = true, num_args = 1..)]
        files: Vec<String>,
    },
    Edit {
        id: String,
        file: String,
    },
    Delete {
        id: String,
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand, Clone)]
pub enum ReleaseCmd {
    List {
        #[command(flatten)]
        limit: LimitArgs,
    },
    View {
        tag: String,
        /// Open in a browser instead of printing.
        #[arg(long)]
        web: bool,
    },
    Create {
        #[arg(long)]
        tag: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        prerelease: bool,
    },
    Upload {
        tag: String,
        files: Vec<String>,
    },
    Download {
        tag: String,
        #[arg(long, default_value = ".")]
        dir: String,
        /// Download only assets matching the glob (`*` and `?` supported;
        /// character classes like `[ab]` are not).
        #[arg(long)]
        pattern: Option<String>,
    },
    /// Edit a release. At least one flag is required.
    #[command(group = clap::ArgGroup::new("edit_flags").required(true).multiple(true).args(["name", "notes", "prerelease"]))]
    Edit {
        tag: String,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        #[arg(long)]
        prerelease: bool,
    },
    Delete {
        tag: String,
        #[arg(long)]
        yes: bool,
    },
}

/// Repo subcommands. `repo sync` omitted: no fork-synchronize endpoint in v5 swagger (verified 2026-07-18).
#[derive(Subcommand, Clone)]
pub enum LabelCmd {
    List {
        #[command(flatten)]
        limit: LimitArgs,
    },
    Create {
        name: String,
        #[arg(long)]
        color: String,
        // Ticket asked for --description but Gitee v5 POST /repos/{owner}/{repo}/labels
        // has no description param (swagger 2026-07-18).
    },
    /// Edit a label. At least one flag is required.
    #[command(group = clap::ArgGroup::new("edit_flags").required(true).multiple(true).args(["new_name", "color"]))]
    Edit {
        name: String,
        #[arg(long = "name")]
        new_name: Option<String>,
        #[arg(long)]
        color: Option<String>,
    },
    Delete {
        name: String,
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand, Clone)]
pub enum RepoCmd {
    /// Show repository details. Targets the resolved repo unless given owner/name.
    View {
        #[arg(value_name = "REPO")]
        target: Option<String>,
        /// Open in a browser instead of printing.
        #[arg(long)]
        web: bool,
    },
    /// List repositories. Bare lists the authenticated user's; with an arg,
    /// lists that user/org's public repos.
    List {
        owner: Option<String>,
        #[command(flatten)]
        limit: LimitArgs,
    },
    /// Clone a repository via git.
    Clone {
        /// owner/name or a full Gitee URL.
        spec: String,
        /// Local directory to clone into.
        dir: Option<String>,
        /// Use the SSH URL instead of HTTPS.
        #[arg(long)]
        ssh: bool,
    },
    /// Fork the resolved repository into your account.
    Fork {
        /// After forking, add the new repo as a git remote with this name.
        #[arg(long = "add-remote")]
        add_remote: Option<String>,
    },
    /// Create a repository under your account or an organization.
    Create {
        name: String,
        /// Create under this organization (POST /orgs/{org}/repos).
        #[arg(long)]
        org: Option<String>,
        #[arg(long)]
        private: bool,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        homepage: Option<String>,
        #[arg(long = "gitignore")]
        gitignore: Option<String>,
        #[arg(long)]
        license: Option<String>,
    },
    /// Edit repository settings. At least one flag is required.
    #[command(group = clap::ArgGroup::new("edit_flags").required(true).multiple(true).args(["description", "homepage", "private", "public", "default_branch"]))]
    Edit {
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        homepage: Option<String>,
        #[arg(long, conflicts_with = "public")]
        private: bool,
        #[arg(long, conflicts_with = "private")]
        public: bool,
        #[arg(long = "default-branch")]
        default_branch: Option<String>,
    },
    /// Rename a repository's URL slug (`path` on the API).
    Rename {
        new_path: String,
    },
        /// Star the resolved repository.
    Star,
    /// Unstar the resolved repository.
    Unstar,
    /// Watch (subscribe to) the resolved repository.
    Watch,
    /// Unwatch the resolved repository.
    Unwatch,
    /// Delete a repository.
    Delete {
        #[arg(long)]
        yes: bool,
    },
}


#[derive(Subcommand, Clone)]
pub enum MilestoneCmd {
    List {
        #[command(flatten)]
        list: ListArgs,
    },
    /// Show details of a milestone.
    View {
        number: i64,
    },
    Create {
        #[arg(long)]
        title: String,
        /// Due date (required by the Gitee API — POST without `due_on` returns 400).
        /// Accepted format: `YYYY-MM-DD`.
        #[arg(long = "due-on")]
        due_on: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, value_parser = ["open", "closed"])]
        state: Option<String>,
    },
    /// Edit a milestone. At least one flag is required.
    #[command(group = clap::ArgGroup::new("milestone_edit_flags").required(true).multiple(true).args(["title", "due_on", "description", "state"]))]
    Edit {
        number: i64,
        #[arg(long)]
        title: Option<String>,
        #[arg(long = "due-on")]
        due_on: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long, value_parser = ["open", "closed"])]
        state: Option<String>,
    },
}



#[derive(Subcommand, Clone)]
pub enum ConfigCmd {
    List,
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BuildKind {
    Cargo,
    Npm,
}

#[derive(Subcommand, Clone)]
pub enum ExtensionCmd {
    /// List extension commands discovered on PATH and in the managed dir.
    List,
    /// Clone a Gitee repo into the managed extensions dir and (optionally) build it.
    Install {
        /// `owner/repo` (or a full Gitee URL) of the extension to install.
        repo: String,
        /// Build system to run after cloning: `cargo` or `npm`.
        #[arg(long, value_parser = ["cargo", "npm"])]
        build: Option<String>,
        /// Skip the trust confirmation prompt.
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Scaffold a new extension project in the current directory.
    Create {
        /// Extension name (without the `gitee-` prefix).
        name: String,
        /// Scaffold a Rust extension (Cargo.toml + src/main.rs) instead of a shell script.
        #[arg(long)]
        cargo: bool,
    },
    /// Remove an installed extension from the managed dir.
    Remove {
        name: String,
        #[arg(long, short = 'y')]
        yes: bool,
    },
    /// Pull (and rebuild, if needed) installed extensions. With no name: all.
    Upgrade {
        name: Option<String>,
    },
}

#[derive(Subcommand, Clone)]
pub enum AliasCmd {
    List,
    Set {
        name: String,
        /// Expansion words (joined with spaces). Prefer quoting in the shell.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        expansion: Vec<String>,
    },
    Delete {
        name: String,
    },
}

#[derive(Subcommand, Clone)]
pub enum OrgCmd {
    List {
        #[command(flatten)]
        limit: LimitArgs,
    },
}

#[derive(Subcommand, Clone)]
pub enum SshKeyCmd {
    List {
        #[command(flatten)]
        limit: LimitArgs,
    },
    Add {
        /// Path to the public key file.
        pubkey_file: String,
        #[arg(long)]
        title: Option<String>,
    },
    Delete {
        id: i64,
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand, Clone)]
pub enum CollaboratorCmd {
    List {
        #[command(flatten)]
        limit: LimitArgs,
    },
    Add {
        username: String,
        /// Permission: pull | push | admin (English enums).
        #[arg(long, default_value = "push")]
        permission: String,
    },
    Remove {
        username: String,
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand, Clone)]
pub enum WebhookCmd {
    List {
        #[command(flatten)]
        limit: LimitArgs,
    },
    Create {
        #[arg(long)]
        url: String,
        /// Event flags (comma-separated or repeatable): push_events, tag_push_events,
        /// issues_events, merge_requests_events, note_events.
        #[arg(long)]
        events: Vec<String>,
        #[arg(long)]
        password: Option<String>,
    },
    Delete {
        id: i64,
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand, Clone)]
pub enum AuthCmd {
    /// Store a personal access token (validated against the API unless --force).
    Login {
        #[arg(long)]
        token: Option<String>,
        /// Skip the token-validation probe.
        #[arg(long)]
        force: bool,
    },
    /// Show whether you are logged in and where the token is stored.
    Status,
    /// Print the active token (e.g. to pipe into another tool).
    Token,
    /// Forget the stored token for the current host.
    Logout,
    /// Configure git to use gitee as a credential helper for this host.
    SetupGit,
    /// Switch the active user for this host.
    Switch {
        #[arg(long)]
        user: String,
    },
    /// Git credential-helper protocol (usually invoked by git, not humans).
    #[command(subcommand)]
    GitCredential(GitCredentialCmd),
}

#[derive(Subcommand, Clone)]
pub enum GitCredentialCmd {
    Get,
    Store,
    Erase,
}

#[cfg(test)]
mod parse_tests {
    use super::{AliasCmd, AuthCmd, Cli, CollaboratorCmd, Command, ConfigCmd, ExtensionCmd, GistCmd, GitCredentialCmd, IssueCmd, IssueCommentCmd, MilestoneCmd, OrgCmd, PrCmd, PrCommentCmd, ReleaseCmd, RepoCmd, SshKeyCmd, WebhookCmd};
    use clap::Parser;

    #[test]
    fn pr_create_parses_without_title_for_interactive() {
        let cli = Cli::try_parse_from(["gitee", "pr", "create"])
            .expect("pr create without flags should parse (interactive or usage at runtime)");
        let Command::Pr(PrCmd::Create { title, fill, .. }) = cli.cmd else {
            panic!("expected pr create");
        };
        assert!(title.is_none());
        assert!(!fill);

        let cli = Cli::try_parse_from(["gitee", "pr", "create", "--fill"])
            .expect("--fill alone should parse");
        let Command::Pr(PrCmd::Create { title, fill, .. }) = cli.cmd else {
            panic!("expected pr create");
        };
        assert!(title.is_none());
        assert!(fill);
    }

    #[test]
    fn issue_create_parses_without_title_for_interactive() {
        let cli = Cli::try_parse_from(["gitee", "issue", "create"])
            .expect("issue create without flags should parse (interactive or usage at runtime)");
        let Command::Issue(IssueCmd::Create { title, .. }) = cli.cmd else {
            panic!("expected issue create");
        };
        assert!(title.is_none());
    }

    #[test]
    fn search_requires_query_positional() {
        assert!(Cli::try_parse_from(["gitee", "search", "repos"]).is_err());
        assert!(Cli::try_parse_from(["gitee", "search", "issues"]).is_err());
        assert!(Cli::try_parse_from(["gitee", "search", "users"]).is_err());
    }

    #[test]
    fn search_parses_flags() {
        let cli = Cli::try_parse_from([
            "gitee", "search", "repos", "gitee",
            "--owner", "oschina",
            "--language", "Rust",
            "--fork",
            "--sort", "stars_count",
            "--order", "desc",
            "--limit", "5",
        ])
        .expect("search repos should parse");
        let Command::Search(super::SearchCmd::Repos {
            query,
            owner,
            language,
            fork,
            sort,
            order,
            limit,
        }) = cli.cmd
        else {
            panic!("expected search repos");
        };
        assert_eq!(query, "gitee");
        assert_eq!(owner.as_deref(), Some("oschina"));
        assert_eq!(language.as_deref(), Some("Rust"));
        assert!(fork);
        assert_eq!(sort.as_deref(), Some("stars_count"));
        assert_eq!(order.as_deref(), Some("desc"));
        assert_eq!(limit.limit, 5);

        let cli = Cli::try_parse_from([
            "gitee", "search", "issues", "login",
            "--state", "open",
            "--author", "alice",
            "--assignee", "bob",
            "--label", "bug",
            "--language", "Go",
            "--sort", "updated_at",
            "--order", "asc",
            "--limit", "3",
        ])
        .expect("search issues should parse");
        let Command::Search(super::SearchCmd::Issues {
            query,
            state,
            author,
            assignee,
            label,
            language,
            sort,
            order,
            limit,
        }) = cli.cmd
        else {
            panic!("expected search issues");
        };
        assert_eq!(query, "login");
        assert_eq!(state.as_deref(), Some("open"));
        assert_eq!(author.as_deref(), Some("alice"));
        assert_eq!(assignee.as_deref(), Some("bob"));
        assert_eq!(label.as_deref(), Some("bug"));
        assert_eq!(language.as_deref(), Some("Go"));
        assert_eq!(sort.as_deref(), Some("updated_at"));
        assert_eq!(order.as_deref(), Some("asc"));
        assert_eq!(limit.limit, 3);

        let cli = Cli::try_parse_from([
            "gitee", "search", "users", "kip",
            "--sort", "followers_count",
            "--order", "desc",
            "--limit", "3",
        ])
        .expect("search users should parse");
        let Command::Search(super::SearchCmd::Users {
            query,
            sort,
            order,
            limit,
        }) = cli.cmd
        else {
            panic!("expected search users");
        };
        assert_eq!(query, "kip");
        assert_eq!(sort.as_deref(), Some("followers_count"));
        assert_eq!(order.as_deref(), Some("desc"));
        assert_eq!(limit.limit, 3);
    }

    #[test]
    fn search_issues_accepts_global_repo_flag() {
        let cli = Cli::try_parse_from([
            "gitee",
            "--repo",
            "oschina/gitee-cli",
            "search",
            "issues",
            "bug",
            "--limit",
            "3",
        ])
        .expect("search issues with global --repo should parse");
        assert_eq!(cli.repo.as_deref(), Some("oschina/gitee-cli"));
        let Command::Search(super::SearchCmd::Issues { query, limit, .. }) = cli.cmd else {
            panic!("expected search issues");
        };
        assert_eq!(query, "bug");
        assert_eq!(limit.limit, 3);
    }


    #[test]
    fn pr_create_parses_full_flag_surface() {
        let cli = Cli::try_parse_from([
            "gitee", "pr", "create",
            "--title", "T",
            "--assignee", "me",
            "--tester", "qa1",
            "--label", "bug,ui",
            "--milestone", "v1.0",
            "--close-issue", "I1AB2C",
        ])
        .expect("pr create should parse");
        let Command::Pr(PrCmd::Create {
            assignee,
            tester,
            label,
            milestone,
            close_issue,
            ..
        }) = cli.cmd
        else {
            panic!("expected pr create");
        };
        assert_eq!(assignee, vec!["me".to_string()]);
        assert_eq!(tester, vec!["qa1".to_string()]);
        assert_eq!(label, vec!["bug,ui".to_string()]);
        assert_eq!(milestone.as_deref(), Some("v1.0"));
        assert_eq!(close_issue.as_deref(), Some("I1AB2C"));
    }

    #[test]
    fn release_edit_requires_at_least_one_flag() {
        let r = Cli::try_parse_from(["gitee", "release", "edit", "v1.0"]);
        assert!(r.is_err(), "release edit with no flags must fail");
    }

    #[test]
    fn release_download_parses_flags() {
        let cli = Cli::try_parse_from([
            "gitee",
            "release",
            "download",
            "v1.0",
            "--dir",
            "/tmp/out",
            "--pattern",
            "*.tar.xz",
        ])
        .expect("release download should parse");
        let Command::Release(ReleaseCmd::Download { tag, dir, pattern }) = cli.cmd else {
            panic!("expected release download");
        };
        assert_eq!(tag, "v1.0");
        assert_eq!(dir, "/tmp/out");
        assert_eq!(pattern.as_deref(), Some("*.tar.xz"));
    }

    #[test]
    fn issue_edit_requires_at_least_one_flag() {
        let r = Cli::try_parse_from(["gitee", "issue", "edit", "I1AB"]);
        assert!(r.is_err(), "issue edit with no flags must fail");
    }

    #[test]
    fn issue_edit_state_alone_parses() {
        let cli = Cli::try_parse_from(["gitee", "issue", "edit", "I1AB", "--state", "progressing"])
            .expect("issue edit --state alone should parse");
        let Command::Issue(IssueCmd::Edit { number, state, .. }) = cli.cmd else {
            panic!("expected issue edit");
        };
        assert_eq!(number, "I1AB");
        assert_eq!(state.as_deref(), Some("progressing"));
    }

    #[test]
    fn gist_create_requires_at_least_one_file() {
        let r = Cli::try_parse_from(["gitee", "gist", "create"]);
        assert!(r.is_err(), "gist create with no files must fail");
    }

    #[test]
    fn gist_flags_parse() {
        let cli = Cli::try_parse_from([
            "gitee", "gist", "create", "a.txt",
            "--desc", "my snippet",
            "--public",
            "--filename", "b.txt",
        ])
        .expect("gist create should parse");
        let Command::Gist(GistCmd::Create {
            files,
            desc,
            public,
            filename,
            ..
        }) = cli.cmd
        else {
            panic!("expected gist create");
        };
        assert_eq!(files, vec!["a.txt".to_string()]);
        assert_eq!(desc.as_deref(), Some("my snippet"));
        assert!(public);
        assert_eq!(filename.as_deref(), Some("b.txt"));

        let cli = Cli::try_parse_from(["gitee", "gist", "delete", "abc123", "--yes"])
            .expect("gist delete should parse");
        let Command::Gist(GistCmd::Delete { id, yes }) = cli.cmd else {
            panic!("expected gist delete");
        };
        assert_eq!(id, "abc123");
        assert!(yes);
    }


    #[test]
    fn issue_edit_and_create_parse_new_flags() {
        let cli = Cli::try_parse_from([
            "gitee", "issue", "edit", "I1AB",
            "--title", "Retitle",
            "--label", "bug",
            "--assignee", "dev1",
            "--milestone", "v1.0",
            "--security-hole",
        ])
        .expect("issue edit should parse");
        let Command::Issue(IssueCmd::Edit {
            number,
            title,
            security_hole,
            ..
        }) = cli.cmd
        else {
            panic!("expected issue edit");
        };
        assert_eq!(number, "I1AB");
        assert_eq!(title.as_deref(), Some("Retitle"));
        assert!(security_hole);

        let cli = Cli::try_parse_from([
            "gitee", "issue", "create",
            "--title", "T",
            "--milestone", "3",
            "--security-hole",
        ])
        .expect("issue create should parse");
        let Command::Issue(IssueCmd::Create {
            milestone,
            security_hole,
            ..
        }) = cli.cmd
        else {
            panic!("expected issue create");
        };
        assert_eq!(milestone.as_deref(), Some("3"));
        assert!(security_hole);
    }

    #[test]
    fn repo_edit_requires_at_least_one_flag() {
        let r = Cli::try_parse_from(["gitee", "repo", "edit"]);
        assert!(r.is_err(), "repo edit with no flags must fail");
    }

    #[test]
    fn repo_edit_private_public_conflict() {
        let r = Cli::try_parse_from(["gitee", "repo", "edit", "--private", "--public"]);
        assert!(r.is_err(), "--private and --public must conflict");
    }

    #[test]
    fn repo_create_flags_parse() {
        let cli = Cli::try_parse_from([
            "gitee",
            "repo",
            "create",
            "my-repo",
            "--org",
            "acme",
            "--private",
            "--description",
            "desc",
            "--homepage",
            "https://example.com",
            "--gitignore",
            "Rust",
            "--license",
            "MIT",
        ])
        .expect("repo create should parse");
        let Command::Repo(RepoCmd::Create {
            name,
            org,
            private,
            description,
            homepage,
            gitignore,
            license,
        }) = cli.cmd
        else {
            panic!("expected repo create");
        };
        assert_eq!(name, "my-repo");
        assert_eq!(org.as_deref(), Some("acme"));
        assert!(private);
        assert_eq!(description.as_deref(), Some("desc"));
        assert_eq!(homepage.as_deref(), Some("https://example.com"));
        assert_eq!(gitignore.as_deref(), Some("Rust"));
        assert_eq!(license.as_deref(), Some("MIT"));
    }

    #[test]
    fn pr_edit_requires_at_least_one_flag() {
        let r = Cli::try_parse_from(["gitee", "pr", "edit", "5"]);
        assert!(r.is_err(), "pr edit with no flags must fail");
    }

    #[test]
    fn pr_test_parses() {
        let cli = Cli::try_parse_from(["gitee", "pr", "test", "12"])
            .expect("pr test should parse");
        let Command::Pr(PrCmd::Test { number, force }) = cli.cmd else {
            panic!("expected pr test");
        };
        assert_eq!(number, 12);
        assert!(!force);

        let cli = Cli::try_parse_from(["gitee", "pr", "test", "12", "--force"])
            .expect("pr test --force should parse");
        let Command::Pr(PrCmd::Test { number, force }) = cli.cmd else {
            panic!("expected pr test");
        };
        assert_eq!(number, 12);
        assert!(force);
    }

    #[test]
    fn label_edit_requires_at_least_one_flag() {
        let r = Cli::try_parse_from(["gitee", "label", "edit", "bug"]);
        assert!(r.is_err(), "label edit with no flags must fail");
    }

    #[test]
    fn label_create_requires_color() {
        let r = Cli::try_parse_from(["gitee", "label", "create", "bug"]);
        assert!(r.is_err(), "label create without --color must fail");
    }

    #[test]
    fn pr_edit_parses_accumulating_flags() {
        let cli = Cli::try_parse_from([
            "gitee", "pr", "edit", "5",
            "--title", "New title",
            "--label", "a,b",
            "--label", "c",
            "--assignee", "dev1",
            "--tester", "qa1",
            "--milestone", "v1.0",
        ])
        .expect("pr edit should parse");
        let Command::Pr(PrCmd::Edit {
            number,
            title,
            label,
            assignee,
            tester,
            milestone,
            ..
        }) = cli.cmd
        else {
            panic!("expected pr edit, got {:?}", std::mem::discriminant(&cli.cmd));
        };
        assert_eq!(number, 5);
        assert_eq!(title.as_deref(), Some("New title"));
        assert_eq!(label, vec!["a,b".to_string(), "c".to_string()]);
        assert_eq!(assignee, vec!["dev1".to_string()]);
        assert_eq!(tester, vec!["qa1".to_string()]);
        assert_eq!(milestone.as_deref(), Some("v1.0"));
    }

    #[test]
    fn jq_without_json_is_a_usage_error() {
        let r = Cli::try_parse_from(["gitee", "pr", "list", "--jq", ".[0]"]);
        assert!(r.is_err(), "--jq without --json must fail");
    }

    #[test]
    fn milestone_create_requires_title_and_due_on() {
        assert!(Cli::try_parse_from(["gitee", "milestone", "create"]).is_err());
        assert!(Cli::try_parse_from(["gitee", "milestone", "create", "--title", "T"]).is_err());
        let cli = Cli::try_parse_from([
            "gitee", "milestone", "create", "--title", "T", "--due-on", "2026-12-31",
        ])
        .expect("milestone create should parse");
        let Command::Milestone(MilestoneCmd::Create { title, due_on, .. }) = cli.cmd else {
            panic!("expected milestone create");
        };
        assert_eq!(title, "T");
        assert_eq!(due_on, "2026-12-31");
    }

    #[test]
    fn milestone_edit_requires_at_least_one_flag() {
        let r = Cli::try_parse_from(["gitee", "milestone", "edit", "1"]);
        assert!(r.is_err(), "milestone edit with no flags must fail");
    }


    #[test]
    fn jq_parses_after_bare_json_flag() {
        let cli = Cli::try_parse_from(["gitee", "pr", "list", "--json", "--jq", ".[0].title"])
            .expect("--json --jq should parse");
        assert_eq!(cli.json.as_deref(), Some(""));
        assert_eq!(cli.jq.as_deref(), Some(".[0].title"));
    }

    #[test]
    fn jq_parses_after_json_field_projection() {
        let cli = Cli::try_parse_from([
            "gitee",
            "issue",
            "list",
            "--json",
            "number,title",
            "--jq",
            "map(.number)",
        ])
        .expect("--json fields --jq should parse");
        assert_eq!(cli.json.as_deref(), Some("number,title"));
        assert_eq!(cli.jq.as_deref(), Some("map(.number)"));
    }

    #[test]
    fn status_parses() {
        let cli = Cli::try_parse_from(["gitee", "status"]).expect("status should parse");
        let Command::Status { limit } = cli.cmd else {
            panic!("expected status");
        };
        assert_eq!(limit.limit, 30);

        let cli = Cli::try_parse_from(["gitee", "status", "--limit", "5"])
            .expect("status with limit should parse");
        let Command::Status { limit } = cli.cmd else {
            panic!("expected status with limit");
        };
        assert_eq!(limit.limit, 5);
    }

    #[test]
    fn pr_status_parses() {
        let cli = Cli::try_parse_from(["gitee", "pr", "status"]).expect("pr status should parse");
        let Command::Pr(PrCmd::Status { limit }) = cli.cmd else {
            panic!("expected pr status");
        };
        assert_eq!(limit.limit, 30);

        let cli = Cli::try_parse_from(["gitee", "pr", "status", "--limit", "5"])
            .expect("pr status --limit should parse");
        let Command::Pr(PrCmd::Status { limit }) = cli.cmd else {
            panic!("expected pr status");
        };
        assert_eq!(limit.limit, 5);
    }

    #[test]
    fn issue_status_parses() {
        let cli = Cli::try_parse_from(["gitee", "issue", "status"])
            .expect("issue status should parse");
        let Command::Issue(IssueCmd::Status { limit }) = cli.cmd else {
            panic!("expected issue status");
        };
        assert_eq!(limit.limit, 30);

        let cli = Cli::try_parse_from(["gitee", "issue", "status", "--limit", "5"])
            .expect("issue status --limit should parse");
        let Command::Issue(IssueCmd::Status { limit }) = cli.cmd else {
            panic!("expected issue status");
        };
        assert_eq!(limit.limit, 5);
    }

    #[test]
    fn org_list_parses_limit() {
        let cli = Cli::try_parse_from(["gitee", "org", "list", "--limit", "5"]).expect("org list");
        let Command::Org(OrgCmd::List { limit }) = cli.cmd else { panic!("expected org list") };
        assert_eq!(limit.limit, 5);
    }

    #[test]
    fn ssh_key_commands_parse() {
        let cli = Cli::try_parse_from(["gitee", "ssh-key", "list"]).expect("ssh-key list");
        assert!(matches!(cli.cmd, Command::SshKey(SshKeyCmd::List { .. })));

        let cli = Cli::try_parse_from([
            "gitee", "ssh-key", "add", "~/.ssh/id_ed25519.pub", "--title", "laptop",
        ]).expect("ssh-key add");
        let Command::SshKey(SshKeyCmd::Add { pubkey_file, title }) = cli.cmd else { panic!("add") };
        assert_eq!(pubkey_file, "~/.ssh/id_ed25519.pub");
        assert_eq!(title.as_deref(), Some("laptop"));

        let cli = Cli::try_parse_from(["gitee", "ssh-key", "delete", "99", "--yes"]).expect("delete");
        let Command::SshKey(SshKeyCmd::Delete { id, yes }) = cli.cmd else { panic!("delete") };
        assert_eq!(id, 99);
        assert!(yes);
    }

    #[test]
    fn collaborator_commands_parse() {
        let cli = Cli::try_parse_from([
            "gitee", "collaborator", "add", "alice", "--permission", "admin",
        ]).expect("collaborator add");
        let Command::Collaborator(CollaboratorCmd::Add { username, permission }) = cli.cmd else { panic!("add") };
        assert_eq!(username, "alice");
        assert_eq!(permission, "admin");

        let cli = Cli::try_parse_from(["gitee", "collaborator", "remove", "alice", "-y"]).expect("remove");
        let Command::Collaborator(CollaboratorCmd::Remove { username, yes }) = cli.cmd else { panic!("remove") };
        assert_eq!(username, "alice");
        assert!(yes);
    }

    #[test]
    fn webhook_commands_parse() {
        let cli = Cli::try_parse_from([
            "gitee", "webhook", "create",
            "--url", "https://example.com/hook",
            "--events", "push_events,issues_events",
            "--password", "s3cret",
        ]).expect("webhook create");
        let Command::Webhook(WebhookCmd::Create { url, events, password }) = cli.cmd else { panic!("create") };
        assert_eq!(url, "https://example.com/hook");
        assert_eq!(events, vec!["push_events,issues_events".to_string()]);
        assert_eq!(password.as_deref(), Some("s3cret"));

        let cli = Cli::try_parse_from(["gitee", "webhook", "delete", "55", "--yes"]).expect("delete");
        let Command::Webhook(WebhookCmd::Delete { id, yes }) = cli.cmd else { panic!("delete") };
        assert_eq!(id, 55);
        assert!(yes);

        let cli = Cli::try_parse_from([
            "gitee", "webhook", "create",
            "--url", "https://example.com/hook",
            "--events", "pull_requests_events",
        ]).expect("webhook alias event");
        let Command::Webhook(WebhookCmd::Create { events, .. }) = cli.cmd else { panic!("create") };
        assert_eq!(events, vec!["pull_requests_events".to_string()]);
    }

    #[test]
    fn repo_star_watch_parse() {
        let cli = Cli::try_parse_from(["gitee", "repo", "star"]).expect("star");
        assert!(matches!(cli.cmd, Command::Repo(RepoCmd::Star)));
        let cli = Cli::try_parse_from(["gitee", "repo", "unstar"]).expect("unstar");
        assert!(matches!(cli.cmd, Command::Repo(RepoCmd::Unstar)));
        let cli = Cli::try_parse_from(["gitee", "repo", "watch"]).expect("watch");
        assert!(matches!(cli.cmd, Command::Repo(RepoCmd::Watch)));
        let cli = Cli::try_parse_from(["gitee", "repo", "unwatch"]).expect("unwatch");
        assert!(matches!(cli.cmd, Command::Repo(RepoCmd::Unwatch)));
    }

    #[test]
    fn config_and_alias_parse() {
        let cli = Cli::try_parse_from(["gitee", "config", "set", "host", "gitee.com"]).unwrap();
        let Command::Config(ConfigCmd::Set { key, value }) = cli.cmd else { panic!("config set") };
        assert_eq!(key, "host");
        assert_eq!(value, "gitee.com");

        let cli = Cli::try_parse_from(["gitee", "alias", "set", "co", "pr", "checkout"]).unwrap();
        let Command::Alias(AliasCmd::Set { name, expansion }) = cli.cmd else { panic!("alias set") };
        assert_eq!(name, "co");
        assert_eq!(expansion, vec!["pr", "checkout"]);

        let cli = Cli::try_parse_from(["gitee", "alias", "delete", "co"]).unwrap();
        assert!(matches!(cli.cmd, Command::Alias(AliasCmd::Delete { .. })));
    }

    #[test]
    fn auth_setup_switch_credential_parse() {
        let cli = Cli::try_parse_from(["gitee", "auth", "setup-git"]).unwrap();
        assert!(matches!(cli.cmd, Command::Auth(AuthCmd::SetupGit)));
        let cli = Cli::try_parse_from(["gitee", "auth", "switch", "--user", "kip"]).unwrap();
        let Command::Auth(AuthCmd::Switch { user }) = cli.cmd else { panic!("switch") };
        assert_eq!(user, "kip");
        let cli = Cli::try_parse_from(["gitee", "auth", "git-credential", "get"]).unwrap();
        assert!(matches!(
            cli.cmd,
            Command::Auth(AuthCmd::GitCredential(GitCredentialCmd::Get))
        ));
    }

    #[test]
    fn extension_list_parse() {
        let cli = Cli::try_parse_from(["gitee", "extension", "list"]).unwrap();
        assert!(matches!(cli.cmd, Command::Extension(ExtensionCmd::List)));
    }

    #[test]
    fn extension_install_parse() {
        let cli = Cli::try_parse_from([
            "gitee", "extension", "install", "owner/my-ext", "--build", "cargo", "--yes",
        ])
        .unwrap();
        let Command::Extension(ExtensionCmd::Install { repo, build, yes }) = cli.cmd else {
            panic!("expected install");
        };
        assert_eq!(repo, "owner/my-ext");
        assert_eq!(build.as_deref(), Some("cargo"));
        assert!(yes);

        let cli = Cli::try_parse_from(["gitee", "extension", "install", "owner/ext"]).unwrap();
        let Command::Extension(ExtensionCmd::Install { build, yes, .. }) = cli.cmd else {
            panic!("expected install");
        };
        assert!(build.is_none());
        assert!(!yes);
    }

    #[test]
    fn extension_install_rejects_bad_build_value() {
        assert!(Cli::try_parse_from([
            "gitee", "extension", "install", "owner/ext", "--build", "go"
        ])
        .is_err());
    }

    #[test]
    fn extension_create_parse() {
        let cli = Cli::try_parse_from(["gitee", "extension", "create", "demo"]).unwrap();
        let Command::Extension(ExtensionCmd::Create { name, cargo }) = cli.cmd else {
            panic!("expected create");
        };
        assert_eq!(name, "demo");
        assert!(!cargo);

        let cli = Cli::try_parse_from(["gitee", "extension", "create", "demo", "--cargo"]).unwrap();
        let Command::Extension(ExtensionCmd::Create { cargo, .. }) = cli.cmd else {
            panic!("expected create");
        };
        assert!(cargo);
    }

    #[test]
    fn extension_remove_parse() {
        let cli = Cli::try_parse_from(["gitee", "extension", "remove", "demo", "--yes"]).unwrap();
        let Command::Extension(ExtensionCmd::Remove { name, yes }) = cli.cmd else {
            panic!("expected remove");
        };
        assert_eq!(name, "demo");
        assert!(yes);
    }

    #[test]
    fn extension_upgrade_parse() {
        let cli = Cli::try_parse_from(["gitee", "extension", "upgrade"]).unwrap();
        let Command::Extension(ExtensionCmd::Upgrade { name }) = cli.cmd else {
            panic!("expected upgrade");
        };
        assert!(name.is_none());

        let cli = Cli::try_parse_from(["gitee", "extension", "upgrade", "demo"]).unwrap();
        let Command::Extension(ExtensionCmd::Upgrade { name }) = cli.cmd else {
            panic!("expected upgrade");
        };
        assert_eq!(name.as_deref(), Some("demo"));
    }

    #[test]
    fn external_extension_parse() {
        let cli = Cli::try_parse_from(["gitee", "myext", "arg1", "--flag"]).unwrap();
        let Command::External(args) = cli.cmd else { panic!("external") };
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "myext");
        assert_eq!(args[1], "arg1");
        assert_eq!(args[2], "--flag");
    }

    #[test]
    fn browse_and_web_parse() {
        let cli = Cli::try_parse_from(["gitee", "browse"]).unwrap();
        assert!(matches!(cli.cmd, Command::Browse));
        let cli = Cli::try_parse_from(["gitee", "pr", "view", "12", "--web"]).unwrap();
        let Command::Pr(PrCmd::View { number, web }) = cli.cmd else { panic!("pr view") };
        assert_eq!(number, 12);
        assert!(web);
        let cli = Cli::try_parse_from(["gitee", "issue", "view", "I1", "--web"]).unwrap();
        let Command::Issue(IssueCmd::View { number, web }) = cli.cmd else { panic!("issue view") };
        assert_eq!(number, "I1");
        assert!(web);
        let cli = Cli::try_parse_from(["gitee", "release", "view", "v1", "--web"]).unwrap();
        let Command::Release(ReleaseCmd::View { tag, web }) = cli.cmd else { panic!("release view") };
        assert_eq!(tag, "v1");
        assert!(web);
        let cli = Cli::try_parse_from(["gitee", "repo", "view", "--web"]).unwrap();
        let Command::Repo(RepoCmd::View { web, .. }) = cli.cmd else { panic!("repo view") };
        assert!(web);
    }

    #[test]
    fn issue_comment_create_parses() {
        let cli = Cli::try_parse_from([
            "gitee", "issue", "comment", "create", "I88", "-m", "looking into it",
        ])
        .expect("issue comment create should parse");
        let Command::Issue(IssueCmd::Comment(IssueCommentCmd::Create { number, body })) = cli.cmd
        else {
            panic!("expected issue comment create");
        };
        assert_eq!(number, "I88");
        assert_eq!(body.body, "looking into it");
    }

    #[test]
    fn pr_comment_create_parses() {
        let cli = Cli::try_parse_from([
            "gitee", "pr", "comment", "create", "42", "-m", "LGTM",
        ])
        .expect("pr comment create should parse");
        let Command::Pr(PrCmd::Comment(PrCommentCmd::Create { number, body })) = cli.cmd else {
            panic!("expected pr comment create");
        };
        assert_eq!(number, 42);
        assert_eq!(body.body, "LGTM");
    }

    #[test]
    fn old_flat_comment_form_is_rejected() {
        assert!(
            Cli::try_parse_from(["gitee", "issue", "comment", "I88", "-m", "x"]).is_err(),
            "old issue comment form must fail"
        );
        assert!(
            Cli::try_parse_from(["gitee", "pr", "comment", "42", "-m", "x"]).is_err(),
            "old pr comment form must fail"
        );
    }

    #[test]
    fn issue_comment_list_parses_limit() {
        let cli = Cli::try_parse_from([
            "gitee", "issue", "comment", "list", "I88", "--limit", "5",
        ])
        .expect("issue comment list should parse");
        let Command::Issue(IssueCmd::Comment(IssueCommentCmd::List { number, limit })) = cli.cmd
        else {
            panic!("expected issue comment list");
        };
        assert_eq!(number, "I88");
        assert_eq!(limit.limit, 5);
    }

    #[test]
    fn pr_comment_list_parses_type_and_limit() {
        let cli = Cli::try_parse_from([
            "gitee", "pr", "comment", "list", "42", "--type", "diff", "--limit", "10",
        ])
        .expect("pr comment list should parse");
        let Command::Pr(PrCmd::Comment(PrCommentCmd::List {
            number,
            comment_type,
            limit,
        })) = cli.cmd
        else {
            panic!("expected pr comment list");
        };
        assert_eq!(number, 42);
        assert_eq!(comment_type.as_deref(), Some("diff"));
        assert_eq!(limit.limit, 10);
    }

    #[test]
    fn issue_comment_edit_parses_by_id_and_last() {
        let cli = Cli::try_parse_from([
            "gitee", "issue", "comment", "edit", "7", "-m", "fixed",
        ])
        .expect("issue comment edit by id should parse");
        let Command::Issue(IssueCmd::Comment(IssueCommentCmd::Edit {
            target,
            last,
            body,
        })) = cli.cmd
        else {
            panic!("expected issue comment edit");
        };
        assert_eq!(target, "7");
        assert!(!last);
        assert_eq!(body.as_deref(), Some("fixed"));

        let cli = Cli::try_parse_from([
            "gitee", "issue", "comment", "edit", "I88", "--last", "-m", "fixed",
        ])
        .expect("issue comment edit --last should parse");
        let Command::Issue(IssueCmd::Comment(IssueCommentCmd::Edit {
            target,
            last,
            body,
        })) = cli.cmd
        else {
            panic!("expected issue comment edit --last");
        };
        assert_eq!(target, "I88");
        assert!(last);
        assert_eq!(body.as_deref(), Some("fixed"));
    }

    #[test]
    fn pr_comment_edit_parses_by_id_and_last() {
        let cli = Cli::try_parse_from([
            "gitee", "pr", "comment", "edit", "42", "-m", "fixed",
        ])
        .expect("pr comment edit by id should parse");
        let Command::Pr(PrCmd::Comment(PrCommentCmd::Edit {
            target,
            last,
            body,
        })) = cli.cmd
        else {
            panic!("expected pr comment edit");
        };
        assert_eq!(target, 42);
        assert!(!last);
        assert_eq!(body.as_deref(), Some("fixed"));

        let cli = Cli::try_parse_from([
            "gitee", "pr", "comment", "edit", "12", "--last", "-m", "fixed",
        ])
        .expect("pr comment edit --last should parse");
        let Command::Pr(PrCmd::Comment(PrCommentCmd::Edit {
            target,
            last,
            body,
        })) = cli.cmd
        else {
            panic!("expected pr comment edit --last");
        };
        assert_eq!(target, 12);
        assert!(last);
        assert_eq!(body.as_deref(), Some("fixed"));
    }
}
