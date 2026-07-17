use serde::Serialize;
use std::io::IsTerminal;
use tabled::{Table, Tabled};

use crate::models::*;

pub struct Output {
    pub json: Option<String>,
}

impl Output {
    /// Render either as JSON (when `--json` was given) or via the human printer.
    pub fn render<T: Serialize>(&self, data: &T, human: impl FnOnce()) {
        match &self.json {
            Some(spec) => print_json(data, spec),
            None => human(),
        }
    }
}

fn print_json<T: Serialize>(data: &T, spec: &str) {
    let value = serde_json::to_value(data)
        .unwrap_or_else(|e| serde_json::json!({"error": format!("serialize: {e}")}));
    let out = if spec.trim().is_empty() {
        value
    } else {
        let fields: Vec<String> = spec
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();
        project(value, &fields)
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&out)
            .unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    );
}

/// Project a JSON value down to the requested fields. Arrays project each
/// element; objects keep only listed keys; scalars pass through unchanged.
fn project(value: serde_json::Value, fields: &[String]) -> serde_json::Value {
    match value {
        serde_json::Value::Array(items) => serde_json::Value::Array(
            items.into_iter().map(|v| project(v, fields)).collect(),
        ),
        other => pick(other, fields),
    }
}

fn pick(value: serde_json::Value, fields: &[String]) -> serde_json::Value {
    if let serde_json::Value::Object(map) = value {
        let mut out = serde_json::Map::new();
        for f in fields {
            if let Some(v) = map.get(f) {
                out.insert(f.clone(), v.clone());
            }
        }
        serde_json::Value::Object(out)
    } else {
        value
    }
}


#[cfg(test)]
mod project_tests {
    use super::project;
    use serde_json::json;

    #[test]
    fn object_keeps_only_listed_keys() {
        let value = json!({"a": 1, "b": 2, "c": 3});
        let fields = vec!["a".to_string(), "c".to_string()];
        assert_eq!(project(value, &fields), json!({"a": 1, "c": 3}));
    }

    #[test]
    fn array_projects_each_element() {
        let value = json!([
            {"a": 1, "b": 2},
            {"a": 3, "b": 4}
        ]);
        let fields = vec!["a".to_string()];
        assert_eq!(
            project(value, &fields),
            json!([{"a": 1}, {"a": 3}])
        );
    }

    #[test]
    fn missing_keys_are_omitted() {
        let value = json!({"a": 1});
        let fields = vec!["a".to_string(), "missing".to_string()];
        assert_eq!(project(value, &fields), json!({"a": 1}));
    }

    #[test]
    fn empty_field_list_yields_empty_object() {
        let value = json!({"a": 1, "b": 2});
        assert_eq!(project(value, &[]), json!({}));
    }

    #[test]
    fn scalar_passes_through_unchanged() {
        let value = json!(42);
        let fields = vec!["a".to_string()];
        assert_eq!(project(value, &fields), json!(42));
    }
}

// --- color --------------------------------------------------------------

fn color() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

