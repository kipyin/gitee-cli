use serde::{Deserialize, Serialize};
use std::fmt;

macro_rules! state_enum {
    ($name:ident { $($variant:ident => $s:literal),+ $(,)? }) => {
        /// Gitee lifecycle state. Unknown values deserialize to `Unknown`
        /// (forward-compatible); serialization always emits the API string,
        /// so `--json` output is unchanged for known states.
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        pub enum $name {
            #[default]
            $($variant),+,
            Unknown,
        }

        impl $name {
            pub fn as_str(&self) -> &'static str {
                match self {
                    $($name::$variant => $s),+,
                    $name::Unknown => "unknown",
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                let raw = String::deserialize(d)?;
                Ok(match raw.to_lowercase().as_str() {
                    $($s => $name::$variant),+,
                    _ => $name::Unknown,
                })
            }
        }
    };
}

state_enum!(PrState { Open => "open", Closed => "closed", Merged => "merged" });
state_enum!(IssueState {
    Open => "open",
    Progressing => "progressing",
    Closed => "closed",
    Rejected => "rejected",
});

/// Gitee merge methods for PR merge (form field `merge_method`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MergeMethod {
    #[default]
    Merge,
    Squash,
    Rebase,
}

impl MergeMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct UserBasic {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub login: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub html_url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Label {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub color: Option<String>,
}

/// Minimal repo reference embedded in PR `head`/`base` (includes clone URLs on live API).
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct PrRepoRef {
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub ssh_url: Option<String>,
    #[serde(default)]
    pub clone_url: Option<String>,
}

impl PrRepoRef {
    /// Best URL for fetching this repo's refs: SSH, then HTTPS clone, then web URL.
    pub fn fetch_url(&self) -> Option<String> {
        [&self.ssh_url, &self.clone_url, &self.html_url]
            .into_iter()
            .flatten()
            .find(|u| !u.is_empty())
            .cloned()
    }
}

/// Gitee returns PR `head`/`base` as objects `{ ref, label, sha, repo, user }`,
/// not plain strings (the swagger model is wrong on this point).
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct PrBranch {
    #[serde(default, rename = "ref")]
    pub git_ref: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub repo: Option<PrRepoRef>,
}

/// Gitee nests the unified diff under `patch.diff` (not a plain string).
#[derive(Deserialize, Clone, Debug, Default)]
struct FilePatchBody {
    #[serde(default)]
    diff: Option<String>,
}

fn deserialize_patch<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value: Option<FilePatchBody> = Option::deserialize(deserializer)?;
    Ok(value.and_then(|p| p.diff.filter(|d| !d.is_empty())))
}

fn serialize_patch<S>(patch: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    patch.serialize(serializer)
}

