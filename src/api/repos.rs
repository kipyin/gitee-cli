use super::client::Client;
use crate::error::{GiteeError, Result};
use crate::models::{Milestone, RepoDetails};
use base64::Engine;

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

    /// All milestones of a repo (used to resolve --milestone titles to numbers).
    pub fn list_milestones(&self, owner: &str, name: &str) -> Result<Vec<Milestone>> {
        self.client
            .get(&format!("/repos/{owner}/{name}/milestones"), &[])
    }

    /// Fetch a file's text via the contents API (GET /repos/{o}/{r}/contents/{path}?ref=).
    /// `Ok(None)` when the path does not exist (404) or is not a file.
    pub fn file_contents(
        &self,
        owner: &str,
        name: &str,
        path: &str,
        git_ref: &str,
    ) -> Result<Option<String>> {
        let q = [("ref", git_ref)];
        let result: Result<serde_json::Value> = self
            .client
            .get(&format!("/repos/{owner}/{name}/contents/{path}"), &q);
        let value = match result {
            Ok(v) => v,
            Err(GiteeError::NotFound(_)) => return Ok(None),
            Err(e) => return Err(e),
        };
        // A file path returns one object; a directory returns an array of entries.
        let content = value.get("content").and_then(|c| c.as_str());
        match content {
            Some(c) => Ok(Some(decode_contents(c)?)),
            None => Ok(None),
        }
    }
}

/// Contents-API payloads are base64 with embedded newlines; strip whitespace first.
fn decode_contents(content: &str) -> Result<String> {
    let compact: String = content.chars().filter(|c| !c.is_whitespace()).collect();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&compact)
        .map_err(|e| GiteeError::Api {
            status: 200,
            message: format!("contents API returned invalid base64: {e}"),
        })?;
    String::from_utf8(bytes).map_err(|e| GiteeError::Api {
        status: 200,
        message: format!("contents API returned non-UTF-8 content: {e}"),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn decode_contents_strips_newlines() {
        // "hello gitee" base64, wrapped as the API wraps payloads.
        let wrapped = "aGVsb\nG8gZ2l0\nZWU=\n";
        assert_eq!(super::decode_contents(wrapped).unwrap(), "hello gitee");
    }

    #[test]
    fn decode_contents_rejects_garbage() {
        assert!(super::decode_contents("!!!not-base64!!!").is_err());
    }
}
