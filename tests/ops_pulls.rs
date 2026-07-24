use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::pulls::{CreatePr, EditPr, PrFilter};
use gitee_cli_rs::api::StateChange;
use gitee_cli_rs::models::{MergeMethod, PrState};
use gitee_cli_rs::repo::Repo;

const PULL_REQUEST_JSON: &str = include_str!("fixtures/pull_request.json");
const PR_FILE_DIFF_JSON: &str = include_str!("fixtures/pr_file_diff.json");

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

fn test_repo() -> Repo {
    Repo {
        owner: "oschina".to_string(),
        name: "gitee-cli".to_string(),
    }
}

#[test]
fn list_hits_pulls_path_with_state_and_author_single_page() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
            mockito::Matcher::UrlEncoded("author".into(), "dev1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!("[{PULL_REQUEST_JSON}]"))
        .create();

    let items = client(&server)
        .pulls(&test_repo())
        .list(&PrFilter {
            state: Some("open"),
            author: Some("dev1"),
            limit: 50,
            ..Default::default()
        })
        .expect("list should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].number, 12);
    assert_eq!(items[0].title, "Add pagination helpers");
}

#[test]
fn get_deserializes_pull_request() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    let pr = client(&server)
        .pulls(&test_repo())
        .get(12)
        .expect("get should succeed");

    mock.assert();
    assert_eq!(pr.number, 12);
    assert_eq!(pr.head.git_ref, "feature/paging");
    assert_eq!(pr.base.git_ref, "master");
}

#[test]
fn files_deserializes_file_diffs() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/files";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PR_FILE_DIFF_JSON)
        .create();

    let files = client(&server)
        .pulls(&test_repo())
        .files(12)
        .expect("files should succeed");

    mock.assert();
    assert_eq!(files.len(), 2);
    assert_eq!(files[0].path, "pom.xml");
    assert!(files[0].patch.as_ref().unwrap().contains("3.5.15"));
    assert!(files[1].patch.is_none());
}

#[test]
fn create_posts_form_title_head_base_body() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("title".into(), "New feature".into()),
            mockito::Matcher::UrlEncoded("head".into(), "feature/x".into()),
            mockito::Matcher::UrlEncoded("base".into(), "master".into()),
            mockito::Matcher::UrlEncoded("body".into(), "Please review".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    let pr = client(&server)
        .pulls(&test_repo())
        .create(&CreatePr {
            title: "New feature",
            head: "feature/x",
            base: "master",
            body: Some("Please review"),
            ..Default::default()
        })
        .expect("create should succeed");

    mock.assert();
    assert_eq!(pr.number, 12);
}

#[test]
fn create_sends_full_parity_fields() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("title".into(), "New feature".into()),
            mockito::Matcher::UrlEncoded("labels".into(), "bug,ui".into()),
            mockito::Matcher::UrlEncoded("assignees".into(), "me".into()),
            mockito::Matcher::UrlEncoded("testers".into(), "qa1".into()),
            mockito::Matcher::UrlEncoded("milestone_number".into(), "7".into()),
            mockito::Matcher::UrlEncoded("issue".into(), "I1AB2C".into()),
            mockito::Matcher::UrlEncoded("close_related_issue".into(), "true".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    client(&server)
        .pulls(&test_repo())
        .create(&CreatePr {
            title: "New feature",
            head: "feature/x",
            base: "master",
            labels: Some("bug,ui"),
            assignees: Some("me"),
            testers: Some("qa1"),
            milestone_number: Some(7),
            issue: Some("I1AB2C"),
            close_related_issue: true,
            ..Default::default()
        })
        .expect("create should succeed");

    mock.assert();
}

#[test]
fn file_contents_decodes_base64_file() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/contents/PULL_REQUEST_TEMPLATE.md";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::UrlEncoded("ref".into(), "master".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"encoding":"base64","content":"IyMgVGVtcGxhdGUK"}"#)
        .create();

    let text = client(&server)
        .repos()
        .file_contents("oschina", "gitee-cli", "PULL_REQUEST_TEMPLATE.md", "master")
        .expect("file_contents should succeed");

    mock.assert();
    assert_eq!(text.as_deref(), Some("## Template\n"));
}

#[test]
fn file_contents_404_is_none() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/contents/.gitee/PULL_REQUEST_TEMPLATE.md";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::UrlEncoded("ref".into(), "master".into()))
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"Not Found"}"#)
        .create();

    let text = client(&server)
        .repos()
        .file_contents(
            "oschina",
            "gitee-cli",
            ".gitee/PULL_REQUEST_TEMPLATE.md",
            "master",
        )
        .expect("404 should map to Ok(None)");

    mock.assert();
    assert_eq!(text, None);
}

