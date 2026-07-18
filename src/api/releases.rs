use super::client::Client;
use crate::error::Result;
use crate::models::{Release, ReleaseAsset};
use crate::repo::Repo;

pub struct Releases<'a> {
    client: &'a Client,
    repo: &'a Repo,
}

pub struct CreateRelease<'a> {
    pub tag: &'a str,
    pub name: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub target: Option<&'a str>,
    pub prerelease: bool,
}

impl Releases<'_> {
    pub(crate) fn new<'a>(client: &'a Client, repo: &'a Repo) -> Releases<'a> {
        Releases { client, repo }
    }

    pub fn list(&self, limit: usize) -> Result<Vec<Release>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let path = format!("/repos/{o}/{r}/releases");
        self.client.get_paged(&path, &[], limit)
    }

    /// Gitee quirk: a missing release returns HTTP 200 with a JSON `null`
    /// body (not 404). Deserialize as Option and map null to NotFound.
    pub fn get_by_tag(&self, tag: &str) -> Result<Release> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let rel: Option<Release> = self
            .client
            .get(&format!("/repos/{o}/{r}/releases/tags/{tag}"), &[])?;
        rel.ok_or_else(|| {
            crate::error::GiteeError::NotFound(format!("release {tag}"))
        })
    }

    /// Gitee quirks: `body` is REQUIRED and must be non-empty (400 otherwise),
    /// so it defaults to the display name; `prerelease` is always sent as
    /// `"true"` or `"false"`; `name` defaults to `tag`.
    pub fn create(&self, req: &CreateRelease<'_>) -> Result<Release> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let display_name = req.name.unwrap_or(req.tag);
        let mut f: Vec<(&str, String)> = vec![
            ("tag_name", req.tag.to_string()),
            ("name", display_name.to_string()),
            ("body", req.notes.unwrap_or(display_name).to_string()),
        ];
        if let Some(t) = req.target {
            f.push(("target_commitish", t.to_string()));
        }
        f.push((
            "prerelease",
            if req.prerelease {
                "true".to_string()
            } else {
                "false".to_string()
            },
        ));
        let form = Client::str_refs(&f);
        self.client.post(&format!("/repos/{o}/{r}/releases"), &form)
    }

    pub fn upload(&self, tag: &str, file_path: &str) -> Result<ReleaseAsset> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let release = self.get_by_tag(tag)?;
        let id = release.id;
        self.client.post_multipart(
            &format!("/repos/{o}/{r}/releases/{id}/attach_files"),
            file_path,
        )
    }
}
