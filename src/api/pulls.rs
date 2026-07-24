use super::client::Client;
use crate::api::{resolve_latest_comment, StateChange};
use crate::error::{GiteeError, Result};
use crate::models::{
    Comment, FileDiff, Label, MergeMethod, PrComment, PrCommentKind, PrState, PullRequest,
};
use crate::repo::Repo;
use std::collections::HashSet;

pub struct Pulls<'a> {
    client: &'a Client,
    repo: &'a Repo,
}

#[derive(Default)]
pub struct PrFilter<'a> {
    pub state: Option<&'a str>,
    pub author: Option<&'a str>,
    /// 评审者 (reviewer) login — server-side filter per the v5 swagger.
    pub assignee: Option<&'a str>,
    /// 测试者 (tester) login — server-side filter per the v5 swagger.
    pub tester: Option<&'a str>,
    pub limit: usize,
}

#[derive(Default)]
pub struct CreatePr<'a> {
    pub title: &'a str,
    pub head: &'a str,
    pub base: &'a str,
    pub body: Option<&'a str>,
    /// Comma-joined by the handler (v5 takes one `labels` string).
    pub labels: Option<&'a str>,
    pub assignees: Option<&'a str>,
    pub testers: Option<&'a str>,
    pub milestone_number: Option<i64>,
    /// Linked issue ident; paired with close_related_issue=true so the issue
    /// closes on merge (swagger: `issue` is the string ident;
    /// `close_related_issue` is boolean).
    pub issue: Option<&'a str>,
    pub close_related_issue: bool,
}

/// Fields for `pr edit`. All optional; only `Some` fields are sent, so unset
/// values are never blanked. `labels`/`assignees`/`testers` arrive pre-joined
/// (comma-separated) by the handler. Param names per the v5 swagger, except
/// `assignees`/`testers` which the PATCH swagger omits — names follow the PR
/// create endpoint; live round-trip is unverified (no mutations in tests).
#[derive(Default)]
pub struct EditPr<'a> {
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
    pub labels: Option<&'a str>,
    pub assignees: Option<&'a str>,
    pub testers: Option<&'a str>,
    pub milestone_number: Option<i64>,
}

/// Optional form fields for `Pulls::comment` line/diff comments (`commit_id`,
/// `path`, `position`). Omitted fields are not sent.
#[derive(Default)]
pub struct PrCommentPositional<'a> {
    pub path: Option<&'a str>,
    pub position: Option<i64>,
    pub commit_id: Option<&'a str>,
}

/// Filters for `Pulls::list_comments`. `kind` is the CLI vocabulary; ops maps
/// it to Gitee's `comment_type` query (`diff_comment` | `pr_comment`).
#[derive(Default)]
pub struct PrCommentFilter {
    pub kind: Option<PrCommentKind>,
    pub limit: usize,
}