#[test]
fn merge_puts_squash_and_close_related_issue_true() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/merge";

    let mock = server
        .mock("PUT", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("merge_method".into(), "squash".into()),
            mockito::Matcher::UrlEncoded("close_related_issue".into(), "true".into()),
        ]))
        .with_status(200)
        .create();

    client(&server)
        .pulls(&test_repo())
        .merge(12, MergeMethod::Squash, true)
        .expect("merge should succeed");

    mock.assert();
}

#[test]
fn merge_puts_merge_and_close_related_issue_false() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/merge";

    let mock = server
        .mock("PUT", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("merge_method".into(), "merge".into()),
            mockito::Matcher::UrlEncoded("close_related_issue".into(), "false".into()),
        ]))
        .with_status(200)
        .create();

    client(&server)
        .pulls(&test_repo())
        .merge(12, MergeMethod::Merge, false)
        .expect("merge should succeed");

    mock.assert();
}

#[test]
fn set_state_patches_form_state_closed() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12";

    let mock = server
        .mock("PATCH", api_path(path).as_str())
        .match_body(mockito::Matcher::UrlEncoded(
            "state".into(),
            "closed".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    let pr = client(&server)
        .pulls(&test_repo())
        .set_state(12, PrState::Closed)
        .expect("set_state should succeed");

    mock.assert();
    assert_eq!(pr.number, 12);
}

/// Idempotent close: GET returns an open PR → PATCH fires → Changed.
#[test]
fn set_state_idempotent_open_pr_patches_and_returns_changed() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/pulls/12";
    let patch_path = "/repos/oschina/gitee-cli/pulls/12";

    server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();
    let patch = server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::UrlEncoded("state".into(), "closed".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    let change = client(&server)
        .pulls(&test_repo())
        .set_state_idempotent(12, PrState::Closed)
        .expect("idempotent close should succeed");

    patch.assert();
    assert!(matches!(change, StateChange::Changed(_)));
}

/// Idempotent close: GET returns an already-closed PR → no PATCH → Already.
#[test]
fn set_state_idempotent_already_closed_skips_patch() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/pulls/12";
    let closed_body = r#"{"number":12,"title":"x","state":"closed","html_url":"https://gitee.com/x","head":{"ref":"f"},"base":{"ref":"master"}}"#;

    let get = server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(closed_body)
        .create();
    let patch = server
        .mock("PATCH", api_path(get_path).as_str())
        .expect(0)
        .create();

    let change = client(&server)
        .pulls(&test_repo())
        .set_state_idempotent(12, PrState::Closed)
        .expect("idempotent close should succeed");

    get.assert();
    patch.assert();
    assert!(matches!(change, StateChange::Already(_)));
}

/// Idempotent merge: GET returns a merged PR → no PUT → Already.
#[test]
fn merge_idempotent_already_merged_skips_put() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/pulls/12";
    let merged_body = r#"{"number":12,"title":"x","state":"merged","html_url":"https://gitee.com/x","head":{"ref":"f"},"base":{"ref":"master"}}"#;

    let get = server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(merged_body)
        .create();
    let put = server
        .mock("PUT", api_path("/repos/oschina/gitee-cli/pulls/12/merge").as_str())
        .expect(0)
        .create();

    let change = client(&server)
        .pulls(&test_repo())
        .merge_idempotent(12, MergeMethod::Merge, true)
        .expect("idempotent merge should succeed");

    get.assert();
    put.assert();
    assert!(matches!(change, StateChange::Already(())));
}

/// Idempotent merge: GET returns an open PR → PUT fires → Changed.
#[test]
fn merge_idempotent_open_pr_puts_and_returns_changed() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/pulls/12";

    server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();
    let put = server
        .mock("PUT", api_path("/repos/oschina/gitee-cli/pulls/12/merge").as_str())
        .with_status(200)
        .create();

    let change = client(&server)
        .pulls(&test_repo())
        .merge_idempotent(12, MergeMethod::Squash, false)
        .expect("idempotent merge should succeed");

    put.assert();
    assert!(matches!(change, StateChange::Changed(())));
}

