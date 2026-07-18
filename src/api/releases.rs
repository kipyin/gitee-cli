use super::client::Client;
use crate::error::Result;
use crate::models::{Release, ReleaseAsset};
use crate::repo::Repo;

pub struct Releases<'a> {
    client: &'a Client,
    repo: &'a Repo,
}


pub struct EditRelease<'a> {
    pub name: Option<&'a str>,
    pub notes: Option<&'a str>,
    pub prerelease: Option<bool>,
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
        f.push(("prerelease", Client::bool_str(req.prerelease).to_string()));
        let form = Client::str_refs(&f);
        self.client.post(&format!("/repos/{o}/{r}/releases"), &form)
    }


    /// Gitee quirk (swagger 2026-07-18): PATCH requires `tag_name`, `name`, and
    /// `body` on every request — GET-by-tag first, then send the flag value or
    /// the current value for all three. `prerelease` is sent only when requested.
    /// `--latest` omitted: PATCH /releases/{id} has no latest param (swagger 2026-07-18).
    pub fn edit(&self, tag: &str, req: &EditRelease<'_>) -> Result<Release> {
        let current = self.get_by_tag(tag)?;
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let display_name = req
            .name
            .or(current.name.as_deref())
            .unwrap_or(tag);
        let body = req
            .notes
            .or(current.body.as_deref())
            .unwrap_or(display_name);
        let mut f: Vec<(&str, String)> = vec![
            ("tag_name", tag.to_string()),
            ("name", display_name.to_string()),
            ("body", body.to_string()),
        ];
        if req.prerelease == Some(true) {
            f.push(("prerelease", "true".to_string()));
        }
        let form = Client::str_refs(&f);
        self.client
            .patch(&format!("/repos/{o}/{r}/releases/{}", current.id), &form)
    }

    pub fn delete(&self, tag: &str) -> Result<()> {
        let release = self.get_by_tag(tag)?;
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        self.client
            .delete_ok(&format!("/repos/{o}/{r}/releases/{}", release.id))
    }

    // Follow-up (2026-07-18): asset-level DELETE
    // /repos/{owner}/{repo}/releases/{rid}/attach_files/{aid} exists in swagger
    // but is out of scope for this ticket.

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
