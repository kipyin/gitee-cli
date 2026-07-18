use serde::Serialize;
use std::io::IsTerminal;
use std::io::{self, Write};
use tabled::{Table, Tabled};

use crate::error::GiteeError;
use crate::models::*;

pub struct Output {
    pub json: Option<String>,
    pub jq: Option<String>,
}

impl Output {
    /// Render either as JSON (when `--json` was given) or via the human printer.
    pub fn render<T: serde::Serialize, W: Write>(
        &self,
        w: &mut W,
        data: &T,
        human: impl FnOnce(&mut W) -> io::Result<()>,
    ) -> crate::error::Result<()> {
        match &self.json {
            Some(spec) => print_json(w, data, spec, self.jq.as_deref())?,
            None => human(w)?,
        }
        Ok(())
    }
}

fn print_json<T: Serialize, W: Write>(
    w: &mut W,
    data: &T,
    spec: &str,
    jq: Option<&str>,
) -> crate::error::Result<()> {
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
    if let Some(expr) = jq {
        return print_jq(w, &out, expr);
    }
    writeln!(
        w,
        "{}",
        serde_json::to_string_pretty(&out).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    )?;
    Ok(())
}

/// Apply a jq expression to the (already projected) value. Each result prints
/// on its own line: string scalars unquoted, everything else as compact JSON.
fn print_jq<W: Write>(w: &mut W, value: &serde_json::Value, expr: &str) -> crate::error::Result<()> {
    for r in run_jq(value, expr)? {
        match r {
            serde_json::Value::String(s) => writeln!(w, "{s}")?,
            other => writeln!(
                w,
                "{}",
                serde_json::to_string(&other)
                    .map_err(|e| GiteeError::Usage(format!("--jq: cannot serialize result: {e}")))?
            )?,
        }
    }
    Ok(())
}

/// Evaluate `expr` against `value` with the pure-Rust jaq engine (no C libjq).
/// Setup follows the jaq 3.x idiom: core+std+json defs/funs, compile once, then
/// run over a JustLut context (no `inputs` support — we evaluate a single value).
fn run_jq(value: &serde_json::Value, expr: &str) -> crate::error::Result<Vec<serde_json::Value>> {
    use jaq_core::data::JustLut;
    use jaq_core::load::{Arena, File, Loader};
    use jaq_core::{Compiler, Ctx, Vars};
    use jaq_json::Val;

    let arena = Arena::default();
    let defs = jaq_core::defs()
        .chain(jaq_std::defs())
        .chain(jaq_json::defs());
    let loader = Loader::new(defs);
    let modules = loader
        .load(&arena, File { path: (), code: expr })
        .map_err(|errs| invalid_jq(expr, format!("{errs:?}")))?;
    let funs = jaq_core::funs()
        .chain(jaq_std::funs())
        .chain(jaq_json::funs());
    let filter: jaq_core::Filter<JustLut<Val>> = Compiler::default()
        .with_funs(funs)
        .compile(modules)
        .map_err(|errs| invalid_jq(expr, format!("{errs:?}")))?;
    let input: Val = serde_json::from_value(value.clone())
        .map_err(|e| GiteeError::Usage(format!("--jq: cannot convert input: {e}")))?;
    let ctx: Ctx<JustLut<Val>> = Ctx::new(&filter.lut, Vars::new([]));
    let mut out = Vec::new();
    for r in filter.id.run((ctx, input)) {
        let v = r.map_err(|e| GiteeError::Usage(format!("--jq evaluation failed: {e:?}")))?;
        out.push(val_to_json(&v)?);
    }
    Ok(out)
}

