use serde::Serialize;
use tabled::{Table, Tabled};

use crate::models::*;

pub struct Output {
    pub json: bool,
}

pub fn json<T: Serialize>(v: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(v).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    );
}

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
            state: p.state.clone(),
            title: p.title.clone(),
            branch: format!("{} -> {}", p.head.git_ref, p.base.git_ref),
            author: p.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
        })
        .collect();
    let t = Table::new(rows);
    println!("{t}");
}

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
            state: i.state.clone(),
            title: i.title.clone(),
            assignee: i
                .assignee
                .as_ref()
                .map(|a| a.login.clone())
                .unwrap_or_default(),
        })
        .collect();
    let t = Table::new(rows);
    println!("{t}");
}

pub fn one_pr(p: &PullRequest) {
    println!(
        "!{}  {}  [{}]\n{} -> {}\n{}",
        p.number, p.title, p.state, p.head.git_ref, p.base.git_ref, p.html_url
    );
}

pub fn one_issue(i: &Issue) {
    println!("#{}  {}  [{}]\n{}", i.number, i.title, i.state, i.html_url);
}

pub fn comment_line(c: &Comment) {
    let who = c.user.as_ref().map(|u| u.login.as_str()).unwrap_or("?");
    println!(
        "@{who} commented:\n{}\n{}",
        c.body,
        c.html_url.as_deref().unwrap_or("")
    );
}