fn paint(code: &str, s: &str) -> String {
    if color() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn green(s: &str) -> String {
    paint("32", s)
}
pub fn red(s: &str) -> String {
    paint("31", s)
}
pub fn magenta(s: &str) -> String {
    paint("35", s)
}
#[allow(dead_code)]
pub fn yellow(s: &str) -> String {
    paint("33", s)
}
#[allow(dead_code)]
pub fn cyan(s: &str) -> String {
    paint("36", s)
}
pub fn bold(s: &str) -> String {
    paint("1", s)
}
pub fn dim(s: &str) -> String {
    paint("2", s)
}

/// Color a PR/issue state. `merged` flags merged-ness even when state == "closed".
fn color_state(state: &str, merged: bool) -> String {
    let s = state.to_lowercase();
    if merged || s == "merged" {
        magenta(&s)
    } else if s == "closed" || s == "rejected" {
        red(&s)
    } else if s == "open" || s == "progressing" {
        green(&s)
    } else {
        s
    }
}

// --- pull requests ------------------------------------------------------

#[derive(Tabled)]
struct PrRow {
    number: i64,
    state: String,
    title: String,
    branch: String,
    author: String,
}

pub fn pr_table(items: &[PullRequest]) {
    let rows: Vec<PrRow> = items
        .iter()
        .map(|p| PrRow {
            number: p.number,
            state: color_state(&p.state, p.merged_at.is_some()),
            title: p.title.clone(),
            branch: format!("{} -> {}", p.head.git_ref, p.base.git_ref),
            author: p.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
        })
        .collect();
    println!("{}", Table::new(rows));
}

pub fn one_pr(p: &PullRequest) {
    let state = color_state(&p.state, p.merged_at.is_some());
    println!("{}  {}  [{}]", bold(&format!("!{}", p.number)), p.title, state);
    println!("{} -> {}", dim(&p.head.git_ref), dim(&p.base.git_ref));
    println!("{}", dim(&p.html_url));
    if let Some(b) = &p.body {
        let b = b.trim();
        if !b.is_empty() {
            println!("\n{b}");
        }
    }
}

// --- issues -------------------------------------------------------------

#[derive(Tabled)]
struct IssueRow {
    number: String,
    state: String,
    title: String,
    assignee: String,
}

pub fn issue_table(items: &[Issue]) {
    let rows: Vec<IssueRow> = items
        .iter()
        .map(|i| IssueRow {
            number: i.number.clone(),
            state: color_state(&i.state, false),
            title: i.title.clone(),
            assignee: i
                .assignee
                .as_ref()
                .map(|a| a.login.clone())
                .unwrap_or_default(),
        })
        .collect();
    println!("{}", Table::new(rows));
}

pub fn one_issue(i: &Issue) {
    let state = color_state(&i.state, false);
    println!("{}  {}  [{}]", bold(&format!("#{}", i.number)), i.title, state);
    println!("{}", dim(&i.html_url));
    if let Some(b) = &i.body {
        let b = b.trim();
        if !b.is_empty() {
            println!("\n{b}");
        }
    }
}

pub fn comment_line(c: &Comment) {
    let who = c.user.as_ref().map(|u| u.login.as_str()).unwrap_or("?");
    println!(
        "@{who} commented:\n{}\n{}",
        c.body,
        c.html_url.as_deref().unwrap_or("")
    );
}

// --- repositories -------------------------------------------------------

#[derive(Tabled)]
struct RepoRow {
    name: String,
    visibility: String,
    stars: String,
    description: String,
}

pub fn repo_table(items: &[RepoDetails]) {
    let rows: Vec<RepoRow> = items
        .iter()
        .map(|r| RepoRow {
            name: r.full_name.clone(),
            visibility: if r.private.unwrap_or(false) {
                red("private")
            } else {
                green("public")
            },
            stars: r.stargazers_count.unwrap_or(0).to_string(),
            description: r.description.clone().unwrap_or_default(),
        })
        .collect();
    println!("{}", Table::new(rows));
}

pub fn one_repo(r: &RepoDetails) {
    let vis = if r.private.unwrap_or(false) {
        red("private")
    } else {
        green("public")
    };
    println!("{}  [{}]", bold(&r.full_name), vis);
    if let Some(d) = &r.description {
        let d = d.trim();
        if !d.is_empty() {
            println!("{d}");
        }
    }
    println!(
        "default: {}  stars: {}  forks: {}  issues: {}",
        r.default_branch.as_deref().unwrap_or("-"),
        r.stargazers_count.unwrap_or(0),
        r.fork_count.unwrap_or(0),
        r.open_issues_count.unwrap_or(0),
    );
    if !r.html_url.is_empty() {
        println!("{}", dim(&r.html_url));
    }
}
