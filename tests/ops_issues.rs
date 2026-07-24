use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::issues::{CreateIssue, EditIssue};
use gitee_cli_rs::api::StateChange;
use gitee_cli_rs::error::GiteeError;
use gitee_cli_rs::models::IssueState;
use gitee_cli_rs::repo::Repo;

const ISSUE_JSON: &str = include_str!("fixtures/issue.json");

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
fn create_posts_owner_issues_path_with_repo_form_field() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/issues";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("repo".into(), "gitee-cli".into()),
            mockito::Matcher::UrlEncoded("title".into(), "New bug".into()),
            mockito::Matcher::UrlEncoded("body".into(), "Steps to reproduce".into()),
            mockito::Matcher::UrlEncoded("assignee".into(), "dev1".into()),
            mockito::Matcher::UrlEncoded("labels".into(), "bug,auth".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    let issue = client(&server)
        .issues(&test_repo())
        .create(&CreateIssue {
            title: "New bug",
            body: Some("Steps to reproduce"),
            assignee: Some("dev1"),
            labels: Some("bug,auth"),
            ..Default::default()
        })
        .expect("create should succeed");

    mock.assert();
    assert_eq!(issue.number, "88");
    assert_eq!(issue.title, "Login fails with expired token");
}

#[test]
fn set_state_gets_repo_issue_then_patches_owner_issue_json() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/issues/88";
    let patch_path = "/repos/oschina/issues/88";

    let get = server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    let patch = server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::Regex(r#""repo"\s*:\s*"gitee-cli""#.into()),
            mockito::Matcher::Regex(r#""title"\s*:\s*"Login fails with expired token""#.into()),
            mockito::Matcher::Regex(r#""state"\s*:\s*"closed""#.into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":"88","title":"Login fails with expired token","state":"closed","html_url":"https://gitee.com/oschina/gitee-cli/issues/I88"}"#,
        )
        .create();

    let issue = client(&server)
        .issues(&test_repo())
        .set_state("88", IssueState::Closed)
        .expect("set_state should succeed");

    get.assert();
    patch.assert();
    assert_eq!(issue.state, IssueState::Closed);
    assert_eq!(issue.title, "Login fails with expired token");
}

#[test]
fn link_get_then_patch_json_appends_linked_tag() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/issues/88";
    let patch_path = "/repos/oschina/issues/88";

    let get = server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":"88","title":"Track work","body":"Initial notes","html_url":"https://gitee.com/x"}"#,
        )
        .create();

    let patch = server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::Regex(r"Linked: !7".into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    let linked = client(&server)
        .issues(&test_repo())
        .link("88", "!7")
        .expect("link should succeed");

    get.assert();
    patch.assert();
    assert!(linked);
}

#[test]
fn link_already_linked_skips_patch() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/issues/88";

    let get = server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":"88","title":"Done","body":"Already Linked: !7","html_url":"https://gitee.com/x"}"#,
        )
        .create();

    let patch = server
        .mock("PATCH", api_path("/repos/oschina/issues/88").as_str())
        .expect(0)
        .create();

    let linked = client(&server)
        .issues(&test_repo())
        .link("88", "!7")
        .expect("link should short-circuit");

    get.assert();
    patch.assert();
    assert!(!linked);
}

#[test]
fn comment_posts_form_body_to_issue_comments() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/issues/88/comments";
    let response = r#"{"id":7,"body":"Thanks for the report","html_url":"https://gitee.com/oschina/gitee-cli/issues/I88#note_7"}"#;

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::UrlEncoded(
            "body".into(),
            "Thanks for the report".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response)
        .create();

    let comment = client(&server)
        .issues(&test_repo())
        .comment("88", "Thanks for the report")
        .expect("comment should succeed");

    mock.assert();
    assert_eq!(comment.body, "Thanks for the report");
}

#[test]
fn list_comments_hits_issue_comments_path_and_decodes() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/issues/88/comments";
    let body = r#"[
        {
            "id": 7,
            "body": "first",
            "user": {"login": "dev1"},
            "created_at": "2026-01-01T00:00:00+08:00"
        },
        {
            "id": 9,
            "body": "second",
            "user": {"login": "dev2"},
            "created_at": "2026-01-02T00:00:00+08:00"
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
        .issues(&test_repo())
        .list_comments("88", 30)
        .expect("list_comments should succeed");

    mock.assert();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, 7);
    assert_eq!(items[0].body, "first");
    assert_eq!(items[0].user.as_ref().unwrap().login, "dev1");
    assert_eq!(
        items[0].created_at.as_deref(),
        Some("2026-01-01T00:00:00+08:00")
    );
    assert_eq!(items[1].id, 9);
}

#[test]
fn list_comments_truncates_to_limit() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/issues/88/comments";
    let body = r#"[
        {"id":1,"body":"a"},
        {"id":2,"body":"b"},
        {"id":3,"body":"c"}
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
        .issues(&test_repo())
        .list_comments("88", 2)
        .expect("list_comments should succeed");

    mock.assert();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, 1);
    assert_eq!(items[1].id, 2);
}

