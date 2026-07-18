use gitee_cli_rs::api::client::Client;
use gitee_cli_rs::api::releases::CreateRelease;
use gitee_cli_rs::repo::Repo;

const RELEASE_JSON: &str = include_str!("fixtures/release.json");

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
fn create_posts_form_with_name_default_and_prerelease_flag() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/releases";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("tag_name".into(), "v1.3.0".into()),
            mockito::Matcher::UrlEncoded("name".into(), "v1.3.0".into()),
            mockito::Matcher::UrlEncoded("body".into(), "Release notes".into()),
            mockito::Matcher::UrlEncoded("target_commitish".into(), "master".into()),
            mockito::Matcher::UrlEncoded("prerelease".into(), "false".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(RELEASE_JSON)
        .create();

    let release = client(&server)
        .releases(&test_repo())
        .create(&CreateRelease {
            tag: "v1.3.0",
            name: None,
            notes: Some("Release notes"),
            target: Some("master"),
            prerelease: false,
        })
        .expect("create should succeed");

    mock.assert();
    assert_eq!(release.tag_name, "v1.2.0");
    assert_eq!(release.id, 12345);
}

#[test]
fn create_prerelease_true_sends_string_true() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/releases";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("tag_name".into(), "v2.0.0-rc1".into()),
            mockito::Matcher::UrlEncoded("name".into(), "RC 1".into()),
            mockito::Matcher::UrlEncoded("prerelease".into(), "true".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(RELEASE_JSON)
        .create();

    client(&server)
        .releases(&test_repo())
        .create(&CreateRelease {
            tag: "v2.0.0-rc1",
            name: Some("RC 1"),
            notes: None,
            target: None,
            prerelease: true,
        })
        .expect("create should succeed");

    mock.assert();
}

#[test]
fn get_by_tag_hits_tags_path() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/releases/tags/v1.2.0";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(RELEASE_JSON)
        .create();

    let release = client(&server)
        .releases(&test_repo())
        .get_by_tag("v1.2.0")
        .expect("get_by_tag should succeed");

    mock.assert();
    assert_eq!(release.tag_name, "v1.2.0");
    assert_eq!(release.assets.as_ref().expect("assets").len(), 2);
}

#[test]
fn upload_gets_release_then_posts_multipart_attach_files() {
    let mut server = mockito::Server::new();
    let tag_path = "/repos/oschina/gitee-cli/releases/tags/v1.2.0";
    let attach_path = "/repos/oschina/gitee-cli/releases/12345/attach_files";

    let get = server
        .mock("GET", api_path(tag_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(RELEASE_JSON)
        .create();

    let upload = server
        .mock("POST", api_path(attach_path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"name":"upload.bin","browser_download_url":"https://gitee.com/oschina/gitee-cli/releases/download/v1.2.0/upload.bin"}"#,
        )
        .create();

    let mut file_path = std::env::temp_dir();
    file_path.push("gitee-cli-test-upload.bin");
    std::fs::write(&file_path, b"fake release asset").expect("write temp file");

    let asset = client(&server)
        .releases(&test_repo())
        .upload("v1.2.0", file_path.to_str().expect("utf-8 path"))
        .expect("upload should succeed");

    get.assert();
    upload.assert();
    assert_eq!(asset.name, "upload.bin");
    let _ = std::fs::remove_file(&file_path);
}

/// Regression (found by the release mirror job): Gitee rejects release create
/// without a non-empty `body`, so it defaults to the display name.
#[test]
fn create_without_notes_defaults_body_to_name() {
    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/releases";

    let mock = server
        .mock("POST", api_path(path).as_str())
        .match_body(mockito::Matcher::AllOf(vec![
            mockito::Matcher::UrlEncoded("tag_name".into(), "v1.3.0".into()),
            mockito::Matcher::UrlEncoded("body".into(), "v1.3.0".into()),
        ]))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(RELEASE_JSON)
        .create();

    client(&server)
        .releases(&test_repo())
        .create(&CreateRelease {
            tag: "v1.3.0",
            name: None,
            notes: None,
            target: None,
            prerelease: false,
        })
        .expect("create should succeed");

    mock.assert();
}

/// Regression: Gitee returns HTTP 200 with a JSON `null` body for a missing
/// release tag — must surface as NotFound, not a decode error.
#[test]
fn get_by_tag_null_body_maps_to_not_found() {
    use gitee_cli_rs::error::GiteeError;

    let mut server = mockito::Server::new();
    let path = "/repos/oschina/gitee-cli/releases/tags/v9.9.9";

    let mock = server
        .mock("GET", api_path(path).as_str())
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("null")
        .create();

    let err = client(&server)
        .releases(&test_repo())
        .get_by_tag("v9.9.9")
        .expect_err("null body should map to NotFound");

    mock.assert();
    assert!(matches!(err, GiteeError::NotFound(_)), "got {err:?}");
}
