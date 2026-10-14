use sha1::{Digest, Sha1};
use std::fs;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

pub fn is_download_task_cached(task: &DownloadTask) -> bool {
    if !task.dest.exists() {
        return false;
    }
    let meta = match fs::metadata(&task.dest) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if meta.len() == 0 {
        return false;
    }

    // 1. If size is specified, check if file length matches
    if let Some(expected_size) = task.size {
        if meta.len() != expected_size {
            return false;
        }
    }

    // 2. If sha1 is specified, verify sha1
    if let Some(ref expected_sha) = task.sha1 {
        return verify_file_sha1(&task.dest, expected_sha);
    }

    // 3. If size was specified and matched without sha1, or neither was specified,
    // existing non-empty file is considered valid
    true
}

pub fn verify_file_sha1(path: &Path, expected_sha1: &str) -> bool {
    if !path.exists() {
        return false;
    }
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(_) => return false,
    };
    let mut hasher = Sha1::new();
    hasher.update(&data);
    let hash: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    hash.eq_ignore_ascii_case(expected_sha1)
}

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
pub async fn download_file_chunked_with_chunk_listener(
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

/// Downloads multiple files concurrently (e.g. assets, libraries) with a bounded worker pool.
pub async fn download_files_concurrent(
    client: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    max_concurrency: usize,
    progress: Option<Arc<dyn Fn(usize, usize, &str) + Send + Sync>>,
) -> Result<(), String> {
    // Filter out already cached files upfront
    let needed_tasks: Vec<DownloadTask> = tasks
        .into_iter()
        .filter(|t| !is_download_task_cached(t))
        .collect();

    let total_files = needed_tasks.len();
    if total_files == 0 {
        return Ok(());
    }

    let worker_count = max_concurrency.min(total_files).max(1);
    let (tx, rx) = tokio::sync::mpsc::channel::<DownloadTask>(worker_count * 4);
    let rx = Arc::new(tokio::sync::Mutex::new(rx));

    let completed_count = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::with_capacity(worker_count);

    for _ in 0..worker_count {
        let rx = rx.clone();
        let client = client.clone();
        let completed = completed_count.clone();
        let cb = progress.clone();

        handles.push(tokio::spawn(async move {
            while let Some(task) = {
                let mut lock = rx.lock().await;
                lock.recv().await
            } {
                if let Some(parent) = task.dest.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }

                // Download with retries (up to 3 attempts)
                let mut last_error = None;
                let mut success = false;

                for attempt in 0..3 {
                    let resp = match client.get(&task.url).send().await {
                        Ok(r) => r,
                        Err(e) => {
                            last_error = Some(format!("Failed to download {}: {e}", task.url));
                            tokio::time::sleep(std::time::Duration::from_millis(
                                150 * (attempt + 1),
                            ))
                            .await;
                            continue;
                        }
                    };

                    if !resp.status().is_success() {
                        last_error = Some(format!(
                            "Download failed for {}: HTTP {}",
                            task.url,
                            resp.status()
                        ));
                        tokio::time::sleep(std::time::Duration::from_millis(150 * (attempt + 1)))
                            .await;
                        continue;
                    }

                    let bytes = match resp.bytes().await {
                        Ok(b) => b,
                        Err(e) => {
                            last_error =
                                Some(format!("Failed to read bytes for {}: {e}", task.url));
                            tokio::time::sleep(std::time::Duration::from_millis(
                                150 * (attempt + 1),
                            ))
                            .await;
                            continue;
                        }
                    };

                    // Verify size if specified
                    if let Some(expected_size) = task.size {
                        if bytes.len() as u64 != expected_size {
                            last_error = Some(format!(
                                "Size mismatch for {}: expected {}, got {}",
                                task.url,
                                expected_size,
                                bytes.len()
                            ));
                            continue;
                        }
                    }

                    // Verify sha1 in memory before writing
                    if let Some(ref expected) = task.sha1 {
                        let mut hasher = Sha1::new();
                        hasher.update(&bytes);
                        let hash: String = hasher
                            .finalize()
                            .iter()
                            .map(|b| format!("{:02x}", b))
                            .collect();
                        if !hash.eq_ignore_ascii_case(expected) {
                            last_error = Some(format!(
                                "Hash mismatch for {}: expected {}, got {}",
                                task.url, expected, hash
                            ));
                            continue;
                        }
                    }

                    if let Err(e) = tokio::fs::write(&task.dest, &bytes).await {
                        return Err(format!("Failed to write {}: {e}", task.dest.display()));
                    }

                    success = true;
                    break;
                }

                if !success {
                    return Err(
                        last_error.unwrap_or_else(|| format!("Download failed for {}", task.url))
                    );
                }

                let done = completed.fetch_add(1, Ordering::Relaxed) + 1;
                if let Some(ref callback) = cb {
                    let filename = task
                        .dest
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("file");
                    callback(done, total_files, filename);
                }
            }

            Ok::<(), String>(())
        }));
    }

    // Send all tasks to workers
    for task in needed_tasks {
        if tx.send(task).await.is_err() {
            break;
        }
    }
    drop(tx);

    for handle in handles {
        match handle.await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(format!("Download worker panicked: {e}")),
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    struct DownloadMockServer {
        base_url: String,
        _handle: tokio::task::JoinHandle<()>,
    }

    impl DownloadMockServer {
        async fn start<F>(handler: F) -> Self
        where
            F: Fn(&str, &str, &str) -> (u16, Vec<(String, String)>, Vec<u8>)
                + Send
                + Sync
                + 'static,
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
                    let resp =
                        "HTTP/1.1 200 OK\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n";
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
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
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
}
