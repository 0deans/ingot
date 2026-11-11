use super::*;
use crate::{ContentKind, ItemAction, ItemStatus};
use ingot_artifacts::downloader::DownloadError;
use ingot_test_support::{LoopbackServer, ResponseSpec, ScriptedExchange};
use std::error::Error;

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
}

#[tokio::test]
async fn staging_preserves_download_failure_category_and_redacted_source() {
    let fixture = LoopbackServer::exact_script(vec![
        ScriptedExchange::any_path(ResponseSpec::status(503)),
        ScriptedExchange::any_path(ResponseSpec::status(503)),
    ])
    .await;
    let address = fixture.addr();
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
    let _ = fixture.finish().await;
}
