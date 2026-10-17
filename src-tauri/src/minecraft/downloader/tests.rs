use super::*;
use sha1::{Digest, Sha1};
use std::fs;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

async fn unavailable_provider(requests: usize) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        for _ in 0..requests {
            let (mut connection, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            while !request.ends_with(b"\r\n\r\n") {
                assert!(request.len() < 8192, "Request headers exceed fixture limit");
                request.push(connection.read_u8().await.unwrap());
            }
            connection
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                .await
                .unwrap();
        }
    });
    (
        format!("http://{address}/private-artifact?token=download-secret"),
        server,
    )
}

fn assert_download_diagnostics_are_redacted(error: DownloadError) {
    for diagnostic in [error.to_string(), format!("{error:?}")] {
        assert!(!diagnostic.contains("download-secret"));
        assert!(!diagnostic.contains("private-artifact"));
    }
    let boundary = crate::ipc::error::ApiError::from(error);
    assert!(matches!(
        boundary,
        crate::ipc::error::ApiError::ProviderUnavailable { .. }
    ));
    let payload = serde_json::to_string(&boundary).unwrap();
    assert!(!payload.contains("download-secret"));
    assert!(!payload.contains("private-artifact"));
}

#[tokio::test]
async fn batch_download_preserves_transport_cause_without_signed_url() {
    let directory = tempfile::tempdir().unwrap();
    let (url, server) = unavailable_provider(1).await;
    let task = DownloadTask {
        url,
        dest: directory.path().join("artifact.jar"),
        sha1: None,
        size: None,
    };
    let error = download_files_concurrent(&reqwest::Client::new(), vec![task], 1, None)
        .await
        .unwrap_err();
    let source = std::error::Error::source(&error)
        .unwrap()
        .downcast_ref::<reqwest::Error>()
        .unwrap();
    assert_eq!(
        source.status(),
        Some(reqwest::StatusCode::SERVICE_UNAVAILABLE)
    );
    assert!(source.url().is_none());
    assert_download_diagnostics_are_redacted(error);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    server.await.unwrap();
}

#[tokio::test]
async fn single_download_status_diagnostics_exclude_signed_url() {
    let directory = tempfile::tempdir().unwrap();
    let (url, server) = unavailable_provider(2).await;
    let error = download_file_chunked(
        &reqwest::Client::new(),
        &url,
        &directory.path().join("artifact.jar"),
        None,
        None,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        DownloadError::HttpStatus {
            status: reqwest::StatusCode::SERVICE_UNAVAILABLE
        }
    ));
    assert_download_diagnostics_are_redacted(error);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    server.await.unwrap();
}

#[test]
fn cache_detection_with_matching_size_and_mismatched_sha1_returns_false() {
    let temp = tempfile::tempdir().unwrap();
    let file_path = temp.path().join("corrupted.jar");
    fs::write(&file_path, b"12345").unwrap(); // length 5

    let task = DownloadTask {
        url: "https://example.com/test.jar".into(),
        dest: file_path,
        size: Some(5),
        sha1: Some("0000000000000000000000000000000000000000".into()),
    };

    // Previously returned true because size matched! Now correctly returns false:
    assert!(!is_download_task_cached(&task));
}

#[test]
fn cache_detection_with_matching_size_and_matching_sha1_returns_true() {
    let temp = tempfile::tempdir().unwrap();
    let file_path = temp.path().join("valid.jar");
    let content = b"hello world";
    fs::write(&file_path, content).unwrap();

    let mut hasher = Sha1::new();
    hasher.update(content);
    let valid_sha1: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();

    let task = DownloadTask {
        url: "https://example.com/valid.jar".into(),
        dest: file_path,
        size: Some(content.len() as u64),
        sha1: Some(valid_sha1),
    };

    assert!(is_download_task_cached(&task));
}

