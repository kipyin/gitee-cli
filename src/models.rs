use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

/// CLI `--type` for `pr comment list`. Ops maps to Gitee `comment_type`
/// (`diff_comment` | `pr_comment`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrCommentKind {
    Diff,
    General,
}

impl PrCommentKind {
    pub fn from_cli(s: &str) -> Option<Self> {
        match s {
            "diff" => Some(Self::Diff),
            "general" => Some(Self::General),
            _ => None,
        }
    }

    /// Gitee `comment_type` query value.
    pub fn as_api_str(self) -> &'static str {
        match self {
            Self::Diff => "diff_comment",
            Self::General => "pr_comment",
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

/// Organization from GET /user/orgs (swagger `Group`).
/// List payload has `login` and `description` only — not `name` or membership `role`.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Org {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub login: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct SshKey {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct CollaboratorPermissions {
    #[serde(default)]
    pub pull: Option<bool>,
    #[serde(default)]
    pub push: Option<bool>,
    #[serde(default)]
    pub admin: Option<bool>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Collaborator {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub login: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub permissions: Option<CollaboratorPermissions>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Webhook {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default, skip_serializing)]
    pub password: Option<String>,
    #[serde(default)]
    pub result_code: Option<i64>,
    #[serde(default)]
    pub result_msg: Option<String>,
    #[serde(default)]
    pub push_events: Option<bool>,
    #[serde(default)]
    pub tag_push_events: Option<bool>,
    #[serde(default)]
    pub issues_events: Option<bool>,
    #[serde(default)]
    pub merge_requests_events: Option<bool>,
    #[serde(default)]
    pub note_events: Option<bool>,
}


#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Gist {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub public: Option<bool>,
    #[serde(default)]
    pub files: Option<BTreeMap<String, GistFile>>,
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub owner: Option<UserBasic>,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct GistFile {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub raw_url: Option<String>,
    #[serde(default)]
    pub size: Option<i64>,
    #[serde(default)]
    pub truncated: Option<bool>,
}

/// Gitee milestone. `number` is the id the v5 mutation endpoints take
/// (`milestone_number`); `title` is what users type, so CLI flags accept
/// either and resolve via `Milestone::resolve`.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct Milestone {
    #[serde(default)]
    pub number: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub html_url: Option<String>,
    #[serde(default)]
    pub due_on: Option<String>,
    #[serde(default)]
    pub open_issues: Option<i64>,
    #[serde(default)]
    pub closed_issues: Option<i64>,
}

impl Milestone {
    /// Resolve an `--milestone` flag value: a bare integer is used as-is,
    /// otherwise match by exact title. `None` means "no such milestone".
    pub fn resolve(list: &[Milestone], id_or_title: &str) -> Option<i64> {
        if let Ok(n) = id_or_title.trim().parse::<i64>() {
            return Some(n);
        }
        list.iter()
            .find(|m| m.title == id_or_title)
            .map(|m| m.number)
    }
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
///
/// `path` is the canonical JSON field name (matches `gh pr view --json files`).
/// Gitee's v5 API returns `filename`, so deserialization also accepts that
/// alias for forward-compatibility with the live response shape.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct FileDiff {
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default, alias = "filename")]
    pub path: String,
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
    /// Total changed lines (additions + deletions). Gitee doesn't always
    /// return it, hence `Option`; populated when present.
    #[serde(default)]
    pub changes: Option<String>,
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
    pub assignees: Option<Vec<UserBasic>>,
    #[serde(default)]
    pub testers: Option<Vec<UserBasic>>,
    #[serde(default)]
    pub milestone: Option<Milestone>,
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
    /// Per-file diff list. Not returned by `GET /repos/.../pulls/{n}` —
    /// populated by `pr view` via the separate `/pulls/{n}/files` endpoint.
    #[serde(default)]
    pub files: Option<Vec<FileDiff>>,
}

