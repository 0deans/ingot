use std::collections::HashMap;
use std::path::PathBuf;

use super::error::InstanceExportError;
use super::hashing::hash_file;
use super::models::{EnvSupport, ExportOptions, FileSide};
use crate::artifacts::sources::{ModrinthVersionFileMatch, Scope, SourceClient};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurseForgeMatch {
    pub project_id: u32,
    pub file_id: u32,
}

/// Content resolver interface for remote provider lookups.
pub trait ContentResolver: Send + Sync {
    fn resolve_modrinth(
        &self,
        hashes: &[String],
    ) -> impl std::future::Future<
        Output = Result<HashMap<String, ModrinthVersionFileMatch>, InstanceExportError>,
    > + Send;

    fn resolve_curseforge(
        &self,
        fingerprints: &[u32],
    ) -> impl std::future::Future<Output = Result<HashMap<u32, CurseForgeMatch>, InstanceExportError>>
           + Send;
}

pub struct SourceClientResolver<'a> {
    pub client: SourceClient<'a>,
}

impl ContentResolver for SourceClientResolver<'_> {
    async fn resolve_modrinth(
        &self,
        hashes: &[String],
    ) -> Result<HashMap<String, ModrinthVersionFileMatch>, InstanceExportError> {
        self.client
            .modrinth_version_files_by_hash(hashes)
            .await
            .map_err(InstanceExportError::Provider)
    }

    async fn resolve_curseforge(
        &self,
        fingerprints: &[u32],
    ) -> Result<HashMap<u32, CurseForgeMatch>, InstanceExportError> {
        let scope = Scope::default();
        let matches = self
            .client
            .cf_identify(fingerprints, "", &scope)
            .await
            .map_err(InstanceExportError::Provider)?;

        let mut result = HashMap::new();
        for (fp, (release, _)) in matches {
            if let Ok(project_id) = release.project.parse::<u32>() {
                if let Ok(file_id) = release.version_id.parse::<u32>() {
                    result.insert(
                        fp,
                        CurseForgeMatch {
                            project_id,
                            file_id,
                        },
                    );
                }
            }
        }
        Ok(result)
    }
}

pub fn is_candidate_for_lookup(rel_path: &str) -> bool {
    let lower = rel_path.to_ascii_lowercase();
    (lower.starts_with("mods/")
        || lower.starts_with("resourcepacks/")
        || lower.starts_with("shaderpacks/"))
        && !lower.ends_with(".disabled")
}

pub fn is_modrinth_allowed_host(url_str: &str) -> bool {
    if let Ok(parsed) = reqwest::Url::parse(url_str) {
        if let Some(host) = parsed.host_str() {
            let lower = host.to_ascii_lowercase();
            return lower == "cdn.modrinth.com"
                || lower == "github.com"
                || lower == "raw.githubusercontent.com"
                || lower == "gitlab.com";
        }
    }
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedModrinthFile {
    pub path: String,
    pub sha1: String,
    pub sha512: String,
    pub size_bytes: u64,
    pub download_url: String,
    pub client_env: EnvSupport,
    pub server_env: EnvSupport,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassifiedModrinthFile {
    Resolved(ResolvedModrinthFile),
    Embedded {
        rel_path: String,
        source_path: PathBuf,
        target_override_path: String,
        warning: Option<String>,
    },
}

pub fn determine_target_override_path(rel_path: &str, file_sides: &[FileSide]) -> String {
    let side_config = file_sides.iter().find(|s| s.path == rel_path);
    match side_config {
        Some(s) if s.client != EnvSupport::Unsupported && s.server == EnvSupport::Unsupported => {
            format!("client-overrides/{rel_path}")
        }
        Some(s) if s.client == EnvSupport::Unsupported && s.server != EnvSupport::Unsupported => {
            format!("server-overrides/{rel_path}")
        }
        _ => format!("overrides/{rel_path}"),
    }
}

pub async fn classify_modrinth_files<R: ContentResolver>(
    resolver: &R,
    options: &ExportOptions,
    included_files: &[(String, PathBuf)],
) -> Result<Vec<ClassifiedModrinthFile>, InstanceExportError> {
    // 1. Hash candidate files and collect SHA-1s for batch lookup
    let mut candidate_hashes = Vec::new();
    let mut candidate_indices = Vec::new();

    for (idx, (rel_path, full_path)) in included_files.iter().enumerate() {
        if is_candidate_for_lookup(rel_path) {
            let hashes = hash_file(full_path, false)?;
            candidate_hashes.push(hashes.sha1.clone());
            candidate_indices.push((idx, hashes));
        }
    }

    // 2. Query Modrinth in batch
    let lookup_results = if !candidate_hashes.is_empty() {
        resolver.resolve_modrinth(&candidate_hashes).await?
    } else {
        HashMap::new()
    };

    // 3. Classify all files
    let mut classified = Vec::with_capacity(included_files.len());
    let mut cand_iter = candidate_indices.into_iter().peekable();

    for (idx, (rel_path, full_path)) in included_files.iter().enumerate() {
        if cand_iter
            .peek()
            .map(|(c_idx, _)| *c_idx == idx)
            .unwrap_or(false)
        {
            let (_, hashes) = cand_iter.next().unwrap();
            let matched = lookup_results.get(&hashes.sha1);

            let (client_env, server_env) = options
                .file_sides
                .iter()
                .find(|s| s.path == *rel_path)
                .map(|s| (s.client, s.server))
                .unwrap_or((EnvSupport::Required, EnvSupport::Required));

            if let Some(m) = matched {
                if is_modrinth_allowed_host(&m.url) {
                    classified.push(ClassifiedModrinthFile::Resolved(ResolvedModrinthFile {
                        path: rel_path.clone(),
                        sha1: hashes.sha1,
                        sha512: hashes.sha512,
                        size_bytes: hashes.size_bytes,
                        download_url: m.url.clone(),
                        client_env,
                        server_env,
                    }));
                } else {
                    let target_path = determine_target_override_path(rel_path, &options.file_sides);
                    classified.push(ClassifiedModrinthFile::Embedded {
                        rel_path: rel_path.clone(),
                        source_path: full_path.clone(),
                        target_override_path: target_path,
                        warning: Some(format!(
                            "{rel_path}: Download host is not in Modrinth allowlist"
                        )),
                    });
                }
            } else {
                let target_path = determine_target_override_path(rel_path, &options.file_sides);
                classified.push(ClassifiedModrinthFile::Embedded {
                    rel_path: rel_path.clone(),
                    source_path: full_path.clone(),
                    target_override_path: target_path,
                    warning: Some(format!("{rel_path}: Not found on Modrinth")),
                });
            }
        } else {
            // Not a candidate (config, disabled file, etc.)
            let target_path = determine_target_override_path(rel_path, &options.file_sides);
            classified.push(ClassifiedModrinthFile::Embedded {
                rel_path: rel_path.clone(),
                source_path: full_path.clone(),
                target_override_path: target_path,
                warning: None,
            });
        }
    }

    Ok(classified)
}
