use super::client::Client;
use crate::api::{resolve_latest_comment, StateChange};
use crate::error::{GiteeError, Result};
use crate::models::{Comment, Issue, IssueState, Label};
use crate::repo::Repo;
use std::collections::HashSet;

/// When a state write hits Gitee's opaque enterprise/project 404, surface a
/// clearer hint than a bare path. Non-state edits keep the original error.
fn map_issue_state_err(err: GiteeError, changing_state: bool, owner: &str, number: &str) -> GiteeError {
    if !changing_state {
        return err;
    }
    let enterprise = matches!(
        &err,
        GiteeError::Api { message, .. } if message.to_lowercase().contains("project or enterprise")
    );
    if !enterprise {
        return err;
    }
    GiteeError::Api {
        status: 404,
        message: format!(
            "could not change issue {number} state (HTTP 404). Tried PATCH \
             /repos/{owner}/issues/{number} with JSON {{repo, title, state}}. \
             If this is an enterprise/project board issue, that personal/org \
             endpoint may not apply — check --repo/--remote resolve to the \
             right repository and that your token can access it. For raw \
             `gitee api`, use that owner path with a JSON body (not form \
             fields on /repos/{{owner}}/{{repo}}/issues/{{number}})."
        ),
    }
}

pub struct Issues<'a> {
    client: &'a Client,
    repo: &'a Repo,
}

#[derive(Default)]
pub struct IssueFilter<'a> {
    pub state: Option<&'a str>,
    pub assignee: Option<&'a str>,
    /// Creator login — server-side filter per the v5 swagger.
    pub creator: Option<&'a str>,
    pub limit: usize,
}

#[derive(Default)]
pub struct CreateIssue<'a> {
    pub title: &'a str,
    pub body: Option<&'a str>,
    pub assignee: Option<&'a str>,
    pub labels: Option<&'a str>,
    pub milestone_number: Option<i64>,
    pub security_hole: bool,
}

/// Fields for `issue edit`. All optional; only `Some` fields are sent. `labels`
/// arrives comma-joined from the handler. Param names per the v5 swagger
/// (milestone takes the milestone number as integer).
#[derive(Default)]
pub struct EditIssue<'a> {
    pub title: Option<&'a str>,
    pub body: Option<&'a str>,
    pub assignee: Option<&'a str>,
    pub labels: Option<&'a str>,
    pub milestone_number: Option<i64>,
    pub security_hole: Option<bool>,
    /// Lifecycle state (`open` / `progressing` / `closed` / `rejected`).
    pub state: Option<IssueState>,
}

