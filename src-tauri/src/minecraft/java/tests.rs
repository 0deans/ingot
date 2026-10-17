use super::discovery::{executable_name, find_binary, parse_version, JavaProbe};
use super::extract::extract_runtime;
use super::install::{ArchiveFormat, RuntimeRequest};
use super::{ensure_java_runtime, get_required_java_version, JavaError};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct FixedProbe(u32);
impl JavaProbe for FixedProbe {
    fn major_version(&self, _: &Path) -> Result<u32, JavaError> {
        Ok(self.0)
    }
}

fn request(root: &Path) -> RuntimeRequest {
    RuntimeRequest {
        data_dir: root.to_path_buf(),
        major_version: 21,
        custom_path: None,
        system_roots: Vec::new(),
        download_url: "http://127.0.0.1:1/not-requested".into(),
        archive_format: ArchiveFormat::Zip,
    }
}

fn write_binary(root: &Path) -> PathBuf {
    let path = root.join("bin").join(executable_name());
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, b"fake Java binary").unwrap();
    path
}

fn zip_bytes(path: &str) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    zip.start_file(path, zip::write::SimpleFileOptions::default())
        .unwrap();
    zip.write_all(b"binary").unwrap();
    zip.finish().unwrap().into_inner()
}

async fn serve(body: Vec<u8>, status: &str) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/runtime", listener.local_addr().unwrap());
    let status = status.to_owned();
    let task = tokio::spawn(async move {
        let (mut connection, _) = listener.accept().await.unwrap();
        let mut buffer = [0; 4096];
        let _count = connection.read(&mut buffer).await.unwrap();
        let header = format!(
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        connection.write_all(header.as_bytes()).await.unwrap();
        connection.write_all(&body).await.unwrap();
    });
    (url, task)
}

#[test]
fn java_version_rules_cover_metadata_legacy_and_snapshot_versions() {
    for (version, metadata, expected) in [
        ("1.16.5", None, 8),
        ("1.17.1", None, 16),
        ("1.20.4", None, 17),
        ("1.20.5", None, 21),
        ("26w04a", None, 25),
        ("unknown", Some(17), 17),
    ] {
        assert_eq!(get_required_java_version(version, metadata), expected);
    }
    for (text, expected) in [
        ("java version \"1.8.0_351\"", Some(8)),
        ("openjdk version \"21.0.6\" 2025-01-21", Some(21)),
        ("openjdk version \"25-ea\"", Some(25)),
        ("version \"0.0\"", None),
        ("not a runtime", None),
    ] {
        assert_eq!(parse_version(text), expected);
    }
}

#[test]
fn discovery_distinguishes_absence_from_unreadable_directory() {
    let temp = tempfile::tempdir().unwrap();
    assert!(find_binary(&temp.path().join("missing")).unwrap().is_none());
    let file = temp.path().join("file");
    std::fs::write(&file, b"not a directory").unwrap();
    assert!(matches!(find_binary(&file), Err(JavaError::Io(_))));
    let nested = temp.path().join("jdk-21");
    let binary = write_binary(&nested);
    assert_eq!(find_binary(temp.path()).unwrap(), Some(binary));
}

#[tokio::test]
async fn cached_runtime_is_validated_without_network_or_installed_java() {
    let temp = tempfile::tempdir().unwrap();
    let binary = write_binary(&temp.path().join("runtimes/java-21"));
    let result = ensure_java_runtime(
        request(temp.path()),
        &reqwest::Client::new(),
        Arc::new(FixedProbe(21)),
        None,
    )
    .await
    .unwrap();
    assert_eq!(result, binary);
    let result = ensure_java_runtime(
        request(temp.path()),
        &reqwest::Client::new(),
        Arc::new(FixedProbe(17)),
        None,
    )
    .await;
    assert!(matches!(
        result,
        Err(JavaError::VersionMismatch {
            required: 21,
            actual: 17
        })
    ));
    assert!(binary.exists());
}

#[tokio::test]
async fn invalid_explicit_java_path_is_not_silently_replaced() {
    let temp = tempfile::tempdir().unwrap();
    let mut request = request(temp.path());
    request.custom_path = Some(temp.path().join("missing-custom-runtime"));
    let result = ensure_java_runtime(
        request,
        &reqwest::Client::new(),
        Arc::new(FixedProbe(21)),
        None,
    )
    .await;
    assert!(matches!(result, Err(JavaError::MissingBinary)));
    assert!(!temp.path().join("runtimes").exists());
}

