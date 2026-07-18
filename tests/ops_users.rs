use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::users::UserIssueFilter;

const ISSUE_JSON: &str = include_str!("fixtures/issue.json");

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

#[test]
fn me_hits_user_path_and_deserializes_login() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", api_path("/user").as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id": 42, "login": "dev1", "name": "Developer One"}"#)
        .create();

    let me = client(&server).users().me().expect("me should succeed");

    mock.assert();
    assert_eq!(me.login, "dev1");
    assert_eq!(me.name.as_deref(), Some("Developer One"));
}

#[test]
fn user_issues_sends_filter_and_state_single_page() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", api_path("/user/issues").as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
            mockito::Matcher::UrlEncoded("filter".into(), "assigned".into()),
            mockito::Matcher::UrlEncoded("state".into(), "open".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!("[{ISSUE_JSON}]"))
        .create();

    let items = client(&server)
        .users()
        .issues(&UserIssueFilter {
            filter: "assigned",
            state: Some("open"),
            limit: 50,
        })
        .expect("user issues should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].number, "88");
    assert_eq!(items[0].title, "Login fails with expired token");
}

#[test]
fn user_issues_sends_created_filter_and_parses_empty_page() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("GET", api_path("/user/issues").as_str())
        .match_query(mockito::Matcher::AllOf(vec![mockito::Matcher::UrlEncoded(
            "filter".into(),
            "created".into(),
        )]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create();

    let items = client(&server)
        .users()
        .issues(&UserIssueFilter {
            filter: "created",
            state: None,
            limit: 50,
        })
        .expect("user issues should succeed");

    mock.assert();
    assert!(items.is_empty());
}
