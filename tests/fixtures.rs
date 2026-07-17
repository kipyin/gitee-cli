use gitee::models::{FileDiff, Issue, PullRequest, RepoDetails};

const PULL_REQUEST_JSON: &str = include_str!("fixtures/pull_request.json");
const PR_FILE_DIFF_JSON: &str = include_str!("fixtures/pr_file_diff.json");
const ISSUE_JSON: &str = include_str!("fixtures/issue.json");
const REPO_LIST_JSON: &str = include_str!("fixtures/repo_list.json");

#[test]
fn fixture_pull_request_deserializes() {
    let pr: PullRequest = serde_json::from_str(PULL_REQUEST_JSON).expect("pull request json");
    assert_eq!(pr.number, 12);
    assert_eq!(pr.title, "Add pagination helpers");
    assert_eq!(pr.state, "open");
    assert_eq!(pr.head.git_ref, "feature/paging");
    assert_eq!(pr.base.git_ref, "master");
    assert_eq!(pr.user.as_ref().expect("user").login, "dev1");
    assert_eq!(pr.labels.as_ref().expect("labels").len(), 1);
}

#[test]
fn fixture_pr_file_diff_deserializes() {
    let files: Vec<FileDiff> = serde_json::from_str(PR_FILE_DIFF_JSON).expect("file diff json");
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].filename, "pom.xml");
    assert_eq!(files[0].additions.as_deref(), Some("5"));
    assert_eq!(files[0].deletions.as_deref(), Some("6"));
    assert!(files[0].patch.as_ref().expect("patch").contains("3.5.15"));
    assert_eq!(files[1].filename, "logo.png");
    assert!(files[1].patch.is_none());
}

#[test]
fn fixture_issue_deserializes() {
    let issue: Issue = serde_json::from_str(ISSUE_JSON).expect("issue json");
    assert_eq!(issue.number, "88");
    assert_eq!(issue.title, "Login fails with expired token");
    assert_eq!(issue.state, "open");
    assert_eq!(issue.user.as_ref().expect("user").login, "reporter");
    assert_eq!(issue.assignee.as_ref().expect("assignee").login, "dev1");
    assert_eq!(issue.labels.as_ref().expect("labels").len(), 2);
    assert_eq!(issue.comments, Some(7));
}

#[test]
fn fixture_repo_list_deserializes() {
    let repos: Vec<RepoDetails> = serde_json::from_str(REPO_LIST_JSON).expect("repo list json");
    assert_eq!(repos.len(), 2);
    assert_eq!(repos[0].full_name, "oschina/gitee-cli");
    assert_eq!(repos[0].stargazers_count, Some(128));
    assert_eq!(repos[1].full_name, "oschina/docs");
    assert_eq!(repos[1].private, Some(true));
    assert!(repos[1].parent.is_some());
}
