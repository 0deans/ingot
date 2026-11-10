use super::error::DownloadError;
use super::range::validate_content_range;
use super::verify_file_sha1;
use super::worker::{StagingCleanupGuard, WorkerTaskGuard};
use std::fs;
use std::io::SeekFrom;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Semaphore;

/// Downloads a large file using HTTP Range multi-threaded chunking if supported,
/// or regular streaming if ranges are not supported or file is small.
pub async fn download_file_chunked(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    expected_sha1: Option<&str>,
    progress: Option<Arc<dyn Fn(u64, u64) + Send + Sync>>,
) -> Result<(), DownloadError> {
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
) -> Result<(), DownloadError> {
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
        fs::create_dir_all(parent)?;
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
    }

    enum ChunkCompletion {
        Complete,
        Fallback,
        Failed(DownloadError),
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
        let file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&staging_dest)
            .await?;
        if let Err(e) = file.set_len(content_length).await {
            let _ = tokio::fs::remove_file(&staging_dest).await;
            return Err(DownloadError::Io(e));
        }

        let shared_file = Arc::new(tokio::sync::Mutex::new(file));
        let downloaded_bytes = Arc::new(AtomicU64::new(0));
        let num_chunks = content_length.div_ceil(CHUNK_SIZE) as usize;
        let semaphore = Arc::new(Semaphore::new(6)); // Max 6 concurrent chunks

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
                let _permit = acquire_chunk_permit(&sem).await?;
                let range_header = format!("bytes={}-{}", start, end);
                let response = client
                    .get(&url)
                    .header(reqwest::header::RANGE, range_header)
                    .send()
                    .await?;

                let status = response.status();
                if status == reqwest::StatusCode::OK {
                    return Ok::<ChunkResult, DownloadError>(ChunkResult::RangeNotSupported);
                }

                if status != reqwest::StatusCode::PARTIAL_CONTENT {
                    return Err(DownloadError::HttpStatus { status });
                }

                validate_content_range(response.headers(), start, end, content_length)?;

                let chunk_bytes = response.bytes().await?;

                let expected_len = end - start + 1;
                if chunk_bytes.len() as u64 != expected_len {
                    return Err(DownloadError::ChunkLengthMismatch {
                        expected: expected_len,
                        actual: chunk_bytes.len() as u64,
                    });
                }

                {
                    let mut file = file_lock.lock().await;
                    file.seek(SeekFrom::Start(start)).await?;
                    file.write_all(&chunk_bytes).await?;
                }

                let total_dl = downloaded.fetch_add(chunk_bytes.len() as u64, Ordering::Relaxed)
                    + chunk_bytes.len() as u64;
                if let Some(ref hook) = chunk_written_cb {
                    hook(total_dl, content_length);
                }
                report(total_dl, content_length, false);

                Ok::<ChunkResult, DownloadError>(ChunkResult::Success)
            });
        }

        let completion = loop {
            match tasks.set.join_next().await {
                None => break ChunkCompletion::Complete,
                Some(Ok(Ok(ChunkResult::Success))) => {}
                Some(Ok(Ok(ChunkResult::RangeNotSupported))) => break ChunkCompletion::Fallback,
                Some(Ok(Err(error))) => break ChunkCompletion::Failed(error),
                Some(Err(error)) => break ChunkCompletion::Failed(DownloadError::Task(error)),
            }
        };

        tasks.shutdown().await;
        tasks.disarm();
        match completion {
            ChunkCompletion::Complete => {
                report_throttled(content_length, content_length, true);
                let mut file = shared_file.lock().await;
                file.flush().await?;
                file.sync_all().await?;
            }
            ChunkCompletion::Fallback => {
                drop(shared_file);
                let _ = tokio::fs::remove_file(&staging_dest).await;
                use_streaming = true;
            }
            ChunkCompletion::Failed(error) => {
                drop(shared_file);
                let _ = tokio::fs::remove_file(&staging_dest).await;
                return Err(error);
            }
        }
    }

    if use_streaming {
        // Standard streaming download
        let mut response = match client.get(url).send().await {
            Ok(r) => r,
            Err(e) => {
                let _ = tokio::fs::remove_file(&staging_dest).await;
                return Err(DownloadError::from(e));
            }
        };

        if !response.status().is_success() {
            let _ = tokio::fs::remove_file(&staging_dest).await;
            return Err(DownloadError::HttpStatus {
                status: response.status(),
            });
        }

        let total_size = response.content_length().unwrap_or(content_length);
        let mut file = match tokio::fs::File::create(&staging_dest).await {
            Ok(f) => f,
            Err(e) => return Err(DownloadError::Io(e)),
        };

        let mut downloaded: u64 = 0;
        report_throttled(0, total_size, true);

        while let Some(chunk) = match response.chunk().await {
            Ok(c) => c,
            Err(e) => {
                let _ = tokio::fs::remove_file(&staging_dest).await;
                return Err(DownloadError::from(e));
            }
        } {
            if let Err(e) = file.write_all(&chunk).await {
                let _ = tokio::fs::remove_file(&staging_dest).await;
                return Err(DownloadError::Io(e));
            }
            downloaded += chunk.len() as u64;
            report_throttled(downloaded, total_size, false);
        }

        report_throttled(total_size, total_size, true);
        file.flush().await?;
        file.sync_all().await?;
    }

    // Verify SHA-1 on temporary file before committing to dest
    if let Some(expected) = expected_sha1 {
        if !verify_file_sha1(&staging_dest, expected) {
            let _ = tokio::fs::remove_file(&staging_dest).await;
            return Err(DownloadError::Sha1Mismatch(dest.to_path_buf()));
        }
    }

    // Commit file
    tokio::fs::rename(&staging_dest, dest).await.map_err(|e| {
        let _ = fs::remove_file(&staging_dest);
        DownloadError::Io(e)
    })?;

    staging_guard.committed = true;
    Ok(())
}

async fn acquire_chunk_permit(
    semaphore: &Semaphore,
) -> Result<tokio::sync::SemaphorePermit<'_>, DownloadError> {
    Ok(semaphore.acquire().await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[tokio::test]
    async fn closed_concurrency_limit_preserves_acquisition_failure() {
        let semaphore = Semaphore::new(1);
        semaphore.close();
        let error = acquire_chunk_permit(&semaphore).await.unwrap_err();
        assert!(matches!(error, DownloadError::Semaphore(_)));
        assert!(error.source().unwrap().is::<tokio::sync::AcquireError>());
    }
}
