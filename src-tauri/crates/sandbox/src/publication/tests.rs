use super::*;

fn rootfs(path: &Path, text: &str) {
    fs::create_dir_all(path.join("sbin")).unwrap();
    fs::write(path.join("sbin/apk"), text).unwrap();
    fs::write(path.join(BASE_MARKER), ALPINE_VERSION).unwrap();
}

#[test]
fn prepared_recovery_restores_previous_after_destination_was_replaced() {
    let temp = tempfile::tempdir().unwrap();
    rootfs(&temp.path().join(PREVIOUS), "original");
    rootfs(&temp.path().join("rootfs"), "replacement");
    save_receipt(
        temp.path(),
        &Receipt {
            version: 1,
            had_previous: true,
            phase: Phase::Prepared,
        },
    )
    .unwrap();
    recover(temp.path()).unwrap();
    recover(temp.path()).unwrap();
    assert_eq!(
        fs::read_to_string(temp.path().join("rootfs/sbin/apk")).unwrap(),
        "original"
    );
    assert!(!temp.path().join(RECEIPT).exists());
}

#[test]
fn prepared_recovery_before_first_rename_preserves_original() {
    let temp = tempfile::tempdir().unwrap();
    rootfs(&temp.path().join("rootfs"), "original");
    rootfs(&temp.path().join(STAGE), "replacement");
    save_receipt(
        temp.path(),
        &Receipt {
            version: 1,
            had_previous: true,
            phase: Phase::Prepared,
        },
    )
    .unwrap();
    recover(temp.path()).unwrap();
    assert_eq!(
        fs::read_to_string(temp.path().join("rootfs/sbin/apk")).unwrap(),
        "original"
    );
    assert!(!temp.path().join(STAGE).exists());
}

#[test]
fn committed_recovery_keeps_new_tree_and_cleans_backup() {
    let temp = tempfile::tempdir().unwrap();
    rootfs(&temp.path().join("rootfs"), "replacement");
    rootfs(&temp.path().join(PREVIOUS), "original");
    save_receipt(
        temp.path(),
        &Receipt {
            version: 1,
            had_previous: true,
            phase: Phase::Committed,
        },
    )
    .unwrap();
    recover(temp.path()).unwrap();
    assert_eq!(
        fs::read_to_string(temp.path().join("rootfs/sbin/apk")).unwrap(),
        "replacement"
    );
    assert!(!temp.path().join(PREVIOUS).exists());
}

#[test]
fn cleanup_failure_retains_receipt_and_usable_tree_for_retry() {
    let temp = tempfile::tempdir().unwrap();
    rootfs(&temp.path().join("rootfs"), "replacement");
    fs::write(temp.path().join(PREVIOUS), "blocks directory cleanup").unwrap();
    save_receipt(
        temp.path(),
        &Receipt {
            version: 1,
            had_previous: true,
            phase: Phase::Committed,
        },
    )
    .unwrap();
    assert!(recover(temp.path()).is_err());
    assert!(temp.path().join(RECEIPT).exists());
    assert!(is_rootfs_installed(&temp.path().join("rootfs")).unwrap());
    fs::remove_file(temp.path().join(PREVIOUS)).unwrap();
    recover(temp.path()).unwrap();
}

#[test]
fn corrupt_unknown_or_ambiguous_receipts_block_and_preserve_material() {
    let temp = tempfile::tempdir().unwrap();
    rootfs(&temp.path().join(PREVIOUS), "original");
    assert!(matches!(
        recover(temp.path()),
        Err(SandboxError::RecoveryBlocked(_))
    ));
    fs::write(temp.path().join(RECEIPT), "invalid").unwrap();
    assert!(matches!(
        recover(temp.path()),
        Err(SandboxError::Receipt(_))
    ));
    save_receipt(
        temp.path(),
        &Receipt {
            version: 42,
            had_previous: true,
            phase: Phase::Prepared,
        },
    )
    .unwrap();
    assert!(matches!(
        recover(temp.path()),
        Err(SandboxError::RecoveryBlocked(_))
    ));
    assert!(temp.path().join(PREVIOUS).exists());
    assert!(temp.path().join(RECEIPT).exists());
}

#[test]
fn failed_new_installation_rolls_back_to_absence() {
    let temp = tempfile::tempdir().unwrap();
    rootfs(&temp.path().join("rootfs"), "new");
    save_receipt(
        temp.path(),
        &Receipt {
            version: 1,
            had_previous: false,
            phase: Phase::Prepared,
        },
    )
    .unwrap();
    recover(temp.path()).unwrap();
    assert!(!temp.path().join("rootfs").exists());
}

#[test]
fn publication_commits_replacement_then_acknowledges() {
    let temp = tempfile::tempdir().unwrap();
    rootfs(&temp.path().join("rootfs"), "old");
    rootfs(&temp.path().join(STAGE), "new");
    publish(temp.path()).unwrap();
    assert_eq!(
        fs::read_to_string(temp.path().join("rootfs/sbin/apk")).unwrap(),
        "new"
    );
    assert!(!temp.path().join(RECEIPT).exists());
    assert!(!temp.path().join(PREVIOUS).exists());
}

