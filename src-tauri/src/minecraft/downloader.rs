use sha1::{Digest, Sha1};
use std::fs;
use std::io::SeekFrom;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
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
        return meta.len() == expected_size;
    }

    // 2. If sha1 is specified, verify sha1
    if let Some(ref expected_sha) = task.sha1 {
        return verify_file_sha1(&task.dest, expected_sha);
    }

    // 3. If neither is specified (e.g. Maven release jar), existing non-empty file is considered valid
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
    let hash: String = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect();
    hash.eq_ignore_ascii_case(expected_sha1)
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
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create destination dir: {e}"))?;
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

    if accept_ranges && content_length >= MIN_CHUNKED_SIZE {
        // Multi-threaded chunk range download
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(dest)
            .await
            .map_err(|e| format!("Failed to create destination file: {e}"))?;
        file.set_len(content_length)
            .await
            .map_err(|e| format!("Failed to preallocate file length: {e}"))?;

        let shared_file = Arc::new(tokio::sync::Mutex::new(file));
        let downloaded_bytes = Arc::new(AtomicU64::new(0));
        let num_chunks = ((content_length + CHUNK_SIZE - 1) / CHUNK_SIZE) as usize;
        let semaphore = Arc::new(Semaphore::new(6)); // Max 6 concurrent chunks

        let mut tasks = Vec::with_capacity(num_chunks);

        for i in 0..num_chunks {
            let start = i as u64 * CHUNK_SIZE;
            let end = std::cmp::min(start + CHUNK_SIZE - 1, content_length - 1);
            let client = client.clone();
            let url = target_url.clone();
            let sem = semaphore.clone();
            let file_lock = shared_file.clone();
            let downloaded = downloaded_bytes.clone();
            let report = report_throttled.clone();

            tasks.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.map_err(|e| e.to_string())?;
                let range_header = format!("bytes={}-{}", start, end);
                let response = client
                    .get(&url)
                    .header(reqwest::header::RANGE, range_header)
                    .send()
                    .await
                    .map_err(|e| format!("Chunk request failed: {e}"))?;

                if !response.status().is_success() && response.status() != reqwest::StatusCode::PARTIAL_CONTENT {
                    return Err(format!("Bad status code for chunk: {}", response.status()));
                }

                let chunk_bytes = response
                    .bytes()
                    .await
                    .map_err(|e| format!("Failed to read chunk bytes: {e}"))?;

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
                report(total_dl, content_length, false);

                Ok::<(), String>(())
            }));
        }

        for task in tasks {
            match task.await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => return Err(e),
                Err(e) => return Err(format!("Chunk download task panicked: {e}")),
            }
        }

        report_throttled(content_length, content_length, true);

        // Flush file
        let mut file = shared_file.lock().await;
        let _ = file.flush().await;
    } else {
        // Standard streaming download
        let mut response = client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to download {url}: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "Failed to download {url}: HTTP {}",
                response.status()
            ));
        }

        let total_size = response.content_length().unwrap_or(content_length);
        let mut file = tokio::fs::File::create(dest)
            .await
            .map_err(|e| format!("Failed to create destination file: {e}"))?;

        let mut downloaded: u64 = 0;
        report_throttled(0, total_size, true);

        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|e| format!("Error streaming chunk: {e}"))?
        {
            file.write_all(&chunk)
                .await
                .map_err(|e| format!("Failed to write chunk: {e}"))?;
            downloaded += chunk.len() as u64;
            report_throttled(downloaded, total_size, false);
        }

        report_throttled(total_size, total_size, true);
        let _ = file.flush().await;
    }

    // Verify SHA-1
    if let Some(expected) = expected_sha1 {
        if !verify_file_sha1(dest, expected) {
            let _ = fs::remove_file(dest);
            return Err(format!(
                "SHA-1 verification failed for downloaded file: {}",
                dest.display()
            ));
        }
    }

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
                            tokio::time::sleep(std::time::Duration::from_millis(150 * (attempt + 1))).await;
                            continue;
                        }
                    };

                    if !resp.status().is_success() {
                        last_error = Some(format!("Download failed for {}: HTTP {}", task.url, resp.status()));
                        tokio::time::sleep(std::time::Duration::from_millis(150 * (attempt + 1))).await;
                        continue;
                    }

                    let bytes = match resp.bytes().await {
                        Ok(b) => b,
                        Err(e) => {
                            last_error = Some(format!("Failed to read bytes for {}: {e}", task.url));
                            tokio::time::sleep(std::time::Duration::from_millis(150 * (attempt + 1))).await;
                            continue;
                        }
                    };

                    // Verify sha1 in memory before writing
                    if let Some(ref expected) = task.sha1 {
                        let mut hasher = Sha1::new();
                        hasher.update(&bytes);
                        let hash: String = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect();
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
                    return Err(last_error.unwrap_or_else(|| format!("Download failed for {}", task.url)));
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