#[test]
fn cache_detection_with_mismatched_size_returns_false() {
    let temp = tempfile::tempdir().unwrap();
    let file_path = temp.path().join("test.jar");
    fs::write(&file_path, b"abc").unwrap(); // length 3

    let task = DownloadTask {
        url: "https://example.com/test.jar".into(),
        dest: file_path,
        size: Some(10),
        sha1: None,
    };

    assert!(!is_download_task_cached(&task));
}

#[test]
fn cache_detection_nonexistent_or_empty_returns_false() {
    let temp = tempfile::tempdir().unwrap();
    let missing_path = temp.path().join("missing.jar");
    let empty_path = temp.path().join("empty.jar");
    fs::write(&empty_path, b"").unwrap();

    let task_missing = DownloadTask {
        url: "https://example.com/missing.jar".into(),
        dest: missing_path,
        size: None,
        sha1: None,
    };
    assert!(!is_download_task_cached(&task_missing));

    let task_empty = DownloadTask {
        url: "https://example.com/empty.jar".into(),
        dest: empty_path,
        size: None,
        sha1: None,
    };
    assert!(!is_download_task_cached(&task_empty));
}

#[test]
fn verify_sha1_works() {
    let temp = tempfile::tempdir().unwrap();
    let file_path = temp.path().join("check.txt");
    fs::write(&file_path, b"abc").unwrap();

    // sha1 of "abc" is a9993e364706816aba3e25717850c26c9cd0d89d
    assert!(verify_file_sha1(
        &file_path,
        "a9993e364706816aba3e25717850c26c9cd0d89d"
    ));
    assert!(verify_file_sha1(
        &file_path,
        "A9993E364706816ABA3E25717850C26C9CD0D89D"
    ));
    assert!(!verify_file_sha1(&file_path, "wrong"));
}

struct DownloadMockServer {
    base_url: String,
    _handle: tokio::task::JoinHandle<()>,
}

impl DownloadMockServer {
    async fn start<F>(handler: F) -> Self
    where
        F: Fn(&str, &str, &str) -> (u16, Vec<(String, String)>, Vec<u8>) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{addr}");
        let handler = Arc::new(handler);

        let handle = tokio::spawn(async move {
            loop {
                let Ok((mut stream, _)) = listener.accept().await else {
                    break;
                };
                let handler = Arc::clone(&handler);

                tokio::spawn(async move {
                    let mut buffer = [0u8; 4096];
                    let count = match stream.read(&mut buffer).await {
                        Ok(c) if c > 0 => c,
                        _ => return,
                    };
                    let req_str = String::from_utf8_lossy(&buffer[..count]).to_string();
                    let mut lines = req_str.lines();
                    let req_line = lines.next().unwrap_or("");
                    let mut parts = req_line.split_whitespace();
                    let method = parts.next().unwrap_or("GET");
                    let path = parts.next().unwrap_or("/");

                    let (status, headers, body) = handler(method, path, &req_str);
                    let mut resp_str = format!("HTTP/1.1 {status} OK\r\nConnection: close\r\n");
                    for (k, v) in headers {
                        resp_str.push_str(&format!("{k}: {v}\r\n"));
                    }
                    resp_str.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));

                    let _ = stream.write_all(resp_str.as_bytes()).await;
                    if !body.is_empty() {
                        let _ = stream.write_all(&body).await;
                    }
                    let _ = stream.shutdown().await;
                });
            }
        });

        Self {
            base_url,
            _handle: handle,
        }
    }
}

#[tokio::test]
async fn test_download_file_chunked_falls_back_when_range_ignored() {
    let test_data = vec![b'A'; 7 * 1024 * 1024];
    let test_data_clone = test_data.clone();
    let server = DownloadMockServer::start(move |method, _path, _req| {
        if method == "HEAD" {
            (
                200,
                vec![
                    ("Accept-Ranges".into(), "bytes".into()),
                    ("Content-Length".into(), test_data_clone.len().to_string()),
                ],
                vec![],
            )
        } else {
            (
                200,
                vec![("Content-Type".into(), "application/octet-stream".into())],
                test_data_clone.clone(),
            )
        }
    })
    .await;

    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("test_fallback.bin");
    let client = reqwest::Client::new();

    let res = download_file_chunked(
        &client,
        &format!("{}/test_fallback.bin", server.base_url),
        &dest,
        None,
        None,
    )
    .await;

    assert!(res.is_ok(), "download failed: {:?}", res);
    assert!(dest.exists());
    let read_bytes = fs::read(&dest).unwrap();
    assert_eq!(read_bytes.len(), test_data.len());
    assert_eq!(read_bytes, test_data);
}