/// Minimal repo reference embedded in user-level issues (`GET /user/issues`).
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct IssueRepoRef {
    #[serde(default)]
    pub full_name: Option<String>,
    #[serde(default)]
    pub html_url: Option<String>,
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
    pub security_hole: Option<bool>,
    #[serde(default)]
    pub milestone: Option<Milestone>,
    #[serde(default)]
    pub comments: Option<i64>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    /// Present on user-level lists (`GET /user/issues`); absent on repo lists.
    #[serde(default)]
    pub repository: Option<IssueRepoRef>,
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

/// PR comment (`PullRequestComments` in Gitee v5 swagger). Richer than issue
/// `Comment`/`Note`: carries optional positional fields and `comment_type`
/// (`diff_comment` | `pr_comment`). `position` / `new_line` are strings in the
/// swagger response even though create takes an int position.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct PrComment {
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
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub position: Option<String>,
    #[serde(default)]
    pub new_line: Option<String>,
    #[serde(default)]
    pub commit_id: Option<String>,
    /// `diff_comment` or `pr_comment`.
    #[serde(default)]
    pub comment_type: Option<String>,
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
    /// Set by `repo view` via GET /user/starred/... (not from repo payload).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starred: Option<bool>,
    /// Set by `repo view` via GET /user/subscriptions/... (not from repo payload).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watching: Option<bool>,
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
mod webhook_tests {
    use super::Webhook;

    #[test]
    fn password_omitted_from_json_output() {
        let hook = Webhook {
            id: 1,
            password: Some("s3cret".into()),
            url: Some("https://example.com/hook".into()),
            ..Default::default()
        };
        let value = serde_json::to_value(&hook).expect("serialize");
        assert!(value.get("password").is_none());
    }
}

#[cfg(test)]
mod state_tests {
    use super::{
        Issue, IssueState, MergeMethod, Milestone, PrComment, PrCommentKind, PrState, PullRequest,
    };
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
    fn milestone_resolve_by_id_or_title() {
        let list = vec![
            Milestone {
                number: 7,
                title: "v1.0".into(),
                ..Default::default()
            },
            Milestone {
                number: 9,
                title: "v2.0".into(),
                ..Default::default()
            },
        ];
        assert_eq!(Milestone::resolve(&list, "42"), Some(42));
        assert_eq!(Milestone::resolve(&list, "v2.0"), Some(9));
        assert_eq!(Milestone::resolve(&list, "nope"), None);
    }

    #[test]
    fn pr_deserializes_assignees_testers_milestone() {
        let pr: PullRequest = serde_json::from_str(
            r#"{
                "number": 3,
                "assignees": [{"login": "dev1"}],
                "testers": [{"login": "qa1"}],
                "milestone": {"number": 7, "title": "v1.0"}
            }"#,
        )
        .expect("pr json");
        assert_eq!(pr.assignees.expect("assignees")[0].login, "dev1");
        assert_eq!(pr.testers.expect("testers")[0].login, "qa1");
        let ms = pr.milestone.expect("milestone");
        assert_eq!(ms.number, 7);
        assert_eq!(ms.title, "v1.0");
    }

    #[test]
    fn pr_comment_kind_maps_cli_to_api_comment_type() {
        assert_eq!(PrCommentKind::from_cli("diff"), Some(PrCommentKind::Diff));
        assert_eq!(
            PrCommentKind::from_cli("general"),
            Some(PrCommentKind::General)
        );
        assert_eq!(PrCommentKind::from_cli("other"), None);
        assert_eq!(PrCommentKind::Diff.as_api_str(), "diff_comment");
        assert_eq!(PrCommentKind::General.as_api_str(), "pr_comment");
    }

    #[test]
    fn pr_comment_deserializes_string_position_and_type() {
        let c: PrComment = serde_json::from_str(
            r#"{
                "id": 100,
                "body": "line note",
                "path": "src/main.rs",
                "position": "42",
                "new_line": "10",
                "commit_id": "abc123",
                "comment_type": "diff_comment",
                "user": {"login": "dev2"},
                "created_at": "2026-01-02T00:00:00+08:00"
            }"#,
        )
        .expect("pr comment json");
        assert_eq!(c.id, 100);
        assert_eq!(c.position.as_deref(), Some("42"));
        assert_eq!(c.new_line.as_deref(), Some("10"));
        assert_eq!(c.comment_type.as_deref(), Some("diff_comment"));
        assert_eq!(c.path.as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn issue_deserializes_security_hole_and_milestone() {
        let issue: Issue = serde_json::from_str(
            r#"{
                "number": "I1AB",
                "title": "Leak",
                "security_hole": true,
                "milestone": {"number": 3, "title": "v1.0"}
            }"#,
        )
        .expect("issue json");
        assert_eq!(issue.security_hole, Some(true));
        assert_eq!(issue.milestone.expect("milestone").number, 3);
    }

    #[test]
    fn merge_method_as_str_values() {
        assert_eq!(MergeMethod::Merge.as_str(), "merge");
        assert_eq!(MergeMethod::Squash.as_str(), "squash");
        assert_eq!(MergeMethod::Rebase.as_str(), "rebase");
    }
}