#[tokio::test]
async fn runtime_download_publishes_only_after_archive_and_version_validation() {
    let temp = tempfile::tempdir().unwrap();
    let (url, server) = serve(
        zip_bytes(&format!("jdk-21/bin/{}", executable_name())),
        "200 OK",
    )
    .await;
    let mut request = request(temp.path());
    request.download_url = url;
    let binary = ensure_java_runtime(
        request,
        &reqwest::Client::new(),
        Arc::new(FixedProbe(21)),
        None,
    )
    .await
    .unwrap();
    server.await.unwrap();
    assert_eq!(std::fs::read(binary).unwrap(), b"binary");
    assert_eq!(
        std::fs::read_dir(temp.path().join("runtimes"))
            .unwrap()
            .count(),
        1
    );
}

#[tokio::test]
async fn rejected_download_preserves_existing_incomplete_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let old = temp.path().join("runtimes/java-21");
    std::fs::create_dir_all(&old).unwrap();
    std::fs::write(old.join("recovery-data"), b"keep").unwrap();
    let (url, server) = serve(zip_bytes("../escaped"), "200 OK").await;
    let mut request = request(temp.path());
    request.download_url = url;
    let result = ensure_java_runtime(
        request,
        &reqwest::Client::new(),
        Arc::new(FixedProbe(21)),
        None,
    )
    .await;
    server.await.unwrap();
    assert!(matches!(result, Err(JavaError::UnsafeArchive)));
    assert_eq!(std::fs::read(old.join("recovery-data")).unwrap(), b"keep");
    assert!(!temp.path().join("escaped").exists());
    assert_eq!(
        std::fs::read_dir(temp.path().join("runtimes"))
            .unwrap()
            .count(),
        1
    );
}

#[tokio::test]
async fn http_failure_retains_typed_cause_and_never_publishes() {
    let temp = tempfile::tempdir().unwrap();
    let (url, server) = serve(Vec::new(), "503 Service Unavailable").await;
    let mut request = request(temp.path());
    request.download_url = url;
    let result = ensure_java_runtime(
        request,
        &reqwest::Client::new(),
        Arc::new(FixedProbe(21)),
        None,
    )
    .await;
    server.await.unwrap();
    assert!(
        matches!(result, Err(JavaError::Http(error)) if error.status() == Some(reqwest::StatusCode::SERVICE_UNAVAILABLE))
    );
    assert!(!temp.path().join("runtimes/java-21").exists());
}

#[test]
fn tar_runtime_extraction_preserves_regular_files_and_permissions() {
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("runtime.tar.gz");
    let file = std::fs::File::create(&archive_path).unwrap();
    let gzip = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(gzip);
    let mut header = tar::Header::new_gnu();
    header.set_size(6);
    header.set_mode(0o755);
    header.set_cksum();
    archive
        .append_data(
            &mut header,
            format!("jdk/bin/{}", executable_name()),
            &b"binary"[..],
        )
        .unwrap();
    let gzip = archive.into_inner().unwrap();
    gzip.finish().unwrap();
    let output = temp.path().join("output");
    std::fs::create_dir(&output).unwrap();
    extract_runtime(&archive_path, &output, ArchiveFormat::TarGzip).unwrap();
    assert_eq!(
        std::fs::read(find_binary(&output).unwrap().unwrap()).unwrap(),
        b"binary"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(find_binary(&output).unwrap().unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }
}

fn tar_with_links(path: &Path, links: &[(&str, &str, tar::EntryType)]) {
    let file = std::fs::File::create(path).unwrap();
    let gzip = flate2::write::GzEncoder::new(file, flate2::Compression::default());
    let mut archive = tar::Builder::new(gzip);
    for (name, target, kind) in links {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(*kind);
        header.set_size(0);
        header.set_mode(0o777);
        archive.append_link(&mut header, name, target).unwrap();
    }
    let mut header = tar::Header::new_gnu();
    header.set_size(6);
    header.set_mode(0o755);
    header.set_cksum();
    archive
        .append_data(&mut header, "jdk/lib/library", &b"binary"[..])
        .unwrap();
    archive.into_inner().unwrap().finish().unwrap();
}

#[test]
fn contained_jdk_file_links_are_materialized_after_their_targets() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("archive");
    tar_with_links(
        &archive,
        &[
            ("jdk/bin/alias", "../lib/library", tar::EntryType::Symlink),
            ("jdk/lib/hard-alias", "jdk/bin/alias", tar::EntryType::Link),
        ],
    );
    let output = temp.path().join("output");
    std::fs::create_dir(&output).unwrap();
    extract_runtime(&archive, &output, ArchiveFormat::TarGzip).unwrap();
    for name in ["jdk/bin/alias", "jdk/lib/hard-alias"] {
        let path = output.join(name);
        assert_eq!(std::fs::read(&path).unwrap(), b"binary");
        assert!(std::fs::symlink_metadata(path).unwrap().is_file());
    }
}

