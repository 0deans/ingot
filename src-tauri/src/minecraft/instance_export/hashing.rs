use std::fmt::Write as _;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use sha1::{Digest as _, Sha1};
use sha2::Sha512;

use super::error::InstanceExportError;
use crate::version_change::sources::cf_fingerprint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileHashes {
    pub sha1: String,
    pub sha512: String,
    pub size_bytes: u64,
    pub cf_fingerprint: Option<u32>,
}

/// Calculate SHA-1, SHA-512, file size, and optionally CurseForge fingerprint.
pub fn hash_file(
    path: &Path,
    compute_fingerprint: bool,
) -> Result<FileHashes, InstanceExportError> {
    let mut file = File::open(path).map_err(|e| InstanceExportError::io(path, e))?;
    let mut sha1_hasher = Sha1::new();
    let mut sha512_hasher = Sha512::new();
    let mut total_size: u64 = 0;
    let mut filtered_bytes = Vec::new();

    let mut buf = [0u8; 65536];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| InstanceExportError::io(path, e))?;
        if n == 0 {
            break;
        }
        let chunk = &buf[..n];
        sha1_hasher.update(chunk);
        sha512_hasher.update(chunk);
        total_size += n as u64;

        if compute_fingerprint {
            for &b in chunk {
                if !matches!(b, 9 | 10 | 13 | 32) {
                    filtered_bytes.push(b);
                }
            }
        }
    }

    let sha1_raw = sha1_hasher.finalize();
    let mut sha1 = String::with_capacity(40);
    for b in sha1_raw {
        let _ = write!(sha1, "{b:02x}");
    }

    let sha512_raw = sha512_hasher.finalize();
    let mut sha512 = String::with_capacity(128);
    for b in sha512_raw {
        let _ = write!(sha512, "{b:02x}");
    }

    let fingerprint = if compute_fingerprint {
        Some(cf_fingerprint(&filtered_bytes))
    } else {
        None
    };

    Ok(FileHashes {
        sha1,
        sha512,
        size_bytes: total_size,
        cf_fingerprint: fingerprint,
    })
}
