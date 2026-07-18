use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::webhooks::CreateWebhook;
use gitee_cli_rs::repo::Repo;

const HOOK_JSON: &str = r#"{"id":55,"url":"https://example.com/hook","password":"","result_code":0,"result_msg":""}"#;

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

fn test_repo() -> Repo {
    Repo {
        owner: "oschina".into(),
        name: "gitee-cli".into(),
    }
}

#[test]
fn list_webhooks_hits_hooks_path() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/hooks";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!("[{HOOK_JSON}]"))
        .create();

    let items = client(&server)
        .webhooks(&test_repo())
        .list(30)
        .expect("list");
    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, 55);
    assert_eq!(items[0].url.as_deref(), Some("https://example.com/hook"));
}

#[test]
fn create_webhook_posts_url_events_and_password() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/hooks";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::Regex("url=https%3A%2F%2Fexample.com%2Fhook".into()),
            mockito::Matcher::Regex("password=s3cret".into()),
            mockito::Matcher::Regex("push_events=true".into()),
            mockito::Matcher::Regex("tag_push_events=false".into()),
            mockito::Matcher::Regex("issues_events=true".into()),
            mockito::Matcher::Regex("pull_requests_events=false".into()),
            mockito::Matcher::Regex("note_events=false".into()),
        ]))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(HOOK_JSON)
        .create();

    let hook = client(&server)
        .webhooks(&test_repo())
        .create(&CreateWebhook {
            url: "https://example.com/hook",
            password: Some("s3cret"),
            push_events: true,
            tag_push_events: false,
            issues_events: true,
            pull_requests_events: false,
            note_events: false,
        })
        .expect("create");
    mock.assert();
    assert_eq!(hook.id, 55);
}

#[test]
fn delete_webhook_hits_id_path() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/hooks/55";

    let mock = server
        .mock("DELETE", api_path(path).as_str())
        .with_status(204)
        .create();

    client(&server)
        .webhooks(&test_repo())
        .delete(55)
        .expect("delete");
    mock.assert();
}