impl Pulls<'_> {
    pub(crate) fn new<'a>(client: &'a Client, repo: &'a Repo) -> Pulls<'a> {
        Pulls { client, repo }
    }

    pub fn list(&self, filter: &PrFilter<'_>) -> Result<Vec<PullRequest>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(s) = filter.state {
            q.push(("state", s.to_string()));
        }
        if let Some(a) = filter.author {
            q.push(("author", a.to_string()));
        }
        if let Some(a) = filter.assignee {
            q.push(("assignee", a.to_string()));
        }
        if let Some(t) = filter.tester {
            q.push(("tester", t.to_string()));
        }
        let qref = Client::str_refs(&q);
        let path = format!("/repos/{o}/{r}/pulls");
        self.client.get_paged(&path, &qref, filter.limit)
    }

    pub fn get(&self, number: i64) -> Result<PullRequest> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        self.client
            .get(&format!("/repos/{o}/{r}/pulls/{number}"), &[])
    }

    pub fn files(&self, number: i64) -> Result<Vec<FileDiff>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        self.client
            .get(&format!("/repos/{o}/{r}/pulls/{number}/files"), &[])
    }

    pub fn create(&self, req: &CreatePr<'_>) -> Result<PullRequest> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let mut f: Vec<(&str, String)> = vec![
            ("title", req.title.to_string()),
            ("head", req.head.to_string()),
            ("base", req.base.to_string()),
        ];
        if let Some(b) = req.body {
            f.push(("body", b.to_string()));
        }
        if let Some(v) = req.labels {
            f.push(("labels", v.to_string()));
        }
        if let Some(v) = req.assignees {
            f.push(("assignees", v.to_string()));
        }
        if let Some(v) = req.testers {
            f.push(("testers", v.to_string()));
        }
        if let Some(n) = req.milestone_number {
            f.push(("milestone_number", n.to_string()));
        }
        if let Some(i) = req.issue {
            f.push(("issue", i.to_string()));
        }
        if req.close_related_issue {
            f.push(("close_related_issue", "true".to_string()));
        }
        let form = Client::str_refs(&f);
        self.client.post(&format!("/repos/{o}/{r}/pulls"), &form)
    }

    pub fn merge(&self, number: i64, method: MergeMethod, close_related_issue: bool) -> Result<()> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let close = Client::bool_str(close_related_issue);
        let f: Vec<(&str, String)> = vec![
            ("merge_method", method.as_str().to_string()),
            ("close_related_issue", close.to_string()),
        ];
        let form = Client::str_refs(&f);
        self.client
            .put_ok(&format!("/repos/{o}/{r}/pulls/{number}/merge"), &form)
    }

    /// Idempotent merge: if the PR is already merged, return `Already` without
    /// calling the merge endpoint. Otherwise merge and return `Changed`.
    pub fn merge_idempotent(
        &self,
        number: i64,
        method: MergeMethod,
        close_related_issue: bool,
    ) -> Result<StateChange<()>> {
        let cur: PullRequest = self.get(number)?;
        if cur.state == PrState::Merged {
            return Ok(StateChange::Already(()));
        }
        self.merge(number, method, close_related_issue)?;
        Ok(StateChange::Changed(()))
    }

    pub fn comment(
        &self,
        number: i64,
        body: &str,
        positional: &PrCommentPositional<'_>,
    ) -> Result<Comment> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let mut f: Vec<(&str, String)> = vec![("body", body.to_string())];
        if let Some(p) = positional.path {
            f.push(("path", p.to_string()));
        }
        if let Some(pos) = positional.position {
            f.push(("position", pos.to_string()));
        }
        if let Some(c) = positional.commit_id {
            f.push(("commit_id", c.to_string()));
        }
        let form = Client::str_refs(&f);
        self.client
            .post(&format!("/repos/{o}/{r}/pulls/{number}/comments"), &form)
    }

    /// List comments on a pull request. Optional `kind` maps to Gitee's
    /// `comment_type` query (`diff_comment` | `pr_comment`).
    pub fn list_comments(
        &self,
        number: i64,
        filter: &PrCommentFilter,
    ) -> Result<Vec<PrComment>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(k) = filter.kind {
            q.push(("comment_type", k.as_api_str().to_string()));
        }
        let qref = Client::str_refs(&q);
        self.client.get_paged(
            &format!("/repos/{o}/{r}/pulls/{number}/comments"),
            &qref,
            filter.limit,
        )
    }

    /// Resolve `--last`: the comment by `login` with the latest `created_at`.
    /// Paginates fully (independent of list `--limit`). Nothing found ⇒ Usage.
    pub fn latest_comment(&self, number: i64, login: &str) -> Result<PrComment> {
        let comments = self.list_comments(
            number,
            &PrCommentFilter {
                kind: None,
                limit: usize::MAX,
            },
        )?;
        resolve_latest_comment(&comments, login)
            .cloned()
            .ok_or_else(|| {
                GiteeError::Usage(format!(
                    "no comment by '{login}' on pull request {number}"
                ))
            })
    }

    /// PATCH a pull-request comment by integer `id`.
    pub fn update_comment(&self, id: i64, body: &str) -> Result<PrComment> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let f: Vec<(&str, String)> = vec![("body", body.to_string())];
        let form = Client::str_refs(&f);
        self.client
            .patch(&format!("/repos/{o}/{r}/pulls/comments/{id}"), &form)
    }

    /// `--last` edit: resolve `login`'s most-recent comment on the PR, then PATCH.
    pub fn update_latest_comment(
        &self,
        number: i64,
        login: &str,
        body: &str,
    ) -> Result<PrComment> {
        let comment = self.latest_comment(number, login)?;
        self.update_comment(comment.id, body)
    }

    /// DELETE a pull-request comment by integer `id`. Already gone (404) ⇒ `Already`
    /// (idempotent success, silent at the cmd layer).
    pub fn delete_comment(&self, id: i64) -> Result<StateChange<()>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        match self
            .client
            .delete_ok(&format!("/repos/{o}/{r}/pulls/comments/{id}"))
        {
            Ok(()) => Ok(StateChange::Changed(())),
            Err(GiteeError::NotFound(_)) => Ok(StateChange::Already(())),
            Err(e) => Err(e),
        }
    }

    /// `--last` delete: resolve `login`'s most-recent comment on the PR, then DELETE.
    pub fn delete_latest_comment(
        &self,
        number: i64,
        login: &str,
    ) -> Result<StateChange<()>> {
        let comment = self.latest_comment(number, login)?;
        self.delete_comment(comment.id)
    }

    /// Gitee quirk: POST /review returns an empty body on success; `force` is sent only when true.
    pub fn approve(&self, number: i64, force: bool) -> Result<()> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let mut f: Vec<(&str, String)> = Vec::new();
        if force {
            f.push(("force", "true".to_string()));
        }
        let form = Client::str_refs(&f);
        self.client
            .post_ok(&format!("/repos/{o}/{r}/pulls/{number}/review"), &form)
    }

    /// Gitee quirk: POST /test returns an empty body on success; force sent only when true.
    pub fn test(&self, number: i64, force: bool) -> Result<()> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let mut f: Vec<(&str, String)> = Vec::new();
        if force {
            f.push(("force", "true".to_string()));
        }
        let form = Client::str_refs(&f);
        self.client
            .post_ok(&format!("/repos/{o}/{r}/pulls/{number}/test"), &form)
    }

    /// Gitee accepts form-encoded PATCH on pull requests.
    pub fn set_state(&self, number: i64, state: PrState) -> Result<PullRequest> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let f: Vec<(&str, String)> = vec![("state", state.as_str().to_string())];
        let form = Client::str_refs(&f);
        self.client
            .patch(&format!("/repos/{o}/{r}/pulls/{number}"), &form)
    }

    /// Idempotent state change: GET first; if already in `target`, return
    /// `Already` without PATCHing. Otherwise PATCH and return `Changed`.
    pub fn set_state_idempotent(
        &self,
        number: i64,
        target: PrState,
    ) -> Result<StateChange<PullRequest>> {
        let cur: PullRequest = self.get(number)?;
        if cur.state == target {
            return Ok(StateChange::Already(cur));
        }
        let pr = self.set_state(number, target)?;
        Ok(StateChange::Changed(pr))
    }

    /// PATCH metadata. Only `Some` fields become form entries.
    pub fn edit(&self, number: i64, req: &EditPr<'_>) -> Result<PullRequest> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let mut f: Vec<(&str, String)> = Vec::new();
        if let Some(v) = req.title {
            f.push(("title", v.to_string()));
        }
        if let Some(v) = req.body {
            f.push(("body", v.to_string()));
        }
        if let Some(v) = req.labels {
            f.push(("labels", v.to_string()));
        }
        if let Some(v) = req.assignees {
            f.push(("assignees", v.to_string()));
        }
        if let Some(v) = req.testers {
            f.push(("testers", v.to_string()));
        }
        if let Some(n) = req.milestone_number {
            f.push(("milestone_number", n.to_string()));
        }
        let form = Client::str_refs(&f);
        self.client
            .patch(&format!("/repos/{o}/{r}/pulls/{number}"), &form)
    }

    /// Labels currently attached to a PR. Uses `get_paged` (endpoint is paginated).
    pub fn list_labels(&self, number: i64) -> Result<Vec<Label>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        self.client.get_paged(
            &format!("/repos/{o}/{r}/pulls/{number}/labels"),
            &[],
            usize::MAX,
        )
    }

    /// Add labels without replacing the rest. GETs current membership first;
    /// POSTs only names that are missing. Already-present ⇒ `Already` (no POST).
    pub fn add_labels_idempotent(
        &self,
        number: i64,
        names: &[&str],
    ) -> Result<StateChange<Vec<Label>>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let path = format!("/repos/{o}/{r}/pulls/{number}/labels");
        let current = self.list_labels(number)?;
        let present: HashSet<String> = current.iter().map(|l| l.name.clone()).collect();
        let mut seen = HashSet::new();
        let missing: Vec<&str> = names
            .iter()
            .copied()
            .filter(|n| !present.contains(*n) && seen.insert(*n))
            .collect();
        if missing.is_empty() {
            return Ok(StateChange::Already(current));
        }
        let body = serde_json::Value::Array(
            missing
                .iter()
                .map(|n| serde_json::Value::String((*n).to_string()))
                .collect(),
        );
        let labels: Vec<Label> = self.client.post_json(&path, &body)?;
        Ok(StateChange::Changed(labels))
    }

    /// Remove only the named labels. GETs current membership first; DELETEs
    /// only names that are present. Absent names and DELETE 404 ⇒ no-op.
    pub fn remove_labels_idempotent(
        &self,
        number: i64,
        names: &[&str],
    ) -> Result<StateChange<()>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let current = self.list_labels(number)?;
        let present: HashSet<String> = current.iter().map(|l| l.name.clone()).collect();
        let mut seen = HashSet::new();
        let to_remove: Vec<&str> = names
            .iter()
            .copied()
            .filter(|n| present.contains(*n) && seen.insert(*n))
            .collect();
        if to_remove.is_empty() {
            return Ok(StateChange::Already(()));
        }
        let mut changed = false;
        for name in to_remove {
            match self
                .client
                .delete_ok(&format!("/repos/{o}/{r}/pulls/{number}/labels/{name}"))
            {
                Ok(()) => changed = true,
                Err(GiteeError::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }
        if changed {
            Ok(StateChange::Changed(()))
        } else {
            Ok(StateChange::Already(()))
        }
    }

    /// GET the PR first; if `body` already contains `tag`, returns `Ok(false)` without PATCH.
    /// Otherwise PATCH form `body` with appended `Linked: {tag}` and returns `Ok(true)`.
    pub fn link(&self, number: i64, tag: &str) -> Result<bool> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let pr: PullRequest = self
            .client
            .get(&format!("/repos/{o}/{r}/pulls/{number}"), &[])?;
        let cur = pr.body.clone().unwrap_or_default();
        if cur.contains(tag) {
            return Ok(false);
        }
        let new = format!("{cur}\n\nLinked: {tag}");
        let f: Vec<(&str, String)> = vec![("body", new)];
        let form = Client::str_refs(&f);
        let _: PullRequest = self
            .client
            .patch(&format!("/repos/{o}/{r}/pulls/{number}"), &form)?;
        Ok(true)
    }
}
