use clap::{Parser, Subcommand};
#[derive(Parser)]
#[command(name = "gitee", version, about = "Gitee CLI (gh-like)")]
pub struct Cli {
    #[arg(long, global = true)]
    pub repo: Option<String>,
    #[arg(long, global = true)]
    pub remote: Option<String>,
    #[arg(long, short = 'j', global = true)]
    pub json: bool,
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
    Auth(AuthCmd),
}
#[derive(Subcommand, Clone)]
pub enum PrCmd {
    List {
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        author: Option<String>,
        #[arg(long, default_value_t = 30)]
        limit: usize,
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
        #[arg(long, short = 'm')]
        body: String,
    },
    Approve {
        number: i64,
        #[arg(long)]
        force: bool,
    },
    Close {
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
        #[arg(long)]
        state: Option<String>,
        #[arg(long)]
        assignee: Option<String>,
        #[arg(long, default_value_t = 30)]
        limit: usize,
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
    Link {
        number: String,
        pr: i64,
    },
    Comment {
        number: String,
        #[arg(long, short = 'm')]
        body: String,
    },
}
#[derive(Subcommand, Clone)]
pub enum AuthCmd {
    Login {
        #[arg(long)]
        token: Option<String>,
        #[arg(long, default_value = "gitee.com")]
        host: String,
    },
}
