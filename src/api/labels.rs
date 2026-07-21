use super::client::Client;
use crate::api::StateChange;
use crate::error::{GiteeError, Result};
use crate::models::Label;
use crate::repo::Repo;

pub struct Labels<'a> {
    client: &'a Client,
    repo: &'a Repo,
}

pub struct CreateLabel<'a> {
    pub name: &'a str,
    pub color: &'a str,
}

pub struct EditLabel<'a> {
    pub name: Option<&'a str>,
    pub color: Option<&'a str>,
}

/// Strip one leading '#', require exactly 6 hex chars, lowercase.
pub fn normalize_color(s: &str) -> Result<String> {
    let trimmed = s.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(GiteeError::Usage(format!(
            "invalid color '{s}': must be exactly 6 hex digits (with optional leading #)"
        )));
    }
    Ok(hex.to_ascii_lowercase())
}

/// Compare two colors, treating missing colors as matching the requested
/// one (Gitee sometimes omits `color` on existing labels).
fn colors_match(existing: Option<&str>, requested: &str) -> bool {
    match existing {
        Some(c) => c.eq_ignore_ascii_case(requested),
        None => true,
    }
}

impl Labels<'_> {
    pub(crate) fn new<'a>(client: &'a Client, repo: &'a Repo) -> Labels<'a> {
        Labels { client, repo }
    }

    /// GET /repos/{owner}/{repo}/labels returns a plain array. The swagger
    /// documents no `page`/`per_page` params, so we fetch the full list with
    /// `get` and truncate to `limit` client-side.
    pub fn list(&self, limit: usize) -> Result<Vec<Label>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let path = format!("/repos/{o}/{r}/labels");
        let mut items: Vec<Label> = self.client.get(&path, &[])?;
        if items.len() > limit {
            items.truncate(limit);
        }
        Ok(items)
    }

    pub fn create(&self, req: &CreateLabel<'_>) -> Result<Label> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let color = normalize_color(req.color)?;
        let form = [("name", req.name), ("color", color.as_str())];
        self.client
            .post(&format!("/repos/{o}/{r}/labels"), &form)
    }

    /// Idempotent create: list existing labels first. If a label with the
    /// same name already exists with the requested color, return
    /// `StateChange::Already(label)` and exit 0. If it exists with a
    /// different color, return a Usage error suggesting `label edit`. If it
    /// doesn't exist, create it and return `StateChange::Changed(label)`.
    pub fn create_idempotent(&self, req: &CreateLabel<'_>) -> Result<StateChange<Label>> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let requested = normalize_color(req.color)?;
        let existing: Vec<Label> = self.client.get(&format!("/repos/{o}/{r}/labels"), &[])?;
        if let Some(found) = existing.iter().find(|l| l.name == req.name) {
            if colors_match(found.color.as_deref(), &requested) {
                return Ok(StateChange::Already(found.clone()));
            }
            return Err(GiteeError::Usage(format!(
                "label '{}' already exists with color '{}'. Use `gitee label edit {} --color {}` to change it.",
                req.name,
                found.color.as_deref().unwrap_or("(none)"),
                req.name,
                requested,
            )));
        }
        let form = [("name", req.name), ("color", requested.as_str())];
        let label: Label = self
            .client
            .post(&format!("/repos/{o}/{r}/labels"), &form)?;
        Ok(StateChange::Changed(label))
    }

    pub fn edit(&self, original_name: &str, req: &EditLabel<'_>) -> Result<Label> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        let mut f: Vec<(&str, String)> = Vec::new();
        if let Some(name) = req.name {
            f.push(("name", name.to_string()));
        }
        if let Some(color) = req.color {
            f.push(("color", normalize_color(color)?));
        }
        let form = Client::str_refs(&f);
        self.client
            .patch(&format!("/repos/{o}/{r}/labels/{original_name}"), &form)
    }

    pub fn delete(&self, name: &str) -> Result<()> {
        let o = self.repo.owner.as_str();
        let r = self.repo.name.as_str();
        self.client
            .delete_ok(&format!("/repos/{o}/{r}/labels/{name}"))
    }
}

#[cfg(test)]
mod color_tests {
    use super::normalize_color;

    #[test]
    fn accepts_six_hex_with_or_without_hash() {
        assert_eq!(normalize_color("ff0000").unwrap(), "ff0000");
        assert_eq!(normalize_color("#FF0000").unwrap(), "ff0000");
        assert_eq!(normalize_color("#aabbcc").unwrap(), "aabbcc");
    }

    #[test]
    fn rejects_bad_length_and_non_hex() {
        assert!(normalize_color("fff").is_err());
        assert!(normalize_color("ff000").is_err());
        assert!(normalize_color("ff00000").is_err());
        assert!(normalize_color("gggggg").is_err());
        assert!(normalize_color("#12g456").is_err());
    }
}