/// jaq's `Val` is a JSON superset (byte strings, non-string keys) with no
/// Serialize impl, so convert back by hand. Plain-JSON results round-trip
/// exactly; byte strings decode lossy-UTF-8; non-string object keys error.
fn val_to_json(v: &jaq_json::Val) -> crate::error::Result<serde_json::Value> {
    use jaq_json::Val;
    use serde_json::Value;
    Ok(match v {
        Val::Null => Value::Null,
        Val::Bool(b) => Value::Bool(*b),
        // Num's Display is its JSON spelling; parse it back as a JSON number.
        Val::Num(n) => serde_json::from_str(&n.to_string())
            .map_err(|e| GiteeError::Usage(format!("--jq: cannot convert number {n}: {e}")))?,
        Val::TStr(s) | Val::BStr(s) => Value::String(String::from_utf8_lossy(s).into_owned()),
        Val::Arr(a) => Value::Array(
            a.iter()
                .map(val_to_json)
                .collect::<crate::error::Result<Vec<_>>>()?,
        ),
        Val::Obj(o) => {
            let mut map = serde_json::Map::new();
            for (k, val) in o.iter() {
                let key = match k {
                    Val::TStr(s) => String::from_utf8_lossy(s).into_owned(),
                    other => {
                        return Err(GiteeError::Usage(format!(
                            "--jq: object key is not a string ({other:?}); cannot render as JSON"
                        )))
                    }
                };
                map.insert(key, val_to_json(val)?);
            }
            Value::Object(map)
        }
    })
}

fn invalid_jq(expr: &str, details: String) -> GiteeError {
    GiteeError::Usage(format!("invalid --jq expression '{expr}': {details}"))
}

/// Project a JSON value down to the requested fields. Arrays project each
/// element; objects keep only listed keys; scalars pass through unchanged.
fn project(value: serde_json::Value, fields: &[String]) -> serde_json::Value {
    match value {
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(|v| project(v, fields)).collect())
        }
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
mod jq_tests {
    use crate::out::Output;
    use serde_json::json;

    fn render_json(value: serde_json::Value, json: &str, jq: &str) -> String {
        let out = Output {
            json: Some(json.to_string()),
            jq: Some(jq.to_string()),
        };
        let mut buf = Vec::new();
        out.render(&mut buf, &value, |_w| unreachable!("human path"))
            .expect("render should succeed");
        String::from_utf8(buf).unwrap()
    }

    #[test]
    fn jq_string_scalar_prints_unquoted() {
        let data = json!([{"title": "Fix bug", "number": 1}]);
        assert_eq!(render_json(data, "", ".[0].title"), "Fix bug\n");
    }

    #[test]
    fn jq_applies_after_field_projection() {
        let data = json!([
            {"number": 1, "title": "a", "extra": true},
            {"number": 2, "title": "b", "extra": false}
        ]);
        assert_eq!(render_json(data, "number,title", "map(.number)"), "[1,2]\n");
    }

    #[test]
    fn jq_invalid_expression_is_usage_error() {
        let out = Output {
            json: Some("".to_string()),
            jq: Some(".[".to_string()),
        };
        let mut buf = Vec::new();
        let err = out
            .render(&mut buf, &json!([1]), |_w| unreachable!("human path"))
            .expect_err("invalid expression must fail");
        let msg = err.to_string();
        assert!(msg.contains(".["), "message names the expression: {msg}");
    }

    #[test]
    fn jq_multiple_results_print_one_per_line() {
        let data = json!([1, 2, 3]);
        assert_eq!(render_json(data, "", ".[]"), "1\n2\n3\n");
    }

