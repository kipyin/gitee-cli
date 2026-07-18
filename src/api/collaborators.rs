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
        let (o, r) = (&self.repo.owner, &self.repo.name);
        self.client
            .get_paged(&format!("/repos/{o}/{r}/collaborators"), &[], limit)
    }

    /// Permission vocabulary: `pull` | `push` | `admin` (English enums per Gitee v5 docs / ticket 19).
    pub fn add(&self, username: &str, permission: &str) -> Result<()> {
        let (o, r) = (&self.repo.owner, &self.repo.name);
        self.client.put_ok(
            &format!("/repos/{o}/{r}/collaborators/{username}"),
            &[("permission", permission)],
        )
    }

    pub fn remove(&self, username: &str) -> Result<()> {
        let (o, r) = (&self.repo.owner, &self.repo.name);
        self.client
            .delete_ok(&format!("/repos/{o}/{r}/collaborators/{username}"))
    }
}
