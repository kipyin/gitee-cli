use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::issues::CreateIssue;
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