#[test]
fn comment_posts_form_body() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/comments";
    let response = r#"{"id":99,"body":"LGTM","html_url":"https://gitee.com/oschina/gitee-cli/pulls/12#note_99"}"#;

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::UrlEncoded("body".into(), "LGTM".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response)
        .create();

    let comment = client(&server)
        .pulls(&test_repo())
        .comment(12, "LGTM")
        .expect("comment should succeed");

    mock.assert();
    assert_eq!(comment.body, "LGTM");
}

#[test]
fn list_comments_hits_pull_comments_path_and_decodes_pr_comment() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/comments";
    // position/new_line arrive as strings in Gitee swagger; keep them Option<String>.
    let body = r#"[
        {
            "id": 99,
            "body": "general note",
            "user": {"login": "dev1"},
            "created_at": "2026-01-01T00:00:00+08:00",
            "updated_at": "2026-01-01T00:00:00+08:00",
            "comment_type": "pr_comment"
        },
        {
            "id": 100,
            "body": "line note",
            "user": {"login": "dev2"},
            "created_at": "2026-01-02T00:00:00+08:00",
            "updated_at": "2026-01-02T00:00:00+08:00",
            "path": "src/main.rs",
            "position": "42",
            "new_line": "10",
            "commit_id": "abc123",
            "comment_type": "diff_comment"
        }
    ]"#;

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    let items = client(&server)
        .pulls(&test_repo())
        .list_comments(
            12,
            &gitee_cli_rs::api::pulls::PrCommentFilter {
                limit: 30,
                ..Default::default()
            },
        )
        .expect("list_comments should succeed");

    mock.assert();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, 99);
    assert_eq!(items[0].body, "general note");
    assert_eq!(items[0].comment_type.as_deref(), Some("pr_comment"));
    assert!(items[0].path.is_none());
    assert_eq!(items[1].id, 100);
    assert_eq!(items[1].path.as_deref(), Some("src/main.rs"));
    assert_eq!(items[1].position.as_deref(), Some("42"));
    assert_eq!(items[1].new_line.as_deref(), Some("10"));
    assert_eq!(items[1].commit_id.as_deref(), Some("abc123"));
    assert_eq!(items[1].comment_type.as_deref(), Some("diff_comment"));
}

#[test]
fn list_comments_sends_comment_type_filter() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/comments";
    let body = r#"[{"id":100,"body":"line","comment_type":"diff_comment"}]"#;

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
            mockito::Matcher::UrlEncoded("comment_type".into(), "diff_comment".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    let items = client(&server)
        .pulls(&test_repo())
        .list_comments(
            12,
            &gitee_cli_rs::api::pulls::PrCommentFilter {
                kind: Some(gitee_cli_rs::models::PrCommentKind::Diff),
                limit: 30,
            },
        )
        .expect("list_comments with type should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, 100);
}

#[test]
fn latest_comment_picks_authors_most_recent_pr_comment() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/comments";
    let body = r#"[
        {
            "id": 10,
            "body": "older",
            "user": {"login": "me"},
            "created_at": "2026-01-01T00:00:00+08:00",
            "comment_type": "pr_comment"
        },
        {
            "id": 11,
            "body": "newer",
            "user": {"login": "me"},
            "created_at": "2026-01-02T00:00:00+08:00",
            "path": "a.rs",
            "position": "3",
            "comment_type": "diff_comment"
        }
    ]"#;

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    let comment = client(&server)
        .pulls(&test_repo())
        .latest_comment(12, "me")
        .expect("latest_comment should succeed");

    mock.assert();
    assert_eq!(comment.id, 11);
    assert_eq!(comment.body, "newer");
    assert_eq!(comment.path.as_deref(), Some("a.rs"));
}

#[test]
fn latest_comment_errors_when_author_has_none_on_pr() {
    use gitee_cli_rs::error::GiteeError;

    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/comments";
    let body = r#"[{"id":1,"body":"x","user":{"login":"other"},"created_at":"2026-01-01T00:00:00+08:00"}]"#;

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    let err = client(&server)
        .pulls(&test_repo())
        .latest_comment(12, "me")
        .expect_err("no comment by me should error");

    mock.assert();
    assert!(
        matches!(err, GiteeError::Usage(_)),
        "expected Usage error, got {err:?}"
    );
}

#[test]
fn approve_force_true_sends_force_field() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/review";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::UrlEncoded("force".into(), "true".into()))
        .with_status(200)
        .create();

    client(&server)
        .pulls(&test_repo())
        .approve(12, true)
        .expect("approve should succeed");

    mock.assert();
}

