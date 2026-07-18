use clap::{Parser, Subcommand};

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
    /// Log HTTP requests and responses to stderr.
    #[arg(long, global = true)]
    pub debug: bool,
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
    Repo(RepoCmd),
    #[command(subcommand)]
    Auth(AuthCmd),
    /// Print a shell completion script (bash, zsh, fish, powershell, elvish).
    Completions { shell: Option<String> },
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
    /// Show details of a pull request.
    View {
        number: i64,
    },
    /// Show the unified diff of a pull request.
    Diff {
        number: i64,
    },
    /// Fetch and check out a pull request branch locally.
    Checkout {
        number: i64,
    },
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        head: Option<String>,
        #[arg(long)]
        base: Option<String>,
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
    Comment {
        number: i64,
        #[command(flatten)]
        body: CommentArgs,
    },
    Approve {
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
}

#[derive(Subcommand, Clone)]
pub enum IssueCmd {
    List {
        #[command(flatten)]
        list: ListArgs,
        #[arg(long)]
        assignee: Option<String>,
    },
    /// Show details of an issue.
    View {
        number: String,
    },
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long)]
        labels: Option<String>,
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
    Comment {
        number: String,
        #[command(flatten)]
        body: CommentArgs,
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
}

#[derive(Subcommand, Clone)]
pub enum RepoCmd {
    /// Show repository details. Targets the resolved repo unless given owner/name.
    View {
        #[arg(value_name = "REPO")]
        target: Option<String>,
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
}
