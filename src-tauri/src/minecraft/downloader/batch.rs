use super::{is_download_task_cached, DownloadTask};
use sha1::{Digest, Sha1};
use std::collections::HashSet;
use std::io::Write;
use std::sync::Arc;

pub type BatchProgressCallback = Arc<dyn Fn(usize, usize, &str) + Send + Sync>;

#[derive(Debug, thiserror::Error)]
enum BatchError {
    #[error("Duplicate download destination: {0}")]
    Duplicate(std::path::PathBuf),
    #[error("Download transport failed")]
    Transport(#[from] reqwest::Error),
    #[error("Downloaded content does not match its declared {0}")]
    Integrity(&'static str),
    #[error("Download filesystem operation failed")]
    Io(#[from] std::io::Error),
    #[error("Download task failed")]
    Join(#[from] tokio::task::JoinError),
}

fn sha1_hex(bytes: &[u8]) -> String {
    Sha1::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn download_one(
    client: reqwest::Client,
    task: DownloadTask,
) -> Result<DownloadTask, BatchError> {
    download_one_with_preparer(client, task, prepare_download).await
}

fn prepare_download(
    bytes: &[u8],
    destination: &std::path::Path,
) -> std::io::Result<tempfile::NamedTempFile> {
    let parent = destination.parent().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Destination has no parent",
        )
    })?;
    std::fs::create_dir_all(parent)?;
    let mut staging = tempfile::NamedTempFile::new_in(parent)?;
    staging.write_all(bytes)?;
    staging.as_file().sync_all()?;
    Ok(staging)
}

async fn download_one_with_preparer(
    client: reqwest::Client,
    task: DownloadTask,
    prepare: impl FnOnce(&[u8], &std::path::Path) -> std::io::Result<tempfile::NamedTempFile>
        + Send
        + 'static,
) -> Result<DownloadTask, BatchError> {
    let bytes = client
        .get(&task.url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    if task.size.is_some_and(|size| size != bytes.len() as u64) {
        return Err(BatchError::Integrity("size"));
    }
    if let Some(expected) = &task.sha1 {
        let hash = sha1_hex(&bytes);
        if !hash.eq_ignore_ascii_case(expected) {
            return Err(BatchError::Integrity("SHA-1"));
        }
    }
    let destination = task.dest.clone();
    // Blocking work only prepares an owned temporary file. A dropped receiver can
    // never publish it; the returned tempfile is removed when the task result drops.
    let prepared = tokio::task::spawn_blocking(move || prepare(&bytes, &destination)).await??;
    // No await separates ownership of verified bytes and atomic publication.
    prepared
        .persist(&task.dest)
        .map_err(|error| BatchError::Io(error.error))?;
    Ok(task)
}

/// Completed files are committed individually. Failure cancels remaining work;
/// already committed, verified files remain valid cache entries for a retry.
pub async fn download_files_concurrent(
    client: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    max_concurrency: usize,
    progress: Option<BatchProgressCallback>,
) -> Result<(), String> {
    run_batch(client, tasks, max_concurrency, progress)
        .await
        .map_err(|error| error.to_string())
}

async fn run_batch(
    client: &reqwest::Client,
    tasks: Vec<DownloadTask>,
    max_concurrency: usize,
    progress: Option<BatchProgressCallback>,
) -> Result<(), BatchError> {
    let mut destinations = HashSet::new();
    for task in &tasks {
        if !destinations.insert(task.dest.clone()) {
            return Err(BatchError::Duplicate(task.dest.clone()));
        }
    }
    let mut needed = tasks
        .into_iter()
        .filter(|task| !is_download_task_cached(task))
        .collect::<Vec<_>>();
    let total = needed.len();
    let capacity = max_concurrency.max(1).min(total);
    let mut workers = tokio::task::JoinSet::new();
    for _ in 0..capacity {
        if let Some(task) = needed.pop() {
            workers.spawn(download_one(client.clone(), task));
        }
    }
    let mut completed = 0;
    while let Some(result) = workers.join_next().await {
        let task = match result {
            Ok(Ok(task)) => task,
            Ok(Err(error)) => {
                workers.shutdown().await;
                return Err(error);
            }
            Err(error) => {
                workers.shutdown().await;
                return Err(BatchError::Join(error));
            }
        };
        completed += 1;
        if let Some(callback) = &progress {
            let filename = task
                .dest
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("file");
            callback(completed, total, filename);
        }
        if let Some(task) = needed.pop() {
            workers.spawn(download_one(client.clone(), task));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn response_server(body: &'static [u8]) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            assert!(socket.read(&mut request).await.unwrap() > 0);
            socket
                .write_all(
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    )
                    .as_bytes(),
                )
                .await
                .unwrap();
            socket.write_all(body).await.unwrap();
        });
        (url, handle)
    }

    #[tokio::test]
    async fn cancelled_worker_cannot_publish_after_blocking_preparation_finishes() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("library.jar");
        std::fs::write(&destination, b"previous").unwrap();
        let (url, server) = response_server(b"new").await;
        let task = DownloadTask {
            url,
            dest: destination.clone(),
            size: Some(3),
            sha1: Some(sha1_hex(b"new")),
        };
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let worker = tokio::spawn(download_one_with_preparer(
            reqwest::Client::new(),
            task,
            move |bytes, path| {
                let staged = prepare_download(bytes, path)?;
                started_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(staged)
            },
        ));
        tokio::time::timeout(std::time::Duration::from_secs(5), started_rx)
            .await
            .unwrap()
            .unwrap();
        server.await.unwrap();
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 2);
        worker.abort();
        match worker.await {
            Err(error) => assert!(error.is_cancelled()),
            Ok(_) => panic!("worker completed before cancellation"),
        }
        assert_eq!(std::fs::read(&destination).unwrap(), b"previous");
        release_tx.send(()).unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while std::fs::read_dir(temp.path()).unwrap().count() != 1 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(std::fs::read(destination).unwrap(), b"previous");
    }