#[test]
fn latest_comment_picks_authors_most_recent_by_created_at() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/issues/88/comments";
    let body = r#"[
        {
            "id": 1,
            "body": "older mine",
            "user": {"login": "me"},
            "created_at": "2026-01-01T00:00:00+08:00"
        },
        {
            "id": 2,
            "body": "theirs",
            "user": {"login": "other"},
            "created_at": "2026-01-03T00:00:00+08:00"
        },
        {
            "id": 3,
            "body": "newer mine",
            "user": {"login": "me"},
            "created_at": "2026-01-02T00:00:00+08:00"
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
        .issues(&test_repo())
        .latest_comment("88", "me")
        .expect("latest_comment should succeed");

    mock.assert();
    assert_eq!(comment.id, 3);
    assert_eq!(comment.body, "newer mine");
}

#[test]
fn latest_comment_errors_when_author_has_none() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/issues/88/comments";
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
        .issues(&test_repo())
        .latest_comment("88", "me")
        .expect_err("no comment by me should error");

    mock.assert();
    assert!(
        matches!(err, GiteeError::Usage(_)),
        "expected Usage error, got {err:?}"
    );
}

#[test]
fn latest_comment_paginates_fully_beyond_list_limit() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/issues/88/comments";

    // Full first page (100 items) with no match; second page carries the user's comment.
    let page1: Vec<String> = (1..=100)
        .map(|i| {
            format!(
                r#"{{"id":{i},"body":"p1","user":{{"login":"other"}},"created_at":"2026-01-01T00:00:00+08:00"}}"#
            )
        })
        .collect();
    let page1_body = format!("[{}]", page1.join(","));
    let page2_body = r#"[{"id":101,"body":"mine on page 2","user":{"login":"me"},"created_at":"2026-01-02T00:00:00+08:00"}]"#;

    let m1 = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page1_body)
        .create();
    let m2 = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "2".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(page2_body)
        .create();

    let comment = client(&server)
        .issues(&test_repo())
        .latest_comment("88", "me")
        .expect("latest_comment should page through");

    m1.assert();
    m2.assert();
    assert_eq!(comment.id, 101);
    assert_eq!(comment.body, "mine on page 2");
}

#[test]
fn create_sends_milestone_and_security_hole() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/issues";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("repo".into(), "gitee-cli".into()),
            mockito::Matcher::UrlEncoded("title".into(), "New bug".into()),
            mockito::Matcher::UrlEncoded("milestone".into(), "3".into()),
            mockito::Matcher::UrlEncoded("security_hole".into(), "true".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    client(&server)
        .issues(&test_repo())
        .create(&CreateIssue {
            title: "New bug",
            milestone_number: Some(3),
            security_hole: true,
            ..Default::default()
        })
        .expect("create should succeed");

    mock.assert();
}