/// One changed file from GET /repos/{owner}/{repo}/pulls/{number}/files.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct FileDiff {
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(
        default,
        deserialize_with = "deserialize_patch",
        serialize_with = "serialize_patch"
    )]
    pub patch: Option<String>,
    #[serde(default)]
    pub additions: Option<String>,
    #[serde(default)]
    pub deletions: Option<String>,
    #[serde(default)]
    pub raw_url: Option<String>,
    #[serde(default)]
    pub blob_url: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct PullRequest {
    #[serde(default)]
    pub number: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: PrState,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub head: PrBranch,
    #[serde(default)]
    pub base: PrBranch,
    #[serde(default)]
    pub user: Option<UserBasic>,
    #[serde(default)]
    pub draft: Option<bool>,
    #[serde(default)]
    pub labels: Option<Vec<Label>>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub merged_at: Option<String>,
    #[serde(default)]
    pub closed_at: Option<String>,
    #[serde(default)]
    pub mergeable: Option<bool>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Issue {
    #[serde(default)]
    pub number: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: IssueState,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub user: Option<UserBasic>,
    #[serde(default)]
    pub assignee: Option<UserBasic>,
    #[serde(default)]
    pub labels: Option<Vec<Label>>,
    #[serde(default)]
    pub comments: Option<i64>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Comment {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub user: Option<UserBasic>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct RepoDetails {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub full_name: String,
    #[serde(default)]
    pub human_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub html_url: String,
    #[serde(default)]
    pub ssh_url: Option<String>,
    #[serde(default)]
    pub clone_url: Option<String>,
    #[serde(default)]
    pub default_branch: Option<String>,
    #[serde(default)]
    pub private: Option<bool>,
    #[serde(default)]
    pub stargazers_count: Option<i64>,
    #[serde(default)]
    pub fork_count: Option<i64>,
    #[serde(default)]
    pub open_issues_count: Option<i64>,
    #[serde(default)]
    pub owner: Option<UserBasic>,
    #[serde(default)]
    pub parent: Option<Box<RepoDetails>>,
}

impl RepoDetails {
    /// Clone URL policy: SSH when `ssh` is set, HTTPS otherwise; falls back to
    /// the web URL when the preferred URL is missing.
    pub fn preferred_url(&self, ssh: bool) -> String {
        let pick = if ssh { &self.ssh_url } else { &self.clone_url };
        if let Some(u) = pick {
            if !u.is_empty() {
                return u.clone();
            }
        }
        self.html_url.clone()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct RepoInfo {
    #[serde(default)]
    pub default_branch: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct ReleaseAsset {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub browser_download_url: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Release {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub tag_name: String,
    #[serde(default)]
    pub target_commitish: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub prerelease: Option<bool>,
    #[serde(default)]
    pub draft: Option<bool>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub author: Option<UserBasic>,
    #[serde(default)]
    pub assets: Option<Vec<ReleaseAsset>>,
}

#[cfg(test)]
mod state_tests {
    use super::{IssueState, MergeMethod, PrState};
    use serde_json;

    #[test]
    fn pr_state_deserializes_known_values_case_insensitive() {
        assert_eq!(
            serde_json::from_str::<PrState>(r#""open""#).unwrap(),
            PrState::Open
        );
        assert_eq!(
            serde_json::from_str::<PrState>(r#""CLOSED""#).unwrap(),
            PrState::Closed
        );
        assert_eq!(
            serde_json::from_str::<PrState>(r#""Merged""#).unwrap(),
            PrState::Merged
        );
    }

    #[test]
    fn pr_state_unknown_string_deserializes_to_unknown() {
        assert_eq!(
            serde_json::from_str::<PrState>(r#""draft""#).unwrap(),
            PrState::Unknown
        );
    }

    #[test]
    fn pr_state_serializes_canonical_lowercase() {
        assert_eq!(serde_json::to_string(&PrState::Open).unwrap(), r#""open""#);
        assert_eq!(
            serde_json::to_string(&PrState::Closed).unwrap(),
            r#""closed""#
        );
        assert_eq!(
            serde_json::to_string(&PrState::Merged).unwrap(),
            r#""merged""#
        );
    }

    #[test]
    fn issue_state_deserializes_known_values_case_insensitive() {
        assert_eq!(
            serde_json::from_str::<IssueState>(r#""open""#).unwrap(),
            IssueState::Open
        );
        assert_eq!(
            serde_json::from_str::<IssueState>(r#""PROGRESSING""#).unwrap(),
            IssueState::Progressing
        );
        assert_eq!(
            serde_json::from_str::<IssueState>(r#""Closed""#).unwrap(),
            IssueState::Closed
        );
        assert_eq!(
            serde_json::from_str::<IssueState>(r#""rejected""#).unwrap(),
            IssueState::Rejected
        );
    }

    #[test]
    fn issue_state_unknown_string_deserializes_to_unknown() {
        assert_eq!(
            serde_json::from_str::<IssueState>(r#""archived""#).unwrap(),
            IssueState::Unknown
        );
    }

    #[test]
    fn issue_state_serializes_canonical_lowercase() {
        assert_eq!(
            serde_json::to_string(&IssueState::Open).unwrap(),
            r#""open""#
        );
        assert_eq!(
            serde_json::to_string(&IssueState::Progressing).unwrap(),
            r#""progressing""#
        );
        assert_eq!(
            serde_json::to_string(&IssueState::Closed).unwrap(),
            r#""closed""#
        );
        assert_eq!(
            serde_json::to_string(&IssueState::Rejected).unwrap(),
            r#""rejected""#
        );
    }

    #[test]
    fn merge_method_as_str_values() {
        assert_eq!(MergeMethod::Merge.as_str(), "merge");
        assert_eq!(MergeMethod::Squash.as_str(), "squash");
        assert_eq!(MergeMethod::Rebase.as_str(), "rebase");
    }
}