impl Issues<'_> {
    pub(crate) fn new<'a>(client: &'a Client, repo: &'a Repo) -> Issues<'a> {
        Issues { client, repo }
    }

    pub fn list(&self, filter: &IssueFilter<'_>) -> Result<Vec<Issue>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let mut q: Vec<(&str, String)> = Vec::new();
        if let Some(s) = filter.state {
            q.push(("state", s.to_string()));
        }
        if let Some(a) = filter.assignee {
            q.push(("assignee", a.to_string()));
        }
        if let Some(c) = filter.creator {
            q.push(("creator", c.to_string()));
        }
        let qref = Client::str_refs(&q);
        let path = format!("/repos/{o}/{r}/issues");
        self.client.get_paged(&path, &qref, filter.limit)
    }

    pub fn get(&self, number: &str) -> Result<Issue> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        self.client
            .get(&format!("/repos/{o}/{r}/issues/{number}"), &[])
    }

    /// Gitee quirk: issue creation is POST /repos/{owner}/issues with `repo` as a form field
    /// (no repo segment in the path).
    pub fn create(&self, req: &CreateIssue<'_>) -> Result<Issue> {
        let o = self.repo.owner.as_str();
        let mut f: Vec<(&str, String)> = vec![
            ("repo", self.repo.name.clone()),
            ("title", req.title.to_string()),
        ];
        if let Some(b) = req.body {
            f.push(("body", b.to_string()));
        }
        if let Some(a) = req.assignee {
            f.push(("assignee", a.to_string()));
        }
        if let Some(l) = req.labels {
            f.push(("labels", l.to_string()));
        }
        if let Some(n) = req.milestone_number {
            f.push(("milestone", n.to_string()));
        }
        if req.security_hole {
            f.push(("security_hole", "true".to_string()));
        }
        let form = Client::str_refs(&f);
        self.client.post(&format!("/repos/{o}/issues"), &form)
    }

    /// Gitee quirk: state changes are PATCH /repos/{owner}/issues/{number} with a JSON body
    /// `{repo, title, state}`. The current title must be echoed back or Gitee blanks it.
    pub fn set_state(&self, number: &str, state: IssueState) -> Result<Issue> {
        let o = self.repo.owner.as_str();
        let name = &self.repo.name;
        let cur: Issue = self
            .client
            .get(&format!("/repos/{o}/{name}/issues/{number}"), &[])?;
        let body = serde_json::json!({
            "repo": self.repo.name,
            "title": cur.title,
            "state": state.as_str(),
        });
        self.client
            .patch_json(&format!("/repos/{o}/issues/{number}"), &body)
            .map_err(|e| map_issue_state_err(e, true, o, number))
    }

    /// Idempotent state change: GETs the current issue first; if it is already
    /// in `target`, returns `StateChange::Already(issue)` without a PATCH.
    /// Otherwise applies the PATCH and returns `StateChange::Changed(issue)`.
    pub fn set_state_idempotent(
        &self,
        number: &str,
        target: IssueState,
    ) -> Result<StateChange<Issue>> {
        let o = self.repo.owner.as_str();
        let name = &self.repo.name;
        let cur: Issue = self
            .client
            .get(&format!("/repos/{o}/{name}/issues/{number}"), &[])?;
        if cur.state == target {
            return Ok(StateChange::Already(cur));
        }
        let body = serde_json::json!({
            "repo": self.repo.name,
            "title": cur.title,
            "state": target.as_str(),
        });
        let issue: Issue = self
            .client
            .patch_json(&format!("/repos/{o}/issues/{number}"), &body)
            .map_err(|e| map_issue_state_err(e, true, o, number))?;
        Ok(StateChange::Changed(issue))
    }

    /// PATCH metadata. Same JSON quirk as set_state: `repo` and the current
    /// `title` must always be echoed; only `Some` fields are added.
    pub fn edit(&self, number: &str, req: &EditIssue<'_>) -> Result<Issue> {
        let o = self.repo.owner.as_str();
        let name = &self.repo.name;
        let cur: Issue = self
            .client
            .get(&format!("/repos/{o}/{name}/issues/{number}"), &[])?;
        let mut body = serde_json::json!({
            "repo": self.repo.name,
            "title": req.title.unwrap_or(&cur.title),
        });
        let map = body.as_object_mut().expect("json object");
        if let Some(v) = req.body {
            map.insert("body".into(), v.into());
        }
        if let Some(v) = req.assignee {
            map.insert("assignee".into(), v.into());
        }
        if let Some(v) = req.labels {
            map.insert("labels".into(), v.into());
        }
        if let Some(n) = req.milestone_number {
            map.insert("milestone".into(), n.into());
        }
        if let Some(b) = req.security_hole {
            map.insert("security_hole".into(), b.into());
        }
        if let Some(s) = req.state {
            map.insert("state".into(), s.as_str().into());
        }
        self.client
            .patch_json(&format!("/repos/{o}/issues/{number}"), &body)
            .map_err(|e| map_issue_state_err(e, req.state.is_some(), o, number))
    }

    pub fn comment(&self, number: &str, body: &str) -> Result<Comment> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let f: Vec<(&str, String)> = vec![("body", body.to_string())];
        let form = Client::str_refs(&f);
        self.client
            .post(&format!("/repos/{o}/{r}/issues/{number}/comments"), &form)
    }

    /// List comments on an issue. `limit` caps how many are returned (paging
    /// via `get_paged`).
    pub fn list_comments(&self, number: &str, limit: usize) -> Result<Vec<Comment>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        self.client.get_paged(
            &format!("/repos/{o}/{r}/issues/{number}/comments"),
            &[],
            limit,
        )
    }

    /// Resolve `--last`: the comment by `login` with the latest `created_at`.
    /// Paginates fully (independent of list `--limit`). Nothing found ⇒ Usage.
    pub fn latest_comment(&self, number: &str, login: &str) -> Result<Comment> {
        let comments = self.list_comments(number, usize::MAX)?;
        resolve_latest_comment(&comments, login)
            .cloned()
            .ok_or_else(|| {
                GiteeError::Usage(format!(
                    "no comment by '{login}' on issue {number}"
                ))
            })
    }

    /// PATCH an issue comment by integer `id`.
    pub fn update_comment(&self, id: i64, body: &str) -> Result<Comment> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let f: Vec<(&str, String)> = vec![("body", body.to_string())];
        let form = Client::str_refs(&f);
        self.client
            .patch(&format!("/repos/{o}/{r}/issues/comments/{id}"), &form)
    }

    /// `--last` edit: resolve `login`'s most-recent comment on the issue, then PATCH.
    pub fn update_latest_comment(
        &self,
        number: &str,
        login: &str,
        body: &str,
    ) -> Result<Comment> {
        let comment = self.latest_comment(number, login)?;
        self.update_comment(comment.id, body)
    }

    /// DELETE an issue comment by integer `id`. Already gone (404) ⇒ `Already`
    /// (idempotent success, silent at the cmd layer).
    pub fn delete_comment(&self, id: i64) -> Result<StateChange<()>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        match self
            .client
            .delete_ok(&format!("/repos/{o}/{r}/issues/comments/{id}"))
        {
            Ok(()) => Ok(StateChange::Changed(())),
            Err(GiteeError::NotFound(_)) => Ok(StateChange::Already(())),
            Err(e) => Err(e),
        }
    }

    /// `--last` delete: resolve `login`'s most-recent comment on the issue, then DELETE.
    pub fn delete_latest_comment(
        &self,
        number: &str,
        login: &str,
    ) -> Result<StateChange<()>> {
        let comment = self.latest_comment(number, login)?;
        self.delete_comment(comment.id)
    }

    /// Labels currently attached to an issue. Gitee's issue-labels GET is not
    /// paginated (unlike PR labels).
    pub fn list_labels(&self, number: &str) -> Result<Vec<Label>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        self.client
            .get(&format!("/repos/{o}/{r}/issues/{number}/labels"), &[])
    }

    /// Add labels without replacing the rest. GETs current membership first;
    /// POSTs only names that are missing. Already-present ⇒ `Already` (no POST).
    pub fn add_labels_idempotent(
        &self,
        number: &str,
        names: &[&str],
    ) -> Result<StateChange<Vec<Label>>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let path = format!("/repos/{o}/{r}/issues/{number}/labels");
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
        number: &str,
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
                .delete_ok(&format!("/repos/{o}/{r}/issues/{number}/labels/{name}"))
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

    /// GET the issue first; if `body` already contains `tag`, returns `Ok(false)` without PATCH.
    /// Otherwise PATCH JSON `{repo, title, body}` with appended `Linked: {tag}` and returns `Ok(true)`.
    pub fn link(&self, number: &str, tag: &str) -> Result<bool> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let cur: Issue = self
            .client
            .get(&format!("/repos/{o}/{r}/issues/{number}"), &[])?;
        let old = cur.body.clone().unwrap_or_default();
        if old.contains(tag) {
            return Ok(false);
        }
        let new = format!("{old}\n\nLinked: {tag}");
        let body = serde_json::json!({
            "repo": self.repo.name,
            "title": cur.title,
            "body": new,
        });
        let _: Issue = self
            .client
            .patch_json(&format!("/repos/{o}/issues/{number}"), &body)?;
        Ok(true)
    }
}
