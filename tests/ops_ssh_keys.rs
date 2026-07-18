use gitee_cli_rs::api::client::Client;

const KEY_JSON: &str = r#"{"id":99,"key":"ssh-ed25519 AAAA test@host","title":"laptop","created_at":"2026-07-18T00:00:00+08:00"}"#;

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

#[test]
fn list_keys_hits_user_keys_and_truncates() {
    let mut server = mockito::Server::new();
    let body = format!("[{KEY_JSON},{KEY_JSON},{KEY_JSON}]");

    let mock = server
        .mock("GET", api_path("/user/keys").as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    let items = client(&server).users().keys(2).expect("list keys");
    mock.assert();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].id, 99);
    assert_eq!(items[0].title.as_deref(), Some("laptop"));
}

#[test]
fn add_key_posts_key_and_title() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("POST", api_path("/user/keys").as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::Regex(r"key=ssh-ed25519\+AAAA".into()),
            mockito::Matcher::Regex("title=laptop".into()),
        ]))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(KEY_JSON)
        .create();

    let key = client(&server)
        .users()
        .add_key("ssh-ed25519 AAAA", "laptop")
        .expect("add key");
    mock.assert();
    assert_eq!(key.id, 99);
    assert_eq!(key.title.as_deref(), Some("laptop"));
}

#[test]
fn delete_key_hits_id_path() {
    let mut server = mockito::Server::new();

    let mock = server
        .mock("DELETE", api_path("/user/keys/99").as_str())
        .with_status(204)
        .create();

    client(&server).users().delete_key(99).expect("delete key");
    mock.assert();
}
