use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::repo::Repo;

const COLLAB_JSON: &str = r#"{"id":7,"login":"alice","name":"Alice","permissions":{"pull":true,"push":true,"admin":false}}"#;

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
fn list_collaborators_hits_repo_path_and_truncates() {
    let mut server = mockito::Server::new();
    let body = format!("[{COLLAB_JSON},{COLLAB_JSON},{COLLAB_JSON}]");
    let path = "/repos/oschina/gitee-cli/collaborators";

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
        .collaborators(&test_repo())
        .list(2)
        .expect("list");
    mock.assert();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].login, "alice");
}

#[test]
fn add_collaborator_puts_with_permission() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/collaborators/alice";

    let mock = server
        .mock("PUT", api_path(path).as_str())
        .match_body(mockito::Matcher::Regex("permission=push".into()))
        .with_status(204)
        .create();

    client(&server)
        .collaborators(&test_repo())
        .add("alice", "push")
        .expect("add");
    mock.assert();
}

#[test]
fn remove_collaborator_deletes_username_path() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/collaborators/alice";

    let mock = server
        .mock("DELETE", api_path(path).as_str())
        .with_status(204)
        .create();

    client(&server)
        .collaborators(&test_repo())
        .remove("alice")
        .expect("remove");
    mock.assert();
}