#[test]
fn runtime_links_cannot_escape_cycle_or_alias_directories() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("archive");
    for (index, links) in [
        vec![("jdk/escape", "../../outside", tar::EntryType::Symlink)],
        vec![("jdk/escape", "/outside", tar::EntryType::Symlink)],
        vec![("jdk/escape", "C:\\outside", tar::EntryType::Symlink)],
        vec![
            ("jdk/a", "b", tar::EntryType::Symlink),
            ("jdk/b", "a", tar::EntryType::Symlink),
        ],
        vec![("jdk/directory", "lib", tar::EntryType::Symlink)],
    ]
    .into_iter()
    .enumerate()
    {
        tar_with_links(&archive, &links);
        let output = temp.path().join(format!("output-{index}"));
        std::fs::create_dir(&output).unwrap();
        assert!(matches!(
            extract_runtime(&archive, &output, ArchiveFormat::TarGzip),
            Err(JavaError::UnsafeArchive)
        ));
    }
    assert!(!temp.path().join("outside").exists());
}

#[tokio::test]
async fn installed_candidates_are_checked_until_a_matching_major_is_found() {
    struct CandidateProbe;
    impl JavaProbe for CandidateProbe {
        fn major_version(&self, path: &Path) -> Result<u32, JavaError> {
            if path.components().any(|part| part.as_os_str() == "jdk-17") {
                return Ok(17);
            }
            Ok(21)
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let installations = temp.path().join("installed");
    write_binary(&installations.join("jdk-17"));
    let matching = write_binary(&installations.join("jdk-21"));
    let mut request = request(temp.path());
    request.system_roots.push(installations);
    let actual = ensure_java_runtime(
        request,
        &reqwest::Client::new(),
        Arc::new(CandidateProbe),
        None,
    )
    .await
    .unwrap();
    assert_eq!(actual, matching);
}

#[tokio::test]
async fn cancelling_during_runtime_validation_never_publishes() {
    struct ControlledProbe {
        entered: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
        release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
        finished: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    }
    impl JavaProbe for ControlledProbe {
        fn major_version(&self, _: &Path) -> Result<u32, JavaError> {
            self.entered
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(())
                .unwrap();
            self.release.lock().unwrap().recv().unwrap();
            self.finished
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(())
                .unwrap();
            Ok(21)
        }
    }
    let temp = tempfile::tempdir().unwrap();
    let (url, server) = serve(
        zip_bytes(&format!("jdk/bin/{}", executable_name())),
        "200 OK",
    )
    .await;
    let mut request = request(temp.path());
    request.download_url = url;
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let (finished_tx, finished_rx) = tokio::sync::oneshot::channel();
    let probe = Arc::new(ControlledProbe {
        entered: std::sync::Mutex::new(Some(entered_tx)),
        release: std::sync::Mutex::new(release_rx),
        finished: std::sync::Mutex::new(Some(finished_tx)),
    });
    let installation = tokio::spawn(async move {
        ensure_java_runtime(request, &reqwest::Client::new(), probe, None).await
    });
    entered_rx.await.unwrap();
    installation.abort();
    assert!(installation.await.unwrap_err().is_cancelled());
    release_tx.send(()).unwrap();
    finished_rx.await.unwrap();
    server.await.unwrap();
    assert!(!temp.path().join("runtimes/java-21").exists());
}
