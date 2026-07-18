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
    /// jq expression applied to --json output (requires --json). Runs AFTER field
    /// projection. String scalars print unquoted; other results print as JSON;
    /// multiple results print one per line.
    #[arg(long, global = true, requires = "json")]
    pub jq: Option<String>,
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
    /// Raw Gitee REST API request (like `gh api`).
    Api(ApiArgs),
    /// Print a shell completion script (bash, zsh, fish, powershell, elvish).
    Completions { shell: Option<String> },
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
    /// Create a pull request. --fill derives title/body from commits; without
    /// --body/--fill, the repo's pull request template prefills the body.
    Create {
        #[arg(long, required_unless_present = "fill")]
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
        /// Milestone by number or exact title (resolved via the milestones API).
        #[arg(long)]
        milestone: Option<String>,
        /// Mark the issue as a security hole (Gitee-specific).
        #[arg(long)]
        security_hole: bool,
    },
    /// Edit an issue's metadata. At least one flag is required.
    #[command(group = clap::ArgGroup::new("edit_flags").required(true).multiple(true).args(["title", "body", "assignee", "label", "milestone", "security_hole"]))]
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

#[cfg(test)]
mod parse_tests {
    use super::{Cli, Command, IssueCmd, PrCmd};
    use clap::Parser;

    #[test]
    fn pr_create_requires_title_unless_fill() {
        assert!(Cli::try_parse_from(["gitee", "pr", "create"]).is_err());
        let cli = Cli::try_parse_from(["gitee", "pr", "create", "--fill"])
            .expect("--fill alone should parse");
        let Command::Pr(PrCmd::Create { title, fill, .. }) = cli.cmd else {
            panic!("expected pr create");
        };
        assert!(title.is_none());
        assert!(fill);
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
    fn issue_edit_requires_at_least_one_flag() {
        let r = Cli::try_parse_from(["gitee", "issue", "edit", "I1AB"]);
        assert!(r.is_err(), "issue edit with no flags must fail");
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
    fn pr_edit_requires_at_least_one_flag() {
        let r = Cli::try_parse_from(["gitee", "pr", "edit", "5"]);
        assert!(r.is_err(), "pr edit with no flags must fail");
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
}