#[tokio::test]
async fn test_download_file_chunked_fails_and_cleans_up_on_mismatched_range_chunk() {
    let server = DownloadMockServer::start(move |method, _path, _req| {
        if method == "HEAD" {
            (
                200,
                vec![
                    ("Accept-Ranges".into(), "bytes".into()),
                    ("Content-Length".into(), (7 * 1024 * 1024).to_string()),
                ],
                vec![],
            )
        } else {
            (
                206,
                vec![("Content-Range".into(), "bytes 0-4194303/7340032".into())],
                vec![1, 2, 3, 4, 5],
            )
        }
    })
    .await;

    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("test_mismatch.bin");
    let client = reqwest::Client::new();

    let res = download_file_chunked(
        &client,
        &format!("{}/test_mismatch.bin", server.base_url),
        &dest,
        None,
        None,
    )
    .await;

    assert!(res.is_err());
    assert!(!dest.exists());

    let remaining_files: Vec<_> = fs::read_dir(temp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        remaining_files.is_empty(),
        "Expected no staging files left, found: {:?}",
        remaining_files
    );
}

#[tokio::test]
async fn test_download_file_chunked_dropped_future_cleans_up_staging() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_url = format!("http://{addr}");

    let _server_handle = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let mut buffer = [0u8; 1024];
            let _ = stream.read(&mut buffer).await;
            let req_str = String::from_utf8_lossy(&buffer);
            if req_str.starts_with("HEAD") {
                let resp = "HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n";
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            } else {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            }
        }
    });

    let temp = tempfile::tempdir().unwrap();
    let dest_path = temp.path().join("dropped.bin");
    let client = reqwest::Client::new();

    let file_url = format!("{server_url}/dropped.bin");
    let download_future = download_file_chunked(&client, &file_url, &dest_path, None, None);

    let _ = tokio::time::timeout(std::time::Duration::from_millis(50), download_future).await;

    let files: Vec<_> = fs::read_dir(temp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    assert!(
        files.is_empty(),
        "Expected no staging files to leak after drop, found: {:?}",
        files
    );
}

#[tokio::test]
async fn test_download_file_chunked_concurrent_isolation() {
    let server = DownloadMockServer::start(move |_method, _path, _req| {
        (
            200,
            vec![("Content-Type".into(), "application/octet-stream".into())],
            b"concurrent content".to_vec(),
        )
    })
    .await;

    let temp = tempfile::tempdir().unwrap();
    let dest_path = temp.path().join("concurrent.bin");
    let client = reqwest::Client::new();
    let url1 = format!("{}/1", server.base_url);
    let url2 = format!("{}/2", server.base_url);

    let dl1 = download_file_chunked(&client, &url1, &dest_path, None, None);
    let dl2 = download_file_chunked(&client, &url2, &dest_path, None, None);

    let (res1, res2) = tokio::join!(dl1, dl2);
    assert!(res1.is_ok());
    assert!(res2.is_ok());
    assert!(dest_path.exists());
    assert_eq!(fs::read(&dest_path).unwrap(), b"concurrent content");
}

