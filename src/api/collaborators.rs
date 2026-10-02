use super::client::Client;
use crate::error::Result;
use crate::models::Collaborator;
use crate::repo::Repo;

pub struct Collaborators<'a> {
    client: &'a Client,
    repo: &'a Repo,
}

impl Collaborators<'_> {
    pub(crate) fn new<'a>(client: &'a Client, repo: &'a Repo) -> Collaborators<'a> {
        Collaborators { client, repo }
    }

    pub fn list(&self, limit: usize) -> Result<Vec<Collaborator>> {
        self.client
            .get_paged(&self.repo.api_path("collaborators"), &[], limit)
    }

    /// Permission vocabulary: `pull` | `push` | `admin` (English enums per Gitee v5 docs).
    pub fn add(&self, username: &str, permission: &str) -> Result<()> {
        self.client.put_ok(
            &self.repo.api_path(format!("collaborators/{username}")),
            &[("permission", permission)],
        )
    }

    pub fn remove(&self, username: &str) -> Result<()> {
        self.client
            .delete_ok(&self.repo.api_path(format!("collaborators/{username}")))
    }
}
