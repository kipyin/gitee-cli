use super::client::Client;
use crate::error::Result;
use crate::models::{Comment, FileDiff, MergeMethod, PrState, PullRequest};
use crate::repo::Repo;

pub struct Pulls<'a> {
    client: &'a Client,
    repo: &'a Repo,
}

pub struct PrFilter<'a> {
    pub state: Option<&'a str>,
    pub author: Option<&'a str>,
    pub limit: usize,
}

pub struct CreatePr<'a> {
    pub title: &'a str,
    pub head: &'a str,
    pub base: &'a str,
    pub body: Option<&'a str>,
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
        let form = Client::str_refs(&f);
        self.client.post(&format!("/repos/{o}/{r}/pulls"), &form)
    }

    pub fn merge(&self, number: i64, method: MergeMethod, close_related_issue: bool) -> Result<()> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let close = if close_related_issue { "true" } else { "false" };
        let f: Vec<(&str, String)> = vec![
            ("merge_method", method.as_str().to_string()),
            ("close_related_issue", close.to_string()),
        ];
        let form = Client::str_refs(&f);
        self.client
            .put_ok(&format!("/repos/{o}/{r}/pulls/{number}/merge"), &form)
    }

    pub fn comment(&self, number: i64, body: &str) -> Result<Comment> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let f: Vec<(&str, String)> = vec![("body", body.to_string())];
        let form = Client::str_refs(&f);
        self.client
            .post(&format!("/repos/{o}/{r}/pulls/{number}/comments"), &form)
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

    /// Gitee accepts form-encoded PATCH on pull requests.
    pub fn set_state(&self, number: i64, state: PrState) -> Result<PullRequest> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let f: Vec<(&str, String)> = vec![("state", state.as_str().to_string())];
        let form = Client::str_refs(&f);
        self.client
            .patch(&format!("/repos/{o}/{r}/pulls/{number}"), &form)
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
