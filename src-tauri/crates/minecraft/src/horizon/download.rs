use super::models::{HorizonError, HorizonRelease};
use std::fs;
use std::path::{Path, PathBuf};

pub fn sanitize_tag_for_fs(tag: &str) -> String {
    tag.replace(['/', '\\', ':', '@'], "-")
}

pub fn client_binary_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "horizon-client.exe"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "horizon-client"
    }
}

pub async fn ensure_horizon_engine<FProg>(
    client: &reqwest::Client,
    engines_dir: &Path,
    release: &HorizonRelease,
    report_prog: FProg,
) -> Result<PathBuf, HorizonError>
where
    FProg: Fn(&str, usize, usize, &str) + Send + Sync,
{
    let sanitized_tag = sanitize_tag_for_fs(&release.tag);
    let target_dir = engines_dir.join("horizon").join(&sanitized_tag);
    let expected_binary = client_binary_name();

    let candidate_direct = target_dir.join(expected_binary);
    if candidate_direct.is_file() {
        return Ok(candidate_direct);
    }

    if let Ok(entries) = fs::read_dir(&target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let nested = path.join(expected_binary);
                if nested.is_file() {
                    return Ok(nested);
                }
            }
        }
    }

    let download_url = release
        .asset_url
        .as_deref()
        .ok_or_else(|| HorizonError::AssetNotFound(release.tag.clone()))?;

    report_prog(
        "Downloading Horizon",
        0,
        100,
        &format!("Downloading Horizon {} engine...", release.name),
    );

    let response = client
        .get(download_url)
        .header("User-Agent", "Ingot-Launcher")
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(HorizonError::Transport(
            response.error_for_status().unwrap_err(),
        ));
    }

    let bytes = response.bytes().await?;

    report_prog(
        "Extracting Horizon",
        80,
        100,
        "Unpacking native engine files...",
    );

    fs::create_dir_all(&target_dir)?;

    extract_engine_archive(&bytes, download_url, &target_dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o755);
        if candidate_direct.is_file() {
            let _ = fs::set_permissions(&candidate_direct, perms);
        }
    }

    if candidate_direct.is_file() {
        return Ok(candidate_direct);
    }

    if let Ok(entries) = fs::read_dir(&target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let nested = path.join(expected_binary);
                if nested.is_file() {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let perms = fs::Permissions::from_mode(0o755);
                        let _ = fs::set_permissions(&nested, perms);
                    }
                    return Ok(nested);
                }
            }
        }
    }

    Err(HorizonError::ExecutableNotFound(
        target_dir.display().to_string(),
    ))
}

fn extract_engine_archive(
    bytes: &[u8],
    source_url: &str,
    target_dir: &Path,
) -> Result<(), HorizonError> {
    if source_url.ends_with(".zip") {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| HorizonError::Extraction(format!("ZIP error: {e}")))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| HorizonError::Extraction(format!("ZIP entry error: {e}")))?;
            let Some(enclosed) = file.enclosed_name().map(|p| p.to_path_buf()) else {
                continue;
            };

            let out_path = target_dir.join(enclosed);
            if file.is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut out_file = fs::File::create(&out_path)?;
                std::io::copy(&mut file, &mut out_file)?;
            }
        }
        Ok(())
    } else if source_url.ends_with(".tar.gz") || source_url.ends_with(".tgz") {
        let cursor = std::io::Cursor::new(bytes);
        let gz = flate2::read::GzDecoder::new(cursor);
        let mut archive = tar::Archive::new(gz);
        archive
            .unpack(target_dir)
            .map_err(|e| HorizonError::Extraction(format!("TAR error: {e}")))?;
        Ok(())
    } else {
        Err(HorizonError::Extraction(format!(
            "Unsupported archive format: {source_url}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_tag_for_fs() {
        assert_eq!(
            sanitize_tag_for_fs("nightly/build-2026-10-31-1"),
            "nightly-build-2026-10-31-1"
        );
        assert_eq!(sanitize_tag_for_fs("v0.2.30"), "v0.2.30");
        assert_eq!(sanitize_tag_for_fs("launcher@0.2.30"), "launcher-0.2.30");
    }
}
