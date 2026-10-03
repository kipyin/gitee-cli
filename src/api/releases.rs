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
        let path = self.repo.api_path("releases");
        self.client.get_paged(&path, &[], limit)
    }

    /// Gitee quirk: a missing release returns HTTP 200 with a JSON `null`
    /// body (not 404). Deserialize as Option and map null to NotFound.
    pub fn get_by_tag(&self, tag: &str) -> Result<Release> {
        let rel: Option<Release> = self
            .client
            .get(&self.repo.api_path(format!("releases/tags/{tag}")), &[])?;
        rel.ok_or_else(|| crate::error::GiteeError::NotFound(format!("release {tag}")))
    }

    /// Gitee quirks: `body` is REQUIRED and must be non-empty (400 otherwise),
    /// so it defaults to the display name; `prerelease` is always sent as
    /// `"true"` or `"false"`; `name` defaults to `tag`.
    pub fn create(&self, req: &CreateRelease<'_>) -> Result<Release> {
        let display_name = req.name.unwrap_or(req.tag);
        let mut f: Vec<(&str, String)> = vec![
            ("tag_name", req.tag.to_string()),
            ("name", display_name.to_string()),
            ("body", req.notes.unwrap_or(display_name).to_string()),
        ];
        Client::push_some(&mut f, "target_commitish", req.target);
        Client::push_bool(&mut f, "prerelease", req.prerelease);
        let form = Client::str_refs(&f);
        self.client.post(&self.repo.api_path("releases"), &form)
    }

    /// Gitee quirk (swagger 2026-07-18): PATCH requires `tag_name`, `name`, and
    /// `body` on every request — GET-by-tag first, then send the flag value or
    /// the current value for all three. `prerelease` is sent only when requested.
    /// `--latest` omitted: PATCH /releases/{id} has no latest param (swagger 2026-07-18).
    pub fn edit(&self, tag: &str, req: &EditRelease<'_>) -> Result<Release> {
        let current = self.get_by_tag(tag)?;
        let display_name = req.name.or(current.name.as_deref()).unwrap_or(tag);
        let body = req
            .notes
            .or(current.body.as_deref())
            .unwrap_or(display_name);
        let mut f: Vec<(&str, String)> = vec![
            ("tag_name", tag.to_string()),
            ("name", display_name.to_string()),
            ("body", body.to_string()),
        ];
        Client::push_true_flag(&mut f, "prerelease", req.prerelease == Some(true));
        let form = Client::str_refs(&f);
        self.client.patch(
            &self.repo.api_path(format!("releases/{}", current.id)),
            &form,
        )
    }

    pub fn delete(&self, tag: &str) -> Result<()> {
        let release = self.get_by_tag(tag)?;
        self.client
            .delete_ok(&self.repo.api_path(format!("releases/{}", release.id)))
    }

    pub fn upload(&self, tag: &str, file_path: &str) -> Result<ReleaseAsset> {
        let release = self.get_by_tag(tag)?;
        let id = release.id;
        self.client.post_multipart(
            &self.repo.api_path(format!("releases/{id}/attach_files")),
            file_path,
        )
    }
}