    #[test]
    fn existing_directory_is_not_a_completed_cached_file() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("library.jar");
        std::fs::create_dir(&destination).unwrap();
        std::fs::write(destination.join("data"), b"directory is not a library").unwrap();
        let task = DownloadTask {
            url: "invalid://unused".into(),
            dest: destination,
            size: None,
            sha1: None,
        };
        assert!(!is_download_task_cached(&task));
    }

    #[tokio::test]
    async fn integrity_failure_preserves_working_destination() {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("library.jar");
        std::fs::write(&dest, b"previous").unwrap();
        let (url, server) = response_server(b"bad").await;
        let task = DownloadTask {
            url,
            dest: dest.clone(),
            size: Some(4),
            sha1: None,
        };
        let result = run_batch(&reqwest::Client::new(), vec![task], 2, None).await;
        assert!(matches!(result, Err(BatchError::Integrity("size"))));
        server.await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"previous");
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn verified_file_replaces_destination_without_leaking_staging() {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("library.jar");
        std::fs::write(&dest, b"previous").unwrap();
        let (url, server) = response_server(b"new").await;
        let task = DownloadTask {
            url,
            dest: dest.clone(),
            size: Some(3),
            sha1: Some(sha1_hex(b"new")),
        };
        run_batch(&reqwest::Client::new(), vec![task], 1, None)
            .await
            .unwrap();
        server.await.unwrap();
        assert_eq!(std::fs::read(&dest).unwrap(), b"new");
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    }

    #[tokio::test]
    async fn rejects_duplicate_destinations_before_network_or_publication() {
        let temp = tempfile::tempdir().unwrap();
        let task = DownloadTask {
            url: "invalid://not-requested".into(),
            dest: temp.path().join("library.jar"),
            size: None,
            sha1: None,
        };
        let result = run_batch(&reqwest::Client::new(), vec![task.clone(), task], 2, None).await;
        assert!(matches!(result, Err(BatchError::Duplicate(_))));
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn dropping_batch_cancels_in_progress_worker_and_preserves_destination() {
        let temp = tempfile::tempdir().unwrap();
        let dest = temp.path().join("library.jar");
        std::fs::write(&dest, b"previous").unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/file", listener.local_addr().unwrap());
        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            assert!(socket.read(&mut request).await.unwrap() > 0);
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\na")
                .await
                .unwrap();
            started_tx.send(()).unwrap();
            let mut remaining = [0; 32];
            let result = socket.read(&mut remaining).await;
            assert!(
                matches!(result, Ok(0) | Err(_)),
                "Cancelled worker left HTTP response alive"
            );
        });
        let task = DownloadTask {
            url,
            dest: dest.clone(),
            size: Some(100),
            sha1: None,
        };
        let batch =
            tokio::spawn(
                async move { run_batch(&reqwest::Client::new(), vec![task], 1, None).await },
            );
        tokio::time::timeout(std::time::Duration::from_secs(5), started_rx)
            .await
            .unwrap()
            .unwrap();
        batch.abort();
        assert!(batch.await.unwrap_err().is_cancelled());
        tokio::time::timeout(std::time::Duration::from_secs(5), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read(dest).unwrap(), b"previous");
        assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
    }
}
