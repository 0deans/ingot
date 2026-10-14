use super::models::{dir_size, safe_file_name_typed, sha1_hex, Target, WorldInfo};
use super::sources;
use std::fs;

pub struct LocalFile {
    pub folder: usize,
    pub file_name: String,
    pub sha1: String,
    pub fingerprint: Option<u32>,
}

pub fn scan(target: &Target) -> Vec<LocalFile> {
    let mut out = Vec::new();
    for (index, folder) in target.folders.iter().enumerate() {
        let Ok(entries) = fs::read_dir(target.root.join(folder.dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if !entry.path().is_file() || safe_file_name_typed(&file_name, folder.kind).is_err() {
                continue;
            }
            let Ok(data) = fs::read(entry.path()) else {
                continue;
            };
            out.push(LocalFile {
                folder: index,
                file_name,
                sha1: sha1_hex(&data),
                fingerprint: folder
                    .scope
                    .curseforge
                    .then(|| sources::cf_fingerprint(&data)),
            });
        }
    }
    out.sort_unstable_by_key(|f| f.file_name.to_lowercase());
    out
}

pub fn list_worlds(target: &Target) -> Vec<WorldInfo> {
    target
        .worlds
        .iter()
        .map(|w| WorldInfo {
            folder_name: w.clone(),
            bytes: dir_size(&target.root.join(w)) as f64,
        })
        .collect()
}
