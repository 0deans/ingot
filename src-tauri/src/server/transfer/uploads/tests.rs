use super::*;

#[tokio::test]
async fn finalization_lease_excludes_chunks_until_worker_finishes() {
    let manager = Arc::new(UploadManager::default());
    let lease = manager.claim("upload").await.unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let worker = tokio::task::spawn_blocking(move || {
        let _lease = lease;
        entered_tx.send(()).unwrap();
        release_rx.recv().unwrap();
    });
    entered_rx.await.unwrap();
    worker.abort();
    assert!(
        tokio::time::timeout(std::time::Duration::ZERO, manager.claim("upload"))
            .await
            .is_err()
    );
    release_tx.send(()).unwrap();
    worker.await.unwrap();
    assert!(manager.claim("upload").await.is_ok());
}

#[tokio::test]
async fn failed_import_preserves_archive_for_retry() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("upload.zip");
    fs::write(&archive, b"broken zip").unwrap();
    let service = super::super::tests::service(root.path());
    let manager = UploadManager::default();
    let lease = manager.claim("upload").await.unwrap();
    assert!(matches!(
        finalize(lease, &service, &archive),
        Err(TransferError::InvalidZip(_))
    ));
    assert_eq!(fs::read(&archive).unwrap(), b"broken zip");
}

#[tokio::test]
async fn restart_receipt_reuses_committed_server_and_cleans_archive() {
    let root = tempfile::tempdir().unwrap();
    let service = super::super::tests::service(root.path());
    let mut config = super::super::tests::config();
    config.id = uuid::Uuid::new_v4().to_string();
    fs::create_dir(service.servers_dir.join(&config.id)).unwrap();
    fs::write(
        root.path().join("servers.json"),
        serde_json::to_vec(&[config.clone()]).unwrap(),
    )
    .unwrap();
    let archive = root.path().join("upload.zip");
    fs::write(&archive, b"already imported").unwrap();
    fs::write(
        archive.with_extension("import.json"),
        serde_json::to_vec(&ImportReceipt {
            version: 1,
            server_id: config.id.clone(),
        })
        .unwrap(),
    )
    .unwrap();
    let manager = UploadManager::default();
    let lease = manager.claim("upload").await.unwrap();
    assert_eq!(finalize(lease, &service, &archive).unwrap().id, config.id);
    assert!(!archive.exists());
    let lease = manager.claim("upload").await.unwrap();
    assert!(matches!(
        write_chunk(lease, &archive, b"new", true),
        Err(TransferError::UploadFinalized)
    ));
    assert_eq!(service.repository.load().unwrap().len(), 1);
}

#[tokio::test]
async fn cleanup_failure_retries_without_importing_again() {
    let root = tempfile::tempdir().unwrap();
    let service = super::super::tests::service(root.path());
    let config = super::super::tests::config();
    let archive = root.path().join("upload.zip");
    fs::create_dir(&archive).unwrap();
    let manager = UploadManager::default();
    let mut lease = manager.claim("upload").await.unwrap();
    *lease = UploadState::Imported(config.clone());
    assert!(matches!(
        finalize(lease, &service, &archive),
        Err(TransferError::UploadCleanup { .. })
    ));
    fs::remove_dir(&archive).unwrap();
    fs::write(&archive, b"cleanup retry").unwrap();
    let lease = manager.claim("upload").await.unwrap();
    assert_eq!(finalize(lease, &service, &archive).unwrap().id, config.id);
    assert!(!archive.exists());
    assert!(service.repository.load().unwrap().is_empty());
}

#[tokio::test]
async fn pending_restart_receipt_prevents_chunk_from_overwriting_archive() {
    let root = tempfile::tempdir().unwrap();
    let archive = root.path().join("upload.zip");
    fs::write(&archive, b"preserved archive").unwrap();
    fs::write(archive.with_extension("import.json"), b"unknown receipt").unwrap();
    let manager = UploadManager::default();
    let lease = manager.claim("upload").await.unwrap();
    assert!(matches!(
        write_chunk(lease, &archive, b"replacement", true),
        Err(TransferError::UploadFinalized)
    ));
    assert_eq!(fs::read(&archive).unwrap(), b"preserved archive");
}

#[tokio::test]
async fn completed_sessions_do_not_exhaust_upload_admission() {
    let manager = UploadManager::default();
    let config = super::super::tests::config();
    for index in 0..MAX_UPLOADS {
        let mut lease = manager.claim(&format!("upload-{index}")).await.unwrap();
        *lease = UploadState::Finished(config.clone());
    }
    assert!(manager.claim("new-upload").await.is_ok());
    assert_eq!(manager.entries.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn malformed_receipt_is_retained_and_does_not_create_server() {
    let root = tempfile::tempdir().unwrap();
    let service = super::super::tests::service(root.path());
    let archive = root.path().join("upload.zip");
    fs::write(&archive, b"archive retained").unwrap();
    let path = archive.with_extension("import.json");
    let bytes = br#"{"version":1,"server_id":"../invalid"}"#;
    fs::write(&path, bytes).unwrap();
    let manager = UploadManager::default();
    let lease = manager.claim("upload").await.unwrap();
    assert!(matches!(
        finalize(lease, &service, &archive),
        Err(TransferError::Io(_))
    ));
    assert_eq!(fs::read(path).unwrap(), bytes);
    assert_eq!(fs::read(archive).unwrap(), b"archive retained");
    assert!(service.repository.load().unwrap().is_empty());
}

#[cfg(unix)]
#[tokio::test]
async fn linked_archive_cannot_overwrite_external_data() {
    let root = tempfile::tempdir().unwrap();
    let target = root.path().join("target.zip");
    fs::write(&target, b"external data").unwrap();
    let archive = root.path().join("upload.zip");
    std::os::unix::fs::symlink(&target, &archive).unwrap();
    let manager = UploadManager::default();
    let lease = manager.claim("upload").await.unwrap();
    assert!(matches!(
        write_chunk(lease, &archive, b"replacement", true),
        Err(TransferError::UnsupportedEntry)
    ));
    assert_eq!(fs::read(target).unwrap(), b"external data");
}
