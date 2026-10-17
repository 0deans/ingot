use super::verify_file_sha1;
use std::fs;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Semaphore;
fn validate_content_range(
    headers: &reqwest::header::HeaderMap,
    expected_start: u64,
    expected_end: u64,
    expected_total: u64,
) -> Result<(), String> {
    let header_val = headers.get(reqwest::header::CONTENT_RANGE).ok_or_else(|| {
        "Missing Content-Range header in 206 Partial Content response".to_string()
    })?;

    let s = header_val
        .to_str()
        .map_err(|_| "Non-ASCII Content-Range header".to_string())?;

    let bytes_part = s
        .strip_prefix("bytes ")
        .ok_or_else(|| format!("Invalid Content-Range unit in: {s}"))?;

    let (range_part, total_part) = bytes_part
        .split_once('/')
        .ok_or_else(|| format!("Malformed Content-Range (missing '/'): {s}"))?;

    let (start_str, end_str) = range_part
        .split_once('-')
        .ok_or_else(|| format!("Malformed range in Content-Range: {s}"))?;

    let start: u64 = start_str
        .parse()
        .map_err(|_| format!("Invalid start offset in Content-Range: {s}"))?;

    let end: u64 = end_str
        .parse()
        .map_err(|_| format!("Invalid end offset in Content-Range: {s}"))?;

    if start != expected_start || end != expected_end {
        return Err(format!(
            "Content-Range mismatch: requested {expected_start}-{expected_end}, got {start}-{end}"
        ));
    }

    if total_part != "*" {
        let total: u64 = total_part
            .parse()
            .map_err(|_| format!("Invalid total length in Content-Range: {s}"))?;
        if total != expected_total {
            return Err(format!(
                "Content-Range total length mismatch: expected {expected_total}, got {total}"
            ));
        }
    }

    Ok(())
}

/// Downloads a large file using HTTP Range multi-threaded chunking if supported,
/// or regular streaming if ranges are not supported or file is small.
pub async fn download_file_chunked(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    progress: Option<Arc<dyn Fn(u64, u64) + Send + Sync>>,
) -> Result<(), String> {
    download_file_chunked_with_chunk_listener(client, url, dest, expected_sha1, progress, None)
        .await
}

