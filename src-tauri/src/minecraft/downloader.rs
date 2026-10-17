use sha1::{Digest, Sha1};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct DownloadTask {
    pub url: String,
    pub dest: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

pub fn is_download_task_cached(task: &DownloadTask) -> bool {
    let meta = match fs::symlink_metadata(&task.dest) {
        Ok(m) => m,
        Err(_) => return false,
    };
    if !meta.is_file() || meta.len() == 0 {
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

pub mod error;
pub use error::DownloadError;
mod batch;
mod range;
mod single;
mod worker;

pub use batch::{download_files_concurrent, BatchProgressCallback};
pub use single::download_file_chunked;
#[cfg(test)]
use single::download_file_chunked_with_chunk_listener;

#[cfg(test)]
mod tests;
