use super::*;
use crate::minecraft::downloader::DownloadError;
use crate::version_change::{ContentKind, ItemAction, ItemStatus};
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::test]
async fn failed_staging_removal_preserves_existing_file_and_io_cause() {
    let directory = tempfile::tempdir().unwrap();
    let staging = directory.path().join("staging");
    fs::write(&staging, b"recovery material").unwrap();
    let result = stage_typed(&reqwest::Client::new(), &staging, &[], &|_| {}).await;
    let error = result.unwrap_err();
    assert!(matches!(
        &error,
        VersionChangeError::DownloadPrep { operation: "remove previous staging directory", path, .. }
            if path == &staging
    ));
    assert!(error.source().unwrap().is::<std::io::Error>());
    assert_eq!(fs::read(staging).unwrap(), b"recovery material");
    assert!(matches!(
        crate::ipc::ApiError::from(error),
        crate::ipc::ApiError::StorageFailure { .. }
    ));
}

#[tokio::test]
async fn staging_preserves_download_failure_category_and_redacted_source() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let provider = tokio::spawn(async move {
        for _ in 0..2 {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                assert!(request.len() < 8192, "Request headers exceed fixture limit");
                request.push(socket.read_u8().await.unwrap());
            }
            socket
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
        }
    });
    let client = reqwest::Client::builder()
        .no_proxy()
        .resolve("cdn.modrinth.com", address)
        .build()
        .unwrap();
    let url = format!(
        "http://cdn.modrinth.com:{}/artifact.jar?token=staging-secret",
        address.port()
    );
    let item = PlanItem {
        kind: ContentKind::Mod,
        folder: "mods".into(),
        file_name: None,
        title: "Example mod".into(),
        icon_url: None,
        page_url: None,
        source: None,
        project_id: None,
        current_version: None,
        target: Some(TargetFile {
            version_id: "target".into(),
            version_number: "1".into(),
            file_name: "artifact.jar".into(),
            url,
            sha1: Some("0000000000000000000000000000000000000000".into()),
            sha256: None,
            bytes: 1.0,
        }),
        status: ItemStatus::NewDependency,
        action: ItemAction::Add,
        actions: vec![ItemAction::Add],
        note: None,
    };
    let directory = tempfile::tempdir().unwrap();
    let error = stage_typed(&client, directory.path(), &[&item], &|_| {})
        .await
        .unwrap_err();
    assert!(matches!(
        &error,
        VersionChangeError::DownloadFailed { title, source: DownloadError::HttpStatus { status } }
            if title == "Example mod" && *status == reqwest::StatusCode::SERVICE_UNAVAILABLE
    ));
    assert!(error.source().unwrap().is::<DownloadError>());
    assert!(!format!("{error:?}").contains("staging-secret"));
    assert!(!directory.path().join("mods/artifact.jar").exists());
    assert!(matches!(
        crate::ipc::ApiError::from(error),
        crate::ipc::ApiError::ProviderUnavailable { .. }
    ));
    provider.await.unwrap();
}
