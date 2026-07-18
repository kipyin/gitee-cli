use super::client::Client;
use crate::error::Result;
use crate::models::{Issue, Org, SshKey, UserBasic};

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

    /// Organizations for the authenticated user (GET /user/orgs).
    pub fn orgs(&self, limit: usize) -> Result<Vec<Org>> {
        self.client.get_paged("/user/orgs", &[], limit)
    }

    /// SSH keys for the authenticated user (GET /user/keys).
    pub fn keys(&self, limit: usize) -> Result<Vec<SshKey>> {
        self.client.get_paged("/user/keys", &[], limit)
    }

    /// Add an SSH public key (POST /user/keys).
    pub fn add_key(&self, key: &str, title: &str) -> Result<SshKey> {
        self.client
            .post("/user/keys", &[("key", key), ("title", title)])
    }

    /// Delete an SSH key (DELETE /user/keys/{id}).
    pub fn delete_key(&self, id: i64) -> Result<()> {
        self.client.delete_ok(&format!("/user/keys/{id}"))
    }
}
