use gitee_cli_rs::api::client::Client;

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

#[test]
fn orgs_lists_user_orgs_and_respects_limit() {
    let mut server = mockito::Server::new();

    // Swagger `Group` shape: no `name` or `role` on GET /user/orgs.
    let body = r#"[
        {"id":1,"login":"acme","url":"https://gitee.com/acme","description":"Acme open source"},
        {"id":2,"login":"beta","url":"https://gitee.com/beta","description":"Beta team"},
        {"id":3,"login":"gamma","url":"https://gitee.com/gamma","description":""}
    ]"#;

    let mock = server
        .mock("GET", api_path("/user/orgs").as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    let items = client(&server)
        .users()
        .orgs(2)
        .expect("orgs should succeed");

    mock.assert();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].login, "acme");
    assert_eq!(items[0].description.as_deref(), Some("Acme open source"));
    assert_eq!(items[1].login, "beta");
}