#[test]
fn edit_gets_first_then_patches_json_with_echoed_title() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/issues/88";
    let patch_path = "/repos/oschina/issues/88";

    let get = server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    // Fixture title must be echoed (Gitee blanks it otherwise); unset fields absent.
    let patch = server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::Json(serde_json::json!({
            "repo": "gitee-cli",
            "title": "Login fails with expired token",
            "body": "updated body",
            "labels": "bug,ui",
            "security_hole": true
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    client(&server)
        .issues(&test_repo())
        .edit(
            "88",
            &EditIssue {
                body: Some("updated body"),
                labels: Some("bug,ui"),
                security_hole: Some(true),
                ..Default::default()
            },
        )
        .expect("edit should succeed");

    get.assert();
    patch.assert();
}

#[test]
fn edit_state_enterprise_404_is_actionable() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/issues/88";
    let patch_path = "/repos/oschina/issues/88";

    server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    server
        .mock("PATCH", api_path(patch_path).as_str())
        .with_status(404)
        .with_header("content-type", "application/json")
        .with_body(r#"{"message":"project or enterprise"}"#)
        .create();

    let err = client(&server)
        .issues(&test_repo())
        .edit(
            "88",
            &EditIssue {
                state: Some(IssueState::Progressing),
                ..Default::default()
            },
        )
        .expect_err("enterprise 404 should fail");

    match err {
        GiteeError::Api { status, message } => {
            assert_eq!(status, 404);
            assert!(
                message.contains("enterprise/project") || message.contains("enterprise"),
                "expected actionable enterprise hint, got: {message}"
            );
            assert!(
                message.contains("/repos/oschina/issues/88"),
                "expected path tried in message, got: {message}"
            );
        }
        other => panic!("expected Api error, got {other:?}"),
    }
}

#[test]
fn edit_with_state_patches_json_including_state() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/issues/88";
    let patch_path = "/repos/oschina/issues/88";

    let get = server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    let patch = server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::Json(serde_json::json!({
            "repo": "gitee-cli",
            "title": "Login fails with expired token",
            "state": "progressing"
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":"88","title":"Login fails with expired token","state":"progressing","html_url":"https://gitee.com/oschina/gitee-cli/issues/I88"}"#,
        )
        .create();

    let issue = client(&server)
        .issues(&test_repo())
        .edit(
            "88",
            &EditIssue {
                state: Some(IssueState::Progressing),
                ..Default::default()
            },
        )
        .expect("edit with state should succeed");

    get.assert();
    patch.assert();
    assert_eq!(issue.state, IssueState::Progressing);
}

/// open → progressing → closed through `edit` (acceptance: multi-step lifecycle).
#[test]
fn edit_state_open_to_progressing_to_closed() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/issues/88";
    let patch_path = "/repos/oschina/issues/88";
    let repo = test_repo();
    let client = client(&server);

    server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();
    server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::Regex(r#""state"\s*:\s*"progressing""#.into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":"88","title":"Login fails with expired token","state":"progressing","html_url":"https://gitee.com/x"}"#,
        )
        .create();

    let mid = client
        .issues(&repo)
        .edit(
            "88",
            &EditIssue {
                state: Some(IssueState::Progressing),
                ..Default::default()
            },
        )
        .expect("open → progressing");
    assert_eq!(mid.state, IssueState::Progressing);

    server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":"88","title":"Login fails with expired token","state":"progressing","html_url":"https://gitee.com/x"}"#,
        )
        .create();
    server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::Regex(r#""state"\s*:\s*"closed""#.into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":"88","title":"Login fails with expired token","state":"closed","html_url":"https://gitee.com/x"}"#,
        )
        .create();

    let done = client
        .issues(&repo)
        .edit(
            "88",
            &EditIssue {
                state: Some(IssueState::Closed),
                ..Default::default()
            },
        )
        .expect("progressing → closed");
    assert_eq!(done.state, IssueState::Closed);
}

#[test]
fn edit_title_uses_new_title_not_echo() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/issues/88";
    let patch_path = "/repos/oschina/issues/88";

    server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    let patch = server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::Json(serde_json::json!({
            "repo": "gitee-cli",
            "title": "New title",
            "milestone": 3
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    client(&server)
        .issues(&test_repo())
        .edit(
            "88",
            &EditIssue {
                title: Some("New title"),
                milestone_number: Some(3),
                ..Default::default()
            },
        )
        .expect("edit should succeed");

    patch.assert();
}

#[test]
fn list_sends_creator_filter() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/issues";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
            mockito::Matcher::UrlEncoded("creator".into(), "reporter".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!("[{ISSUE_JSON}]"))
        .create();

    let items = client(&server)
        .issues(&test_repo())
        .list(&gitee_cli_rs::api::issues::IssueFilter {
            state: Some("open"),
            creator: Some("reporter"),
            limit: 50,
            ..Default::default()
        })
        .expect("list should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].number, "88");
}

/// Idempotency: closing an already-closed issue short-circuits (no PATCH).
#[test]
fn set_state_idempotent_already_closed_skips_patch() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/issues/88";

    let get = server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":"88","title":"Done","state":"closed","html_url":"https://gitee.com/x"}"#,
        )
        .create();

    let patch = server
        .mock("PATCH", api_path("/repos/oschina/issues/88").as_str())
        .expect(0)
        .create();

    let change = client(&server)
        .issues(&test_repo())
        .set_state_idempotent("88", IssueState::Closed)
        .expect("idempotent close should succeed");

    get.assert();
    patch.assert();
    match change {
        StateChange::Already(issue) => assert_eq!(issue.state, IssueState::Closed),
        other => panic!("expected Already, got {other:?}"),
    }
}

/// Idempotency: closing an open issue PATCHes and returns Changed.
#[test]
fn set_state_idempotent_open_issue_patches_and_returns_changed() {
    let mut server = mockito::Server::new();
    let get_path = "/repos/oschina/gitee-cli/issues/88";
    let patch_path = "/repos/oschina/issues/88";

    server
        .mock("GET", api_path(get_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(ISSUE_JSON)
        .create();

    let patch = server
        .mock("PATCH", api_path(patch_path).as_str())
        .match_body(mockito::Matcher::Regex(r#""state"\s*:\s*"closed""#.into()))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"number":"88","title":"Login fails with expired token","state":"closed","html_url":"https://gitee.com/x"}"#,
        )
        .create();

    let change = client(&server)
        .issues(&test_repo())
        .set_state_idempotent("88", IssueState::Closed)
        .expect("idempotent close should succeed");

    patch.assert();
    match change {
        StateChange::Changed(issue) => assert_eq!(issue.state, IssueState::Closed),
        other => panic!("expected Changed, got {other:?}"),
    }
}
