use serde::{Deserialize, Serialize};

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
}

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct PullRequest {
    #[serde(default)]
    pub number: i64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub state: String,
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
    pub state: String,
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

#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct RepoInfo {
    #[serde(default)]
    pub default_branch: Option<String>,
}