#[tokio::test]
async fn malformed_download_preserves_existing_rootfs() {
    let temp = tempfile::tempdir().unwrap();
    rootfs(&temp.path().join("rootfs"), "original");
    let server = ingot_test_support::LoopbackServer::exact_script(vec![
        ingot_test_support::ScriptedExchange::expect_path(
            "/rootfs",
            ingot_test_support::ResponseSpec::text(200, "invalid"),
        ),
    ])
    .await;
    let endpoint = server.url("/rootfs");
    assert!(
        install(&reqwest::Client::new(), temp.path(), &endpoint, &|_, _| {})
            .await
            .is_err()
    );
    let _ = server.finish().await;
    assert_eq!(
        fs::read_to_string(temp.path().join("rootfs/sbin/apk")).unwrap(),
        "original"
    );
    assert!(!temp.path().join(RECEIPT).exists());
}

#[cfg(unix)]
#[test]
fn archive_created_config_symlink_cannot_overwrite_host_file() {
    let temp = tempfile::tempdir().unwrap();
    let external = temp.path().join("host-resolv.conf");
    fs::write(&external, "host original").unwrap();
    let sandbox = temp.path().join("sandbox");
    fs::create_dir(&sandbox).unwrap();
    rootfs(&sandbox.join("rootfs"), "original");
    let archive_path = temp.path().join("rootfs.tar.gz");
    let gzip = flate2::write::GzEncoder::new(
        File::create(&archive_path).unwrap(),
        flate2::Compression::default(),
    );
    let mut archive = tar::Builder::new(gzip);
    let mut header = tar::Header::new_gnu();
    header.set_size(3);
    header.set_mode(0o755);
    header.set_cksum();
    archive
        .append_data(&mut header, "sbin/apk", &b"apk"[..])
        .unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Directory);
    header.set_size(0);
    header.set_mode(0o755);
    header.set_cksum();
    archive
        .append_data(&mut header, "etc/apk", io::empty())
        .unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o777);
    archive
        .append_link(&mut header, "etc/resolv.conf", &external)
        .unwrap();
    archive.into_inner().unwrap().finish().unwrap();
    assert!(extract_and_publish(&sandbox, &archive_path).is_err());
    assert_eq!(fs::read_to_string(&external).unwrap(), "host original");
    assert!(!sandbox.join(STAGE).exists());
    assert_eq!(
        fs::read_to_string(sandbox.join("rootfs/sbin/apk")).unwrap(),
        "original"
    );
}

#[cfg(unix)]
#[test]
fn linked_publication_target_is_rejected_without_touching_destination() {
    let temp = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    rootfs(external.path(), "external");
    std::os::unix::fs::symlink(external.path(), temp.path().join("rootfs")).unwrap();
    assert!(matches!(
        recover(temp.path()),
        Err(SandboxError::RecoveryBlocked(_))
    ));
    assert_eq!(
        fs::read_to_string(external.path().join("sbin/apk")).unwrap(),
        "external"
    );
}

#[tokio::test]
async fn injected_rootfs_download_validates_and_publishes_complete_tree() {
    let temp = tempfile::tempdir().unwrap();
    rootfs(&temp.path().join("rootfs"), "original");
    let gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    let mut archive = tar::Builder::new(gzip);
    let mut header = tar::Header::new_gnu();
    header.set_size(3);
    header.set_mode(0o755);
    header.set_cksum();
    archive
        .append_data(&mut header, "sbin/apk", &b"apk"[..])
        .unwrap();
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Directory);
    header.set_size(0);
    header.set_mode(0o755);
    header.set_cksum();
    archive
        .append_data(&mut header, "etc/apk", io::empty())
        .unwrap();
    let bytes = archive.into_inner().unwrap().finish().unwrap();
    let server = ingot_test_support::LoopbackServer::exact_script(vec![
        ingot_test_support::ScriptedExchange::expect_path(
            "/rootfs",
            ingot_test_support::ResponseSpec::bytes(200, bytes),
        ),
    ])
    .await;
    let endpoint = server.url("/rootfs");
    install(&reqwest::Client::new(), temp.path(), &endpoint, &|_, _| {})
        .await
        .unwrap();
    let _ = server.finish().await;
    assert!(is_rootfs_installed(&temp.path().join("rootfs")).unwrap());
    assert_eq!(
        fs::read_to_string(temp.path().join("rootfs/sbin/apk")).unwrap(),
        "apk"
    );
    let repositories = fs::read_to_string(temp.path().join("rootfs/etc/apk/repositories")).unwrap();
    assert!(repositories.contains(ALPINE_BRANCH));
    assert!(!temp.path().join(STAGE).exists());
    assert!(!temp.path().join(PREVIOUS).exists());
    assert!(!temp.path().join(RECEIPT).exists());
}
