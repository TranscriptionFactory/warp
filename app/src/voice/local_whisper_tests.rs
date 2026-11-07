use sha2::{Digest as _, Sha256};

use super::download_file;

const BODY: &[u8] = b"model bytes";

async fn download(
    server: &mockito::Server,
    dir: &std::path::Path,
    sha256: &str,
) -> anyhow::Result<()> {
    let url = format!("{}/model", server.url());
    download_file(
        &http_client::Client::new_for_test(),
        &url,
        &dir.join("model.safetensors"),
        sha256,
    )
    .await
}

#[tokio::test]
async fn matching_hash_installs_the_file() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/model")
        .with_body(BODY)
        .create_async()
        .await;
    let dir = tempfile::tempdir().unwrap();

    let sha256 = hex::encode(Sha256::digest(BODY));
    download(&server, dir.path(), &sha256).await.unwrap();

    assert_eq!(
        std::fs::read(dir.path().join("model.safetensors")).unwrap(),
        BODY
    );
    assert!(!dir.path().join("model.part").exists());
}

#[tokio::test]
async fn wrong_hash_leaves_nothing_behind() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/model")
        .with_body(BODY)
        .create_async()
        .await;
    let dir = tempfile::tempdir().unwrap();

    let error = download(&server, dir.path(), "0000").await.unwrap_err();

    assert!(error.to_string().contains("expected SHA-256"), "{error}");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn http_error_leaves_nothing_behind() {
    let mut server = mockito::Server::new_async().await;
    let _mock = server
        .mock("GET", "/model")
        .with_status(503)
        .create_async()
        .await;
    let dir = tempfile::tempdir().unwrap();

    download(&server, dir.path(), "0000").await.unwrap_err();

    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}