    #[test]
    fn jq_non_string_scalar_prints_as_json() {
        let data = json!([{"number": 42}]);
        assert_eq!(render_json(data, "", ".[0].number"), "42\n");
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
        assert_eq!(project(value, &fields), json!([{"a": 1}, {"a": 3}]));
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
pub fn cyan(s: &str) -> String {
    paint("36", s)
}
pub fn bold(s: &str) -> String {
    paint("1", s)
}
pub fn dim(s: &str) -> String {
    paint("2", s)
}

/// Style a PR state. `merged` flags merged-ness even when state == closed.
fn pr_state_style(state: PrState, merged: bool) -> String {
    if merged || state == PrState::Merged {
        return magenta(state.as_str());
    }
    match state {
        PrState::Open => green(state.as_str()),
        PrState::Closed => red(state.as_str()),
        _ => state.as_str().to_string(),
    }
}

/// Style an issue state.
pub(crate) fn issue_state_style(state: IssueState) -> String {
    match state {
        IssueState::Open | IssueState::Progressing => green(state.as_str()),
        IssueState::Closed | IssueState::Rejected => red(state.as_str()),
        _ => state.as_str().to_string(),
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

pub fn pr_table(w: &mut impl Write, items: &[PullRequest]) -> std::io::Result<()> {
    let rows: Vec<PrRow> = items
        .iter()
        .map(|p| PrRow {
            number: p.number,
            state: pr_state_style(p.state, p.merged_at.is_some()),
            title: p.title.clone(),
            branch: format!("{} -> {}", p.head.git_ref, p.base.git_ref),
            author: p.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
        })
        .collect();
    writeln!(w, "{}", Table::new(rows))
}

pub fn one_pr(w: &mut impl Write, p: &PullRequest) -> std::io::Result<()> {
    let state = pr_state_style(p.state, p.merged_at.is_some());
    writeln!(
        w,
        "{}  {}  [{}]",
        bold(&format!("!{}", p.number)),
        p.title,
        state
    )?;
    writeln!(w, "{} -> {}", dim(&p.head.git_ref), dim(&p.base.git_ref))?;
    writeln!(w, "{}", dim(&p.html_url))?;
    if let Some(b) = &p.body {
        let b = b.trim();
        if !b.is_empty() {
            writeln!(w, "\n{b}")?;
        }
    }
    Ok(())
}

/// Colorize one line of unified diff output.
pub fn color_diff_line(line: &str) -> String {
    if line.starts_with("@@") {
        cyan(line)
    } else if line.starts_with('+') && !line.starts_with("+++") {
        green(line)
    } else if line.starts_with('-') && !line.starts_with("---") {
        red(line)
    } else {
        line.to_string()
    }
}

pub fn pr_diff(w: &mut impl Write, files: &[FileDiff]) -> std::io::Result<()> {
    if files.is_empty() {
        writeln!(w, "(no changed files)")?;
        return Ok(());
    }
    for (i, f) in files.iter().enumerate() {
        if i > 0 {
            writeln!(w)?;
        }
        let name = &f.filename;
        writeln!(w, "{}", bold(&format!("diff --git a/{name} b/{name}")))?;
        writeln!(w, "{}", bold(name))?;
        match &f.patch {
            Some(p) if !p.is_empty() => {
                for line in p.lines() {
                    writeln!(w, "{}", color_diff_line(line))?;
                }
            }
            _ => writeln!(w, "{}", dim("(no text diff — binary or too large)"))?,
        }
    }
    Ok(())
}

// --- milestones ---------------------------------------------------------

#[derive(Tabled)]
struct MilestoneRow {
    number: String,
    title: String,
    state: String,
    due_on: String,
}

fn milestone_state_label(state: Option<&String>) -> String {
    match state.map(|s| s.as_str()) {
        Some("open") => green("open"),
        Some("closed") => red("closed"),
        other => other.unwrap_or("").to_string(),
    }
}

pub fn milestone_table(w: &mut impl Write, items: &[Milestone]) -> std::io::Result<()> {
    let rows: Vec<MilestoneRow> = items
        .iter()
        .map(|m| MilestoneRow {
            number: m.number.to_string(),
            title: m.title.clone(),
            state: milestone_state_label(m.state.as_ref()),
            due_on: m.due_on.clone().unwrap_or_default(),
        })
        .collect();
    writeln!(w, "{}", Table::new(rows))
}

pub fn one_milestone(w: &mut impl Write, m: &Milestone) -> std::io::Result<()> {
    let state = milestone_state_label(m.state.as_ref());
    writeln!(
        w,
        "{}  {}  [{}]",
        bold(&format!("#{}", m.number)),
        m.title,
        state
    )?;
    if let Some(d) = &m.due_on {
        writeln!(w, "Due: {d}")?;
    }
    writeln!(w, "{}", dim(m.html_url.as_deref().unwrap_or("")))?;
    let open = m.open_issues.unwrap_or(0);
    let closed = m.closed_issues.unwrap_or(0);
    writeln!(w, "Issues: {} open, {} closed", open, closed)?;
    if let Some(desc) = &m.description {
        let d = desc.trim();
        if !d.is_empty() {
            writeln!(w, "\n{d}")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod milestone_printer_tests {
    use super::*;

    #[test]
    fn milestone_table_contains_number_title_and_state() {
        let milestone = Milestone {
            number: 3,
            title: "v1.0".into(),
            state: Some("open".into()),
            due_on: Some("2026-12-31".into()),
            ..Default::default()
        };

        let mut buf = Vec::new();
        milestone_table(&mut buf, &[milestone]).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("3"));
        assert!(out.contains("v1.0"));
        assert!(out.contains("open"));
    }
}


#[cfg(test)]
mod diff_tests {
    use super::*;

    #[test]
    fn color_diff_line_marks_hunks_and_changes() {
        assert_eq!(color_diff_line("@@ -1,3 +1,4 @@"), cyan("@@ -1,3 +1,4 @@"));
        assert_eq!(color_diff_line("+added"), green("+added"));
        assert_eq!(color_diff_line("-removed"), red("-removed"));
        assert_eq!(color_diff_line(" context"), " context");
        assert_eq!(color_diff_line("+++ b/file"), "+++ b/file");
        assert_eq!(color_diff_line("--- a/file"), "--- a/file");
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

pub fn issue_table(w: &mut impl Write, items: &[Issue]) -> std::io::Result<()> {
    let rows: Vec<IssueRow> = items
        .iter()
        .map(|i| IssueRow {
            number: i.number.clone(),
            state: issue_state_style(i.state),
            title: i.title.clone(),
            assignee: i
                .assignee
                .as_ref()
                .map(|a| a.login.clone())
                .unwrap_or_default(),
        })
        .collect();
    writeln!(w, "{}", Table::new(rows))
}

pub fn one_issue(w: &mut impl Write, i: &Issue) -> std::io::Result<()> {
    let state = issue_state_style(i.state);
    writeln!(
        w,
        "{}  {}  [{}]",
        bold(&format!("#{}", i.number)),
        i.title,
        state
    )?;
    writeln!(w, "{}", dim(&i.html_url))?;
    if let Some(b) = &i.body {
        let b = b.trim();
        if !b.is_empty() {
            writeln!(w, "\n{b}")?;
        }
    }
    Ok(())
}


#[derive(Serialize)]
pub struct PrStatus {
    pub created: Vec<PullRequest>,
    pub assigned: Vec<PullRequest>,
    pub awaiting_test: Vec<PullRequest>,
}

#[derive(Serialize)]
pub struct IssueStatus {
    pub created: Vec<Issue>,
    pub assigned: Vec<Issue>,
}

pub fn pr_status(w: &mut impl Write, s: &PrStatus) -> std::io::Result<()> {
    writeln!(w, "{}", bold("Created by me"))?;
    if s.created.is_empty() {
        writeln!(w, "{}", dim("(none)"))?;
    } else {
        pr_table(w, &s.created)?;
    }
    writeln!(w)?;
    writeln!(w, "{}", bold("Assigned to me"))?;
    if s.assigned.is_empty() {
        writeln!(w, "{}", dim("(none)"))?;
    } else {
        pr_table(w, &s.assigned)?;
    }
    writeln!(w)?;
    writeln!(w, "{}", bold("Awaiting my test"))?;
    if s.awaiting_test.is_empty() {
        writeln!(w, "{}", dim("(none)"))?;
    } else {
        pr_table(w, &s.awaiting_test)?;
    }
    Ok(())
}

pub fn issue_status(w: &mut impl Write, s: &IssueStatus) -> std::io::Result<()> {
    writeln!(w, "{}", bold("Created by me"))?;
    if s.created.is_empty() {
        writeln!(w, "{}", dim("(none)"))?;
    } else {
        issue_table(w, &s.created)?;
    }
    writeln!(w)?;
    writeln!(w, "{}", bold("Assigned to me"))?;
    if s.assigned.is_empty() {
        writeln!(w, "{}", dim("(none)"))?;
    } else {
        issue_table(w, &s.assigned)?;
    }
    Ok(())
}


pub fn comment_line(w: &mut impl Write, c: &Comment) -> std::io::Result<()> {
    let who = c.user.as_ref().map(|u| u.login.as_str()).unwrap_or("?");
    writeln!(
        w,
        "@{who} commented:\n{}\n{}",
        c.body,
        c.html_url.as_deref().unwrap_or("")
    )
}
// --- labels -------------------------------------------------------------

#[derive(Tabled)]
struct LabelRow {
    name: String,
    color: String,
    id: String,
}

pub fn label_table(w: &mut impl Write, items: &[Label]) -> std::io::Result<()> {
    let rows: Vec<LabelRow> = items
        .iter()
        .map(|l| LabelRow {
            name: l.name.clone(),
            color: format!("#{}", l.color.as_deref().unwrap_or("000000")),
            id: l.id.to_string(),
        })
        .collect();
    writeln!(w, "{}", Table::new(rows))
}


// --- releases -----------------------------------------------------------

#[derive(Tabled)]
struct ReleaseRow {
    tag: String,
    name: String,
    status: String,
    created: String,
}

fn release_status(prerelease: Option<bool>) -> String {
    if prerelease.unwrap_or(false) {
        yellow("pre")
    } else {
        green("release")
    }
}

pub fn release_table(w: &mut impl Write, items: &[Release]) -> std::io::Result<()> {
    let rows: Vec<ReleaseRow> = items
        .iter()
        .map(|rel| ReleaseRow {
            tag: rel.tag_name.clone(),
            name: rel.name.clone().unwrap_or_default(),
            status: release_status(rel.prerelease),
            created: rel.created_at.clone().unwrap_or_default(),
        })
        .collect();
    writeln!(w, "{}", Table::new(rows))
}

pub fn one_release(w: &mut impl Write, rel: &Release) -> std::io::Result<()> {
    let title = rel
        .name
        .as_deref()
        .filter(|n| !n.is_empty())
        .unwrap_or(&rel.tag_name);
    writeln!(
        w,
        "{}  {}  [{}]",
        bold(&rel.tag_name),
        title,
        release_status(rel.prerelease)
    )?;
    if let Some(b) = &rel.body {
        let b = b.trim();
        if !b.is_empty() {
            writeln!(w, "\n{b}")?;
        }
    }
    let assets = rel.assets.as_deref().unwrap_or(&[]);
    writeln!(w, "\n{} asset(s)", assets.len())?;
    for asset in assets {
        writeln!(w, "  {}", asset.name)?;
    }
    Ok(())
}


// --- gists --------------------------------------------------------------

#[derive(Tabled)]
struct GistRow {
    id: String,
    description: String,
    files: String,
    updated: String,
}

fn gist_visibility(public: Option<bool>) -> String {
    if public.unwrap_or(false) {
        green("public")
    } else {
        dim("secret")
    }
}

pub fn gist_table(w: &mut impl Write, items: &[Gist]) -> std::io::Result<()> {
    let rows: Vec<GistRow> = items
        .iter()
        .map(|g| GistRow {
            id: g.id.clone(),
            description: g.description.clone().unwrap_or_default(),
            files: g.files.as_ref().map(|f| f.len().to_string()).unwrap_or_else(|| "0".into()),
            updated: g.updated_at.clone().unwrap_or_default(),
        })
        .collect();
    writeln!(w, "{}", Table::new(rows))
}

pub fn one_gist(w: &mut impl Write, g: &Gist) -> std::io::Result<()> {
    writeln!(
        w,
        "{}  {}  [{}]",
        bold(&g.id),
        g.description.as_deref().unwrap_or("(no description)"),
        gist_visibility(g.public),
    )?;
    if let Some(updated) = &g.updated_at {
        writeln!(w, "updated: {updated}")?;
    }
    let files = g.files.as_ref();
    writeln!(w, "
{} file(s)", files.map(|f| f.len()).unwrap_or(0))?;
    if let Some(files) = files {
        for (name, file) in files {
            let size = file.size.map(|s| s.to_string()).unwrap_or_else(|| "?".into());
            writeln!(w, "  {name} ({size} bytes)")?;
        }
    }
    Ok(())
}

pub fn gist_raw(w: &mut impl Write, g: &Gist) -> std::io::Result<()> {
    for (i, (_name, file)) in g.files.iter().flatten().enumerate() {
        if i > 0 {
            writeln!(w)?;
        }
        if let Some(content) = &file.content {
            write!(w, "{content}")?;
        }
    }
    Ok(())
}

// --- repositories -------------------------------------------------------

#[derive(Tabled)]
struct RepoRow {
    name: String,
    visibility: String,
    stars: String,
    description: String,
}

pub fn repo_table(w: &mut impl Write, items: &[RepoDetails]) -> std::io::Result<()> {
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
    writeln!(w, "{}", Table::new(rows))
}

pub fn one_repo(w: &mut impl Write, r: &RepoDetails) -> std::io::Result<()> {
    let vis = if r.private.unwrap_or(false) {
        red("private")
    } else {
        green("public")
    };
    writeln!(w, "{}  [{}]", bold(&r.full_name), vis)?;
    if let Some(d) = &r.description {
        let d = d.trim();
        if !d.is_empty() {
            writeln!(w, "{d}")?;
        }
    }
    writeln!(
        w,
        "default: {}  stars: {}  forks: {}  issues: {}",
        r.default_branch.as_deref().unwrap_or("-"),
        r.stargazers_count.unwrap_or(0),
        r.fork_count.unwrap_or(0),
        r.open_issues_count.unwrap_or(0),
    )?;
    if !r.html_url.is_empty() {
        writeln!(w, "{}", dim(&r.html_url))?;
    }
    Ok(())
}

// --- users --------------------------------------------------------------

#[derive(Tabled)]
struct UserRow {
    login: String,
    name: String,
    html_url: String,
}

pub fn user_table(w: &mut impl Write, items: &[UserBasic]) -> std::io::Result<()> {
    let rows: Vec<UserRow> = items
        .iter()
        .map(|u| UserRow {
            login: u.login.clone(),
            name: u.name.clone().unwrap_or_default(),
            html_url: u.html_url.clone().unwrap_or_default(),
        })
        .collect();
    writeln!(w, "{}", Table::new(rows))
}


#[cfg(test)]
mod printer_tests {
    use super::*;

    fn pr_fixture() -> PullRequest {
        PullRequest {
            number: 12,
            title: "Add pagination helpers".into(),
            head: PrBranch {
                git_ref: "feature/paging".into(),
                ..Default::default()
            },
            base: PrBranch {
                git_ref: "master".into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn pr_table_contains_number_title_and_branch() {
        let mut buf = Vec::new();
        pr_table(&mut buf, &[pr_fixture()]).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("12"));
        assert!(out.contains("Add pagination helpers"));
        assert!(out.contains("feature/paging -> master"));
    }

    #[test]
    fn one_issue_shows_number_and_title() {
        let issue = Issue {
            number: "88".into(),
            title: "Login fails with expired token".into(),
            html_url: "https://gitee.com/oschina/gitee-cli/issues/I88".into(),
            ..Default::default()
        };

        let mut buf = Vec::new();
        one_issue(&mut buf, &issue).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("#88"));
        assert!(out.contains("Login fails with expired token"));
    }

    #[test]
    fn pr_diff_renders_git_header_and_no_text_fallback() {
        let with_patch = FileDiff {
            filename: "pom.xml".into(),
            patch: Some("@@ -1 +1 @@\n-old\n+new".into()),
            ..Default::default()
        };

        let without_patch = FileDiff {
            filename: "logo.png".into(),
            ..Default::default()
        };

        let mut buf = Vec::new();
        pr_diff(&mut buf, &[with_patch, without_patch]).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("diff --git a/pom.xml b/pom.xml"));
        assert!(out.contains("@@ -1 +1 @@"));
        assert!(out.contains("(no text diff — binary or too large)"));
    }

    #[test]
    fn gist_table_shows_id_and_description() {
        let gist = Gist {
            id: "abc123".into(),
            description: Some("test gist snippet".into()),
            updated_at: Some("2024-06-02T12:30:00+08:00".into()),
            files: Some(std::collections::BTreeMap::from([(
                "a.txt".into(),
                GistFile {
                    size: Some(13),
                    ..Default::default()
                },
            )])),
            ..Default::default()
        };

        let mut buf = Vec::new();
        gist_table(&mut buf, &[gist]).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("abc123"));
        assert!(out.contains("test gist snippet"));
    }

    #[test]
    fn one_release_prints_asset_count_and_names() {
        let release = Release {
            tag_name: "v1.2.0".into(),
            name: Some("v1.2.0".into()),
            prerelease: Some(false),
            assets: Some(vec![
                ReleaseAsset {
                    name: "gitee-linux-amd64.tar.xz".into(),
                    browser_download_url: "https://example.com/linux".into(),
                },
                ReleaseAsset {
                    name: "gitee-darwin-arm64.tar.xz".into(),
                    browser_download_url: "https://example.com/darwin".into(),
                },
            ]),
            ..Default::default()
        };

        let mut buf = Vec::new();
        one_release(&mut buf, &release).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("2 asset(s)"));
        assert!(out.contains("gitee-linux-amd64.tar.xz"));
        assert!(out.contains("gitee-darwin-arm64.tar.xz"));
    }

    #[test]
    fn comment_line_format() {
        let comment = Comment {
            body: "Looks good to me".into(),
            html_url: Some("https://gitee.com/oschina/gitee-cli/pulls/12#note_1".into()),
            user: Some(UserBasic {
                login: "dev1".into(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let mut buf = Vec::new();
        comment_line(&mut buf, &comment).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("@dev1 commented:"));
        assert!(out.contains("Looks good to me"));
        assert!(out.contains("https://gitee.com/oschina/gitee-cli/pulls/12#note_1"));
    }

    #[test]
    fn user_table_contains_login_name_and_url() {
        let users = vec![UserBasic {
            login: "kip".into(),
            name: Some("Kip Yin".into()),
            html_url: Some("https://gitee.com/kip".into()),
            ..Default::default()
        }];

        let mut buf = Vec::new();
        user_table(&mut buf, &users).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("kip"));
        assert!(out.contains("Kip Yin"));
        assert!(out.contains("https://gitee.com/kip"));
    }

    #[test]
    fn label_table_shows_name_color_and_id() {
        let labels = vec![Label {
            id: 42,
            name: "bug".into(),
            color: Some("ff0000".into()),
        }];
        let mut buf = Vec::new();
        label_table(&mut buf, &labels).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("bug"));
        assert!(out.contains("#ff0000"));
        assert!(out.contains("42"));
    }

    #[test]
    fn pr_status_renders_sections_and_titles() {
        let status = PrStatus {
            created: vec![pr_fixture()],
            assigned: vec![],
            awaiting_test: vec![PullRequest {
                number: 7,
                title: "Needs QA sign-off".into(),
                ..Default::default()
            }],
        };

        let mut buf = Vec::new();
        pr_status(&mut buf, &status).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("Created by me"));
        assert!(out.contains("Assigned to me"));
        assert!(out.contains("Awaiting my test"));
        assert!(out.contains("Add pagination helpers"));
        assert!(out.contains("Needs QA sign-off"));
        assert!(out.contains("(none)"));
    }

    #[test]
    fn issue_status_renders_sections_and_empty_placeholder() {
        let status = IssueStatus {
            created: vec![Issue {
                number: "42".into(),
                title: "Broken deploy".into(),
                ..Default::default()
            }],
            assigned: vec![],
        };

        let mut buf = Vec::new();
        issue_status(&mut buf, &status).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("Created by me"));
        assert!(out.contains("Assigned to me"));
        assert!(out.contains("Broken deploy"));
        assert!(out.contains("(none)"));
    }

    #[test]
    fn status_structs_serialize_expected_json_keys() {
        let pr_status = PrStatus {
            created: vec![pr_fixture()],
            assigned: vec![],
            awaiting_test: vec![],
        };
        let pr_json = serde_json::to_value(&pr_status).unwrap();
        assert!(pr_json.get("created").unwrap().is_array());
        assert!(pr_json.get("assigned").unwrap().is_array());
        assert!(pr_json.get("awaiting_test").unwrap().is_array());
        assert_eq!(pr_json["created"][0]["title"], "Add pagination helpers");

        let issue_status = IssueStatus {
            created: vec![],
            assigned: vec![Issue {
                number: "1".into(),
                title: "Assigned item".into(),
                ..Default::default()
            }],
        };
        let issue_json = serde_json::to_value(&issue_status).unwrap();
        assert!(issue_json.get("created").unwrap().is_array());
        assert!(issue_json.get("assigned").unwrap().is_array());
        assert_eq!(issue_json["assigned"][0]["title"], "Assigned item");
    }

}
// --- status dashboard ---------------------------------------------------

#[derive(Serialize)]
pub struct Dashboard {
    pub assigned: Vec<Issue>,
    pub created: Vec<Issue>,
}

#[derive(Tabled)]
struct DashboardIssueRow {
    repo: String,
    number: String,
    state: String,
    title: String,
}

fn dashboard_issue_table(w: &mut impl Write, items: &[Issue]) -> std::io::Result<()> {
    let rows: Vec<DashboardIssueRow> = items
        .iter()
        .map(|i| DashboardIssueRow {
            repo: i
                .repository
                .as_ref()
                .and_then(|r| r.full_name.clone())
                .unwrap_or_default(),
            number: i.number.clone(),
            state: issue_state_style(i.state),
            title: i.title.clone(),
        })
        .collect();
    writeln!(w, "{}", Table::new(rows))
}

pub fn dashboard(w: &mut impl Write, d: &Dashboard) -> std::io::Result<()> {
    writeln!(w, "{}", bold("Assigned to me"))?;
    if d.assigned.is_empty() {
        writeln!(w, "{}", dim("(none)"))?;
    } else {
        dashboard_issue_table(w, &d.assigned)?;
    }
    writeln!(w)?;
    writeln!(w, "{}", bold("Created by me"))?;
    if d.created.is_empty() {
        writeln!(w, "{}", dim("(none)"))?;
    } else {
        dashboard_issue_table(w, &d.created)?;
    }
    Ok(())
}

#[cfg(test)]
mod dashboard_printer_tests {
    use super::*;

    #[test]
    fn dashboard_shows_repo_when_set() {
        let d = Dashboard {
            assigned: vec![Issue {
                number: "1".into(),
                title: "Fix bug".into(),
                state: IssueState::Open,
                repository: Some(IssueRepoRef {
                    full_name: Some("owner/repo".into()),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            created: vec![],
        };

        let mut buf = Vec::new();
        dashboard(&mut buf, &d).unwrap();
        let out = String::from_utf8(buf).unwrap();
        assert!(out.contains("owner/repo"));
        assert!(out.contains("(none)"));
    }

    #[test]
    fn dashboard_serialization_has_assigned_created_keys() {
        let d = Dashboard {
            assigned: vec![],
            created: vec![],
        };
        let value = serde_json::to_value(&d).unwrap();
        let obj = value.as_object().unwrap();
        assert!(obj.contains_key("assigned"));
        assert!(obj.contains_key("created"));
    }

    #[test]
    fn dashboard_json_projection_keeps_section_arrays() {
        let d = Dashboard {
            assigned: vec![Issue {
                number: "1".into(),
                ..Default::default()
            }],
            created: vec![Issue {
                number: "2".into(),
                ..Default::default()
            }],
        };
        let value = serde_json::to_value(&d).unwrap();
        let fields = vec!["assigned".to_string(), "created".to_string()];
        let projected = project(value, &fields);
        let obj = projected.as_object().unwrap();
        assert_eq!(obj["assigned"].as_array().unwrap().len(), 1);
        assert_eq!(obj["created"].as_array().unwrap().len(), 1);
    }
}

