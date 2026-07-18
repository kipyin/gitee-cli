use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::gists::{CreateGist, UpdateGist};

const GIST_JSON: &str = include_str!("fixtures/gist.json");

fn client(server: &mockito::ServerGuard) -> Client {
    Client::new(format!("{}/api/v5", server.url()), "fake-token".into())
}

fn api_path(path: &str) -> String {
    format!("/api/v5{path}")
}

#[test]
fn create_posts_nested_form_with_description_and_public() {
    let mut server = mockito::Server::new();
    let path = "/gists";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("description".into(), "my snippet".into()),
            mockito::Matcher::UrlEncoded("public".into(), "true".into()),
            mockito::Matcher::UrlEncoded("files[a.txt][content]".into(), "hello".into()),
        ]))
        .with_status(201)
        .with_header("content-type", "application/json")
        .with_body(GIST_JSON)
        .create();

    let files = [("a.txt".to_string(), "hello".to_string())];
    let gist = client(&server)
        .gists()
        .create(&CreateGist {
            description: "my snippet",
            public: true,
            files: &files,
        })
        .expect("create should succeed");

    mock.assert();
    assert_eq!(gist.id, "abc123def456");
    assert_eq!(gist.description.as_deref(), Some("test gist snippet"));
}

#[test]
fn list_paged_decodes_fixture() {
    let mut server = mockito::Server::new();
    let path = "/gists";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .match_query(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("page".into(), "1".into()),
            mockito::Matcher::UrlEncoded("per_page".into(), "100".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(format!("[{GIST_JSON}]"))
        .create();

    let items = client(&server)
        .gists()
        .list(10)
        .expect("list should succeed");

    mock.assert();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "abc123def456");
}

#[test]
fn get_by_id_hits_gists_path() {
    let mut server = mockito::Server::new();
    let id = "abc123def456";
    let path = format!("/gists/{id}");

    let mock = server
        .mock("GET", api_path(&path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(GIST_JSON)
        .create();

    let gist = client(&server)
        .gists()
        .get(id)
        .expect("get should succeed");

    mock.assert();
    assert_eq!(gist.id, id);
    assert_eq!(
        gist.files
            .as_ref()
            .and_then(|f| f.get("a.txt"))
            .and_then(|f| f.content.as_deref()),
        Some("hello world\n")
    );
}

#[test]
fn edit_patches_nested_file_content() {
    let mut server = mockito::Server::new();
    let id = "abc123def456";
    let path = format!("/gists/{id}");

    let mock = server
        .mock("PATCH", api_path(&path).as_str())
        .match_body(mockito::Matcher::UrlEncoded(
            "files[a.txt][content]".into(),
            "updated content".into(),
        ))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(GIST_JSON)
        .create();

    let files = [("a.txt".to_string(), "updated content".to_string())];
    let gist = client(&server)
        .gists()
        .update(
            id,
            &UpdateGist {
                files: &files,
                description: None,
            },
        )
        .expect("update should succeed");

    mock.assert();
    assert_eq!(gist.id, id);
}

#[test]
fn delete_hits_delete_gists_id() {
    let mut server = mockito::Server::new();
    let id = "abc123def456";
    let path = format!("/gists/{id}");

    let mock = server
        .mock("DELETE", api_path(&path).as_str())
        .with_status(204)
        .create();

    client(&server)
        .gists()
        .delete(id)
        .expect("delete should succeed");

    mock.assert();
}