/// Downloads a large file using HTTP Range multi-threaded chunking if supported,
/// supporting an unthrottled chunk written hook for synchronization.
pub(super) async fn download_file_chunked_with_chunk_listener(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    progress: Option<Arc<dyn Fn(u64, u64) + Send + Sync>>,
    on_chunk_written: Option<Arc<dyn Fn(u64, u64) + Send + Sync>>,
) -> Result<(), String> {
    if dest.exists() {
        let meta = fs::metadata(dest).ok();
        let len = meta.map(|m| m.len()).unwrap_or(0);
        if len > 0 {
            if let Some(sha) = expected_sha1 {
                if verify_file_sha1(dest, sha) {
                    return Ok(());
                }
            } else {
                return Ok(());
            }
        }
    }

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create destination dir: {e}"))?;
    }

    // Probe file with HEAD request
    let head_resp = client.head(url).send().await;
    let (target_url, content_length, accept_ranges) = match head_resp {
        Ok(res) if res.status().is_success() => {
            let final_u = res.url().to_string();
            let cl = res
                .headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0);
            let ar = res
                .headers()
                .get(reqwest::header::ACCEPT_RANGES)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.to_lowercase().contains("bytes"))
                .unwrap_or(false);
            (final_u, cl, ar)
        }
        _ => (url.to_string(), 0, false),
    };

    const CHUNK_SIZE: u64 = 4 * 1024 * 1024; // 4MB chunks
    const MIN_CHUNKED_SIZE: u64 = 6 * 1024 * 1024; // 6MB threshold for chunking

    let last_report = Arc::new(std::sync::Mutex::new(std::time::Instant::now()));
    let report_throttled = {
        let cb = progress.clone();
        let lr = last_report.clone();
        move |cur: u64, tot: u64, force: bool| {
            if let Some(ref callback) = cb {
                let should_report = if force || cur == 0 || (tot > 0 && cur >= tot) {
                    true
                } else {
                    let mut lock = lr.lock().unwrap();
                    if lock.elapsed() >= std::time::Duration::from_millis(80) {
                        *lock = std::time::Instant::now();
                        true
                    } else {
                        false
                    }
                };
                if should_report {
                    callback(cur, tot);
                }
            }
        }
    };

    #[derive(Debug)]
    enum ChunkResult {
        Success,
        RangeNotSupported,
        Failed(String),
    }

    struct StagingCleanupGuard {
        path: PathBuf,
        committed: bool,
    }

    impl Drop for StagingCleanupGuard {
        fn drop(&mut self) {
            if !self.committed && self.path.exists() {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }

    let staging_uuid = uuid::Uuid::new_v4();
    let staging_dest = if let Some(ext) = dest.extension().and_then(|s| s.to_str()) {
        dest.with_extension(format!("{ext}.tmp.{staging_uuid}"))
    } else {
        dest.with_extension(format!("tmp.{staging_uuid}"))
    };

    let mut staging_guard = StagingCleanupGuard {
        path: staging_dest.clone(),
        committed: false,
    };

    let mut use_streaming = !(accept_ranges && content_length >= MIN_CHUNKED_SIZE);

    if !use_streaming {
        // Multi-threaded chunk range download
        let file = match tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&staging_dest)
            .await
        {
            Ok(f) => f,
            Err(e) => return Err(format!("Failed to create temporary destination file: {e}")),
        };
        if let Err(e) = file.set_len(content_length).await {
            let _ = tokio::fs::remove_file(&staging_dest).await;
            return Err(format!("Failed to preallocate temporary file length: {e}"));
        }

        let shared_file = Arc::new(tokio::sync::Mutex::new(file));
        let downloaded_bytes = Arc::new(AtomicU64::new(0));
        let num_chunks = content_length.div_ceil(CHUNK_SIZE) as usize;
        let semaphore = Arc::new(Semaphore::new(6)); // Max 6 concurrent chunks

        struct WorkerTaskGuard<T: Send + 'static> {
            set: tokio::task::JoinSet<T>,
            staging_path: PathBuf,
            completed: Arc<AtomicBool>,
        }

        impl<T: Send + 'static> WorkerTaskGuard<T> {
            fn new(staging_path: PathBuf) -> Self {
                Self {
                    set: tokio::task::JoinSet::new(),
                    staging_path,
                    completed: Arc::new(AtomicBool::new(false)),
                }
            }

            fn spawn<F>(&mut self, task: F)
            where
                F: std::future::Future<Output = T> + Send + 'static,
            {
                self.set.spawn(task);
            }

            fn abort_all(&mut self) {
                self.set.abort_all();
            }

            async fn shutdown(&mut self) {
                self.set.shutdown().await;
            }

            fn disarm(&mut self) {
                self.completed.store(true, Ordering::SeqCst);
            }
        }

        impl<T: Send + 'static> Drop for WorkerTaskGuard<T> {
            fn drop(&mut self) {
                if !self.completed.load(Ordering::SeqCst) {
                    self.set.abort_all();
                    let mut set = std::mem::take(&mut self.set);
                    let staging_path = self.staging_path.clone();
                    if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        handle.spawn(async move {
                            while set.join_next().await.is_some() {}
                            if staging_path.exists() {
                                let _ = tokio::fs::remove_file(&staging_path).await;
                            }
                        });
                    }
                }
            }
        }

        let mut tasks = WorkerTaskGuard::new(staging_dest.clone());

        for i in 0..num_chunks {
            let start = i as u64 * CHUNK_SIZE;
            let end = std::cmp::min(start + CHUNK_SIZE - 1, content_length - 1);
            let client = client.clone();
            let url = target_url.clone();
            let sem = semaphore.clone();
            let file_lock = shared_file.clone();
            let downloaded = downloaded_bytes.clone();
            let report = report_throttled.clone();
            let chunk_written_cb = on_chunk_written.clone();

            tasks.spawn(async move {
                let _permit = sem.acquire().await.map_err(|e| e.to_string())?;
                let range_header = format!("bytes={}-{}", start, end);
                let response = client
                    .get(&url)
                    .header(reqwest::header::RANGE, range_header)
                    .send()
                    .await
                    .map_err(|e| format!("Chunk request failed: {e}"))?;

                let status = response.status();
                if status == reqwest::StatusCode::OK {
                    // Server returned HTTP 200 OK: Range header was ignored; signal fallback
                    return Ok::<ChunkResult, String>(ChunkResult::RangeNotSupported);
                }

                if status != reqwest::StatusCode::PARTIAL_CONTENT {
                    return Ok::<ChunkResult, String>(ChunkResult::Failed(format!(
                        "Bad status code for chunk: {status}"
                    )));
                }

                if let Err(e) =
                    validate_content_range(response.headers(), start, end, content_length)
                {
                    return Ok::<ChunkResult, String>(ChunkResult::Failed(e));
                }

                let chunk_bytes = response
                    .bytes()
                    .await
                    .map_err(|e| format!("Failed to read chunk bytes: {e}"))?;

                let expected_len = end - start + 1;
                if chunk_bytes.len() as u64 != expected_len {
                    return Ok::<ChunkResult, String>(ChunkResult::Failed(format!(
                        "Chunk length mismatch: expected {expected_len} bytes, got {} bytes",
                        chunk_bytes.len()
                    )));
                }

                {
                    let mut file = file_lock.lock().await;
                    file.seek(SeekFrom::Start(start))
                        .await
                        .map_err(|e| format!("Failed to seek to chunk offset: {e}"))?;
                    file.write_all(&chunk_bytes)
                        .await
                        .map_err(|e| format!("Failed to write chunk: {e}"))?;
                }

                let total_dl = downloaded.fetch_add(chunk_bytes.len() as u64, Ordering::Relaxed)
                    + chunk_bytes.len() as u64;
                if let Some(ref hook) = chunk_written_cb {
                    hook(total_dl, content_length);
                }
                report(total_dl, content_length, false);

                Ok::<ChunkResult, String>(ChunkResult::Success)
            });
        }

        let mut fallback = false;
        let mut chunk_err = None;

        while let Some(task_res) = tasks.set.join_next().await {
            match task_res {
                Ok(Ok(ChunkResult::Success)) => {}
                Ok(Ok(ChunkResult::RangeNotSupported)) => {
                    fallback = true;
                    tasks.abort_all();
                    break;
                }
                Ok(Ok(ChunkResult::Failed(e))) => {
                    chunk_err = Some(e);
                    tasks.abort_all();
                    break;
                }
                Ok(Err(e)) => {
                    chunk_err = Some(e);
                    tasks.abort_all();
                    break;
                }
                Err(e) => {
                    if !e.is_cancelled() {
                        chunk_err = Some(format!("Chunk task panicked: {e}"));
                    }
                    tasks.abort_all();
                    break;
                }
            }
        }

        if fallback || chunk_err.is_some() {
            tasks.shutdown().await;
            tasks.disarm();
            drop(shared_file);
            let _ = tokio::fs::remove_file(&staging_dest).await;

            if fallback {
                use_streaming = true;
            } else {
                return Err(chunk_err.unwrap_or_else(|| "Unknown chunk error".into()));
            }
        } else {
            tasks.disarm();
            report_throttled(content_length, content_length, true);
            let mut file = shared_file.lock().await;
            file.flush()
                .await
                .map_err(|e| format!("Failed to flush chunked file: {e}"))?;
            file.sync_all()
                .await
                .map_err(|e| format!("Failed to sync chunked file: {e}"))?;
        }
    }

    if use_streaming {
        // Standard streaming download
        let mut response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                let _ = tokio::fs::remove_file(&staging_dest).await;
                return Err(format!("Failed to download {url}: {e}"));
            }
        };

        if !response.status().is_success() {
            let _ = tokio::fs::remove_file(&staging_dest).await;
            return Err(format!(
                "Failed to download {url}: HTTP {}",
                response.status()
            ));
        }

        let total_size = response.content_length().unwrap_or(content_length);
        let mut file = match tokio::fs::File::create(&staging_dest).await {
            Ok(f) => f,
            Err(e) => return Err(format!("Failed to create temporary destination file: {e}")),
        };

        let mut downloaded: u64 = 0;
        report_throttled(0, total_size, true);

        while let Some(chunk) = match response.chunk().await {
            Ok(c) => c,
            Err(e) => {
                let _ = tokio::fs::remove_file(&staging_dest).await;
                return Err(format!("Error streaming chunk: {e}"));
            }
        } {
            if let Err(e) = file.write_all(&chunk).await {
                let _ = tokio::fs::remove_file(&staging_dest).await;
                return Err(format!("Failed to write chunk: {e}"));
            }
            downloaded += chunk.len() as u64;
            report_throttled(downloaded, total_size, false);
        }

        report_throttled(total_size, total_size, true);
        file.flush()
            .await
            .map_err(|e| format!("Failed to flush streaming file: {e}"))?;
        file.sync_all()
            .await
            .map_err(|e| format!("Failed to sync streaming file: {e}"))?;
    }

    // Verify SHA-1 on temporary file before committing to dest
    if let Some(expected) = expected_sha1 {
        if !verify_file_sha1(&staging_dest, expected) {
            let _ = tokio::fs::remove_file(&staging_dest).await;
            return Err(format!(
                "SHA-1 verification failed for downloaded file: {}",
                dest.display()
            ));
        }
    }

    // Commit file
    tokio::fs::rename(&staging_dest, dest).await.map_err(|e| {
        let _ = fs::remove_file(&staging_dest);
        format!(
            "Failed to move completed download to {}: {e}",
            dest.display()
        )
    })?;

    staging_guard.committed = true;
    Ok(())
}