#[tokio::test]
async fn test_download_file_chunked_fails_on_missing_content_range() {
    let server = DownloadMockServer::start(move |method, _path, _req| {
        if method == "HEAD" {
            (
                200,
                vec![
                    ("Accept-Ranges".into(), "bytes".into()),
                    ("Content-Length".into(), (7 * 1024 * 1024).to_string()),
                ],
                vec![],
            )
        } else {
            // 206 Partial Content but missing Content-Range header
            (206, vec![], vec![1; 4 * 1024 * 1024])
        }
    })
    .await;

    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("test_missing_header.bin");
    let client = reqwest::Client::new();

    let res = download_file_chunked(
        &client,
        &format!("{}/test_missing_header.bin", server.base_url),
        &dest,
        None,
        None,
    )
    .await;

    assert!(res.is_err());
    assert!(!dest.exists());
    let remaining: Vec<_> = fs::read_dir(temp.path()).unwrap().collect();
    assert!(
        remaining.is_empty(),
        "Staging file must be cleaned up on missing Content-Range"
    );
}

#[tokio::test]
async fn test_download_file_chunked_fails_on_malformed_content_range() {
    let server = DownloadMockServer::start(move |method, _path, _req| {
        if method == "HEAD" {
            (
                200,
                vec![
                    ("Accept-Ranges".into(), "bytes".into()),
                    ("Content-Length".into(), (7 * 1024 * 1024).to_string()),
                ],
                vec![],
            )
        } else {
            // Malformed Content-Range header
            (
                206,
                vec![("Content-Range".into(), "bytes invalid-range".into())],
                vec![1; 4 * 1024 * 1024],
            )
        }
    })
    .await;

    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("test_malformed_header.bin");
    let client = reqwest::Client::new();

    let res = download_file_chunked(
        &client,
        &format!("{}/test_malformed_header.bin", server.base_url),
        &dest,
        None,
        None,
    )
    .await;

    assert!(res.is_err());
    assert!(!dest.exists());
    let remaining: Vec<_> = fs::read_dir(temp.path()).unwrap().collect();
    assert!(
        remaining.is_empty(),
        "Staging file must be cleaned up on malformed Content-Range"
    );
}

#[tokio::test]
async fn test_download_file_chunked_fails_on_mismatched_total_content_range() {
    let server = DownloadMockServer::start(move |method, _path, _req| {
        if method == "HEAD" {
            (
                200,
                vec![
                    ("Accept-Ranges".into(), "bytes".into()),
                    ("Content-Length".into(), (7 * 1024 * 1024).to_string()),
                ],
                vec![],
            )
        } else {
            // Total length mismatch: file is 7340032 but header says 9999999
            (
                206,
                vec![("Content-Range".into(), "bytes 0-4194303/9999999".into())],
                vec![1; 4 * 1024 * 1024],
            )
        }
    })
    .await;

    let temp = tempfile::tempdir().unwrap();
    let dest = temp.path().join("test_wrong_total.bin");
    let client = reqwest::Client::new();

    let res = download_file_chunked(
        &client,
        &format!("{}/test_wrong_total.bin", server.base_url),
        &dest,
        None,
        None,
    )
    .await;

    assert!(res.is_err());
    assert!(!dest.exists());
    let remaining: Vec<_> = fs::read_dir(temp.path()).unwrap().collect();
    assert!(
        remaining.is_empty(),
        "Staging file must be cleaned up on total mismatch"
    );
}

