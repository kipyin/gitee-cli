use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::labels::{CreateLabel, EditLabel};
use gitee_cli_rs::repo::Repo;

const LABEL_JSON: &str = r#"{"id":12345,"name":"bug","color":"ff0000"}"#;

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
fn create_posts_form_with_name_and_normalized_color() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/labels";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("name".into(), "bug".into()),
            mockito::Matcher::UrlEncoded("color".into(), "ff0000".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_JSON)
        .create();

    let label = client(&server)
        .labels(&test_repo())
        .create(&CreateLabel {
            name: "bug",
            color: "#FF0000",
        })
        .expect("create should succeed");

    mock.assert();
    assert_eq!(label.name, "bug");
    assert_eq!(label.id, 12345);
}

#[test]
fn list_decodes_array_and_truncates_to_limit() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/labels";
    let body = r#"[
        {"id":1,"name":"a","color":"111111"},
        {"id":2,"name":"b","color":"222222"},
        {"id":3,"name":"c","color":"333333"}
    ]"#;

    let mock = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(body)
        .create();

    let items = client(&server)
        .labels(&test_repo())
        .list(2)
        .expect("list should succeed");

    mock.assert();
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].name, "a");
    assert_eq!(items[1].name, "b");
}

#[test]
fn edit_patches_original_name_with_only_provided_fields() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/labels/bug";

    let mock = server
        .mock("PATCH", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("name".into(), "feature".into()),
            mockito::Matcher::UrlEncoded("color".into(), "00ff00".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":12345,"name":"feature","color":"00ff00"}"#)
        .create();

    let label = client(&server)
        .labels(&test_repo())
        .edit(
            "bug",
            &EditLabel {
                name: Some("feature"),
                color: Some("#00FF00"),
            },
        )
        .expect("edit should succeed");

    mock.assert();
    assert_eq!(label.name, "feature");
}

#[test]
fn edit_sends_only_color_when_name_omitted() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/labels/bug";

    let mock = server
        .mock("PATCH", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![mockito::Matcher::UrlEncoded(
            "color".into(),
            "0000ff".into(),
        )]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(LABEL_JSON)
        .create();

    client(&server)
        .labels(&test_repo())
        .edit(
            "bug",
            &EditLabel {
                name: None,
                color: Some("0000ff"),
            },
        )
        .expect("edit should succeed");

    mock.assert();
}

#[test]
fn delete_hits_labels_path() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/labels/bug";

    let mock = server
        .mock("DELETE", api_path(path).as_str())
        .with_status(204)
        .create();

    client(&server)
        .labels(&test_repo())
        .delete("bug")
        .expect("delete should succeed");

    mock.assert();
}