#[test]
fn approve_force_false_sends_empty_form() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/review";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::Exact(String::new()))
        .with_status(200)
        .create();

    client(&server)
        .pulls(&test_repo())
        .approve(12, false)
        .expect("approve should succeed");

    mock.assert();
}

#[test]
fn test_force_true_sends_force_field() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/test";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::UrlEncoded("force".into(), "true".into()))
        .with_status(200)
        .create();

    client(&server)
        .pulls(&test_repo())
        .test(12, true)
        .expect("test should succeed");

    mock.assert();
}

#[test]
fn test_force_false_sends_empty_form() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12/test";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::Exact(String::new()))
        .with_status(200)
        .create();

    client(&server)
        .pulls(&test_repo())
        .test(12, false)
        .expect("test should succeed");

    mock.assert();
}

#[test]
fn link_get_then_patch_appends_linked_tag() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12";

    let get = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":12,"title":"Link me","body":"Existing body","html_url":"https://gitee.com/x"}"#,
        )
        .create();

    let patch = server
        .mock("PATCH", api_path(path).as_str())
        .match_body(mockito::Matcher::Regex(r"Linked%3A\+%2342".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    let linked = client(&server)
        .pulls(&test_repo())
        .link(12, "#42")
        .expect("link should succeed");

    get.assert();
    patch.assert();
    assert!(linked);
}

#[test]
fn link_already_linked_skips_patch() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12";

    let get = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":12,"title":"Done","body":"Already Linked: #42","html_url":"https://gitee.com/x"}"#,
        )
        .create();

    let patch = server
        .mock("PATCH", api_path(path).as_str())
        .expect(0)
        .create();

    let linked = client(&server)
        .pulls(&test_repo())
        .link(12, "#42")
        .expect("link should short-circuit");

    get.assert();
    patch.assert();
    assert!(!linked);
}

#[test]
fn edit_sends_only_provided_fields_comma_joined() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12";

    let mock = server
        .mock("PATCH", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("title".into(), "New title".into()),
            mockito::Matcher::UrlEncoded("labels".into(), "bug,regression,ui".into()),
            mockito::Matcher::UrlEncoded("assignees".into(), "dev1,dev2".into()),
            mockito::Matcher::UrlEncoded("testers".into(), "qa1".into()),
            mockito::Matcher::UrlEncoded("milestone_number".into(), "7".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    let pr = client(&server)
        .pulls(&test_repo())
        .edit(
            12,
            &EditPr {
                title: Some("New title"),
                labels: Some("bug,regression,ui"),
                assignees: Some("dev1,dev2"),
                testers: Some("qa1"),
                milestone_number: Some(7),
                ..Default::default()
            },
        )
        .expect("edit should succeed");

    mock.assert();
    assert_eq!(pr.number, 12);
}

#[test]
fn edit_omits_unset_fields_from_form_body() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls/12";

    // Only `body` provided: the form must contain exactly that one field.
    let mock = server
        .mock("PATCH", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("body".into(), "hello world".into()),
            mockito::Matcher::Regex("^[^&=]+=[^&]*$".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(PULL_REQUEST_JSON)
        .create();

    client(&server)
        .pulls(&test_repo())
        .edit(
            12,
            &EditPr {
                body: Some("hello world"),
                ..Default::default()
            },
        )
        .expect("edit should succeed");

    mock.assert();
}

#[test]
fn list_milestones_hits_milestones_path() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/milestones";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[{"number":7,"title":"v1.0"},{"number":9,"title":"v2.0"}]"#)
        .create();

    let milestones = client(&server)
        .repos()
        .list_milestones("oschina", "gitee-cli")
        .expect("list milestones should succeed");

    mock.assert();
    assert_eq!(milestones.len(), 2);
    assert_eq!(milestones[0].number, 7);
    assert_eq!(milestones[1].title, "v2.0");
}

#[test]
fn list_sends_assignee_and_tester_filters() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/pulls";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
            mockito::Matcher::UrlEncoded("assignee".into(), "dev1".into()),
            mockito::Matcher::UrlEncoded("tester".into(), "qa1".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!("[{PULL_REQUEST_JSON}]"))
        .create();

    let items = client(&server)
        .pulls(&test_repo())
        .list(&PrFilter {
            state: Some("open"),
            assignee: Some("dev1"),
            tester: Some("qa1"),
            limit: 50,
            ..Default::default()
        })
        .expect("list should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].number, 12);
}
