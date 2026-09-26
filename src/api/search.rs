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
        Client::push_some(&mut q, "owner", filter.owner);
        Client::push_some(&mut q, "language", filter.language);
        if filter.fork {
            q.push(("fork", "true".to_string()));
        }
        Client::push_some(&mut q, "sort", filter.sort);
        Client::push_some(&mut q, "order", filter.order);
        let qref = Client::str_refs(&q);
        self.client
            .get_paged("/search/repositories", &qref, filter.limit)
    }

    pub fn issues(&self, filter: &SearchIssuesFilter<'_>) -> Result<Vec<Issue>> {
        let mut q: Vec<(&str, String)> = vec![("q", filter.q.to_string())];
        Client::push_some(&mut q, "repo", filter.repo);
        Client::push_some(&mut q, "language", filter.language);
        Client::push_some(&mut q, "label", filter.label);
        Client::push_some(&mut q, "state", filter.state);
        Client::push_some(&mut q, "author", filter.author);
        Client::push_some(&mut q, "assignee", filter.assignee);
        Client::push_some(&mut q, "sort", filter.sort);
        Client::push_some(&mut q, "order", filter.order);
        let qref = Client::str_refs(&q);
        self.client.get_paged("/search/issues", &qref, filter.limit)
    }

    pub fn users(&self, filter: &SearchUsersFilter<'_>) -> Result<Vec<UserBasic>> {
        let mut q: Vec<(&str, String)> = vec![("q", filter.q.to_string())];
        Client::push_some(&mut q, "sort", filter.sort);
        Client::push_some(&mut q, "order", filter.order);
        let qref = Client::str_refs(&q);
        self.client.get_paged("/search/users", &qref, filter.limit)
    }
}
