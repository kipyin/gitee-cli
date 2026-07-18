use super::client::Client;
use crate::error::Result;
use crate::models::RepoDetails;

pub struct Repos<'a> {
    client: &'a Client,
}

impl Repos<'_> {
    pub(crate) fn new(client: &Client) -> Repos<'_> {
        Repos { client }
    }

    pub fn get(&self, owner: &str, name: &str) -> Result<RepoDetails> {
        self.client.get(&format!("/repos/{owner}/{name}"), &[])
    }

    pub fn list_mine(&self, limit: usize) -> Result<Vec<RepoDetails>> {
        self.client.get_paged("/user/repos", &[], limit)
    }

    pub fn list_user(&self, owner: &str, limit: usize) -> Result<Vec<RepoDetails>> {
        let path = format!("/users/{owner}/repos");
        self.client.get_paged(&path, &[], limit)
    }

    pub fn fork(&self, owner: &str, name: &str) -> Result<RepoDetails> {
        self.client
            .post(&format!("/repos/{owner}/{name}/forks"), &[])
    }
}
