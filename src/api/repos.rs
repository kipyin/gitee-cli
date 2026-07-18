use super::client::Client;
use crate::error::{GiteeError, Result};
use crate::models::{Milestone, RepoDetails};
use base64::Engine;

pub struct Repos<'a> {
    client: &'a Client,
}

pub struct CreateRepo<'a> {
    pub name: &'a str,
    pub org: Option<&'a str>,
    pub description: Option<&'a str>,
    pub homepage: Option<&'a str>,
    pub gitignore_template: Option<&'a str>,
    pub license_template: Option<&'a str>,
    pub private: bool,
}

pub struct EditRepo<'a> {
    /// Current repository name (required by Gitee on every PATCH).
    pub name: &'a str,
    pub description: Option<&'a str>,
    pub homepage: Option<&'a str>,
    pub private: Option<bool>,
    pub default_branch: Option<&'a str>,
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

    /// Create a repository for the authenticated user or an organization.
    pub fn create(&self, req: &CreateRepo<'_>) -> Result<RepoDetails> {
        let path = match req.org {
            Some(org) => format!("/orgs/{org}/repos"),
            None => "/user/repos".to_string(),
        };
        let mut f: Vec<(&str, String)> = vec![("name", req.name.to_string())];
        if let Some(d) = req.description {
            f.push(("description", d.to_string()));
        }
        if let Some(h) = req.homepage {
            f.push(("homepage", h.to_string()));
        }
        if let Some(g) = req.gitignore_template {
            f.push(("gitignore_template", g.to_string()));
        }
        if let Some(l) = req.license_template {
            f.push(("license_template", l.to_string()));
        }
        // Gitee bool quirk: booleans are urlencoded as "true"/"false" strings.
        // Swagger also documents integer `public`; we send `private` instead.
        f.push(("private", Client::bool_str(req.private).to_string()));
        let form = Client::str_refs(&f);
        self.client.post(&path, &form)
    }

    /// Update repository settings. Gitee quirk: `name` is required on every PATCH
    /// even when only changing other fields — callers must pass the current name.
    pub fn edit(&self, owner: &str, repo: &str, req: &EditRepo<'_>) -> Result<RepoDetails> {
        let mut f: Vec<(&str, String)> = vec![("name", req.name.to_string())];
        if let Some(d) = req.description {
            f.push(("description", d.to_string()));
        }
        if let Some(h) = req.homepage {
            f.push(("homepage", h.to_string()));
        }
        if let Some(p) = req.private {
            f.push(("private", Client::bool_str(p).to_string()));
        }
        if let Some(b) = req.default_branch {
            f.push(("default_branch", b.to_string()));
        }
        let form = Client::str_refs(&f);
        self.client.patch(&format!("/repos/{owner}/{repo}"), &form)
    }

    /// Rename a repository slug. Gitee quirk: `path` is the URL slug (`owner/path`),
    /// while `name` is the display name; both are sent on PATCH. Live probe
    /// (2026-07-18): PATCH updates the slug, but GET on the old slug still
    /// resolves to the repo (canonical `full_name` reflects the new path).
    pub fn rename(
        &self,
        owner: &str,
        repo: &str,
        current_name: &str,
        new_path: &str,
    ) -> Result<RepoDetails> {
        let f: Vec<(&str, String)> = vec![
            ("name", current_name.to_string()),
            ("path", new_path.to_string()),
        ];
        let form = Client::str_refs(&f);
        self.client.patch(&format!("/repos/{owner}/{repo}"), &form)
    }

    pub fn delete(&self, owner: &str, name: &str) -> Result<()> {
        self.client
            .delete_ok(&format!("/repos/{owner}/{name}"))
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
