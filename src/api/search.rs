use super::client::Client;
use crate::error::Result;
use crate::models::{Issue, RepoDetails, UserBasic};

// OMIT search code, search commits, search prs — swagger has NO /search/code or
// /search/commits, and /search/issues has no PR type filter (verified 2026-07-18).

pub struct Search<'a> {
    client: &'a Client,
}

pub struct SearchReposFilter<'a> {
    pub q: &'a str,
    pub owner: Option<&'a str>,
    pub language: Option<&'a str>,
    pub fork: bool,
    pub sort: Option<&'a str>,
    pub order: Option<&'a str>,
    pub limit: usize,
}

pub struct SearchIssuesFilter<'a> {
    pub q: &'a str,
    pub repo: Option<&'a str>,
    pub language: Option<&'a str>,
    pub label: Option<&'a str>,
    pub state: Option<&'a str>,
    pub author: Option<&'a str>,
    pub assignee: Option<&'a str>,
    pub sort: Option<&'a str>,
    pub order: Option<&'a str>,
    pub limit: usize,
}

pub struct SearchUsersFilter<'a> {
    pub q: &'a str,
    pub sort: Option<&'a str>,
    pub order: Option<&'a str>,
    pub limit: usize,
}

impl Search<'_> {
    pub(crate) fn new(client: &Client) -> Search<'_> {
        Search { client }
    }

    pub fn repos(&self, filter: &SearchReposFilter<'_>) -> Result<Vec<RepoDetails>> {
        let mut q: Vec<(&str, String)> = vec![("q", filter.q.to_string())];
        if let Some(v) = filter.owner {
            q.push(("owner", v.to_string()));
        }
        if let Some(v) = filter.language {
            q.push(("language", v.to_string()));
        }
        if filter.fork {
            q.push(("fork", "true".to_string()));
        }
        if let Some(v) = filter.sort {
            q.push(("sort", v.to_string()));
        }
        if let Some(v) = filter.order {
            q.push(("order", v.to_string()));
        }
        let qref = Client::str_refs(&q);
        self.client
            .get_paged("/search/repositories", &qref, filter.limit)
    }

    pub fn issues(&self, filter: &SearchIssuesFilter<'_>) -> Result<Vec<Issue>> {
        let mut q: Vec<(&str, String)> = vec![("q", filter.q.to_string())];
        if let Some(v) = filter.repo {
            q.push(("repo", v.to_string()));
        }
        if let Some(v) = filter.language {
            q.push(("language", v.to_string()));
        }
        if let Some(v) = filter.label {
            q.push(("label", v.to_string()));
        }
        if let Some(v) = filter.state {
            q.push(("state", v.to_string()));
        }
        if let Some(v) = filter.author {
            q.push(("author", v.to_string()));
        }
        if let Some(v) = filter.assignee {
            q.push(("assignee", v.to_string()));
        }
        if let Some(v) = filter.sort {
            q.push(("sort", v.to_string()));
        }
        if let Some(v) = filter.order {
            q.push(("order", v.to_string()));
        }
        let qref = Client::str_refs(&q);
        self.client.get_paged("/search/issues", &qref, filter.limit)
    }

    pub fn users(&self, filter: &SearchUsersFilter<'_>) -> Result<Vec<UserBasic>> {
        let mut q: Vec<(&str, String)> = vec![("q", filter.q.to_string())];
        if let Some(v) = filter.sort {
            q.push(("sort", v.to_string()));
        }
        if let Some(v) = filter.order {
            q.push(("order", v.to_string()));
        }
        let qref = Client::str_refs(&q);
        self.client.get_paged("/search/users", &qref, filter.limit)
    }
}
