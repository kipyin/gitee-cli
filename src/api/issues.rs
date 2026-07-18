use super::client::Client;
use crate::error::Result;
use crate::models::{Comment, Issue, IssueState};
use crate::repo::Repo;

pub struct Issues<'a> {
    client: &'a Client,
    repo: &'a Repo,
}

pub struct IssueFilter<'a> {
    pub state: Option<&'a str>,
    pub assignee: Option<&'a str>,
    pub limit: usize,
}

pub struct CreateIssue<'a> {
    pub title: &'a str,
    pub body: Option<&'a str>,
    pub assignee: Option<&'a str>,
    pub labels: Option<&'a str>,
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
    }

    pub fn comment(&self, number: &str, body: &str) -> Result<Comment> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let f: Vec<(&str, String)> = vec![("body", body.to_string())];
        let form = Client::str_refs(&f);
        self.client
            .post(&format!("/repos/{o}/{r}/issues/{number}/comments"), &form)
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
