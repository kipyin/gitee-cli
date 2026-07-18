use super::client::Client;
use crate::error::Result;
use crate::models::{Issue, UserBasic};

/// User-level operations (no repo scope): the authenticated user and their
/// cross-repo issue lists.
pub struct Users<'a> {
    client: &'a Client,
}

/// Filter for `GET /user/issues`. `filter` selects the list (Gitee requires
/// it): `assigned` | `created` | `all`.
pub struct UserIssueFilter<'a> {
    pub filter: &'a str,
    pub state: Option<&'a str>,
    pub limit: usize,
}

impl Users<'_> {
    pub(crate) fn new<'a>(client: &'a Client) -> Users<'a> {
        Users { client }
    }

    /// The authenticated user (GET /user).
    pub fn me(&self) -> Result<UserBasic> {
        self.client.get("/user", &[])
    }

    /// Cross-repo issues for the authenticated user (GET /user/issues).
    pub fn issues(&self, filter: &UserIssueFilter<'_>) -> Result<Vec<Issue>> {
        let mut q: Vec<(&str, String)> = vec![("filter", filter.filter.to_string())];
        if let Some(s) = filter.state {
            q.push(("state", s.to_string()));
        }
        let qref = Client::str_refs(&q);
        self.client.get_paged("/user/issues", &qref, filter.limit)
    }
}