#[tokio::test]
async fn test_download_file_chunked_dropped_during_write_aborts_workers_and_cleans_up() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_url = format!("http://{addr}");
    let total_len: u64 = 8 * 1024 * 1024;

    let active_workers = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let active_workers_clone = active_workers.clone();

    let server_handle = tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let workers = active_workers_clone.clone();
            tokio::spawn(async move {
                let mut buffer = [0u8; 2048];
                let mut count = 0;
                while count < buffer.len() {
                    let n = stream.read(&mut buffer[count..]).await.unwrap_or(0);
                    if n == 0 {
                        break;
                    }
                    count += n;
                    if buffer[..count].windows(4).any(|w| w == b"\r\n\r\n") {
                        break;
                    }
                }
                let req_str = String::from_utf8_lossy(&buffer[..count]);
                if req_str.starts_with("HEAD") {
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nAccept-Ranges: bytes\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        total_len
                    );
                    let _ = stream.write_all(resp.as_bytes()).await;
                    let _ = stream.shutdown().await;
                } else if req_str.starts_with("GET") {
                    let mut range = None;
                    for line in req_str.lines() {
                        let lower = line.to_lowercase();
                        if lower.starts_with("range:") {
                            if let Some(range_val) = lower.strip_prefix("range:") {
                                let trimmed = range_val.trim();
                                if let Some(spec) = trimmed.strip_prefix("bytes=") {
                                    let parts: Vec<&str> = spec.split('-').collect();
                                    if parts.len() == 2 {
                                        if let (Ok(s), Ok(e)) =
                                            (parts[0].parse::<u64>(), parts[1].parse::<u64>())
                                        {
                                            range = Some((s, e));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if let Some((start, end)) = range {
                        let chunk_len = end - start + 1;
                        let resp_hdr = format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {start}-{end}/{total_len}\r\nContent-Length: {chunk_len}\r\nConnection: close\r\n\r\n"
                        );
                        let _ = stream.write_all(resp_hdr.as_bytes()).await;

                        if start == 0 {
                            // Chunk 0: Send full chunk bytes so worker completes and writes to disk
                            let chunk_data = vec![0x42u8; chunk_len as usize];
                            let _ = stream.write_all(&chunk_data).await;
                            let _ = stream.shutdown().await;
                        } else {
                            workers.fetch_add(1, Ordering::SeqCst);
                            // Subsequent chunk: send partial data and keep stream open to keep worker active
                            let partial = vec![0x43u8; 1024];
                            let _ = stream.write_all(&partial).await;
                            let mut sink = [0u8; 128];
                            let _ = stream.read(&mut sink).await;
                            workers.fetch_sub(1, Ordering::SeqCst);
                        }
                    }
                }
            });
        }
    });

    let (client_written_tx, client_written_rx) = tokio::sync::oneshot::channel::<()>();
    let client_written_tx = std::sync::Arc::new(std::sync::Mutex::new(Some(client_written_tx)));

    let on_chunk_written = std::sync::Arc::new(move |cur: u64, _tot: u64| {
        // Fired directly and unthrottled on the client side after Chunk 0 (4MB) has been written to disk
        if cur >= 4 * 1024 * 1024 {
            if let Ok(mut lock) = client_written_tx.lock() {
                if let Some(tx) = lock.take() {
                    let _ = tx.send(());
                }
            }
        }
    });

    let temp = tempfile::tempdir().unwrap();
    let dest_path = temp.path().join("aborted_chunked.bin");
    let client = reqwest::Client::new();
    let file_url = format!("{server_url}/aborted_chunked.bin");

    let download_future = download_file_chunked_with_chunk_listener(
        &client,
        &file_url,
        &dest_path,
        None,
        None,
        Some(on_chunk_written),
    );

    // Run download until client confirms disk writing started, then drop the future immediately
    tokio::select! {
        _ = download_future => {
            panic!("Download completed unexpectedly");
        }
        _ = client_written_rx => {
            // Future is dropped right here while chunk 0 was written to disk and chunk 1 worker is still running
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
            panic!("Timed out waiting for chunk disk write signal");
        }
    }

    // Verify that background cleanup awaits aborted workers and removes the staging file
    let start = std::time::Instant::now();
    let mut cleaned = false;
    while start.elapsed() < std::time::Duration::from_secs(3) {
        let files: Vec<_> = fs::read_dir(temp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        if files.is_empty() && active_workers.load(Ordering::SeqCst) == 0 {
            cleaned = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    assert!(
        cleaned,
        "Expected staging file to be cleaned up and workers terminated, found files: {:?}",
        fs::read_dir(temp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        active_workers.load(Ordering::SeqCst),
        0,
        "All active worker tasks must be terminated"
    );
    assert!(!dest_path.exists(), "Destination file must not exist");

    server_handle.abort();
}
