use super::super::models::PluginError;
use std::path::Path;
pub async fn download(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
) -> Result<(), PluginError> {
    download_with_stage(client, url, destination, None).await
}

pub(super) async fn download_with_stage(
    client: &reqwest::Client,
    url: &str,
    destination: &Path,
    stage_owner: Option<std::sync::Arc<tempfile::TempDir>>,
) -> Result<(), PluginError> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(PluginError::RequestFailed)?;
    if !response.status().is_success() {
        return Err(PluginError::HttpStatus(response.status()));
    }
    let bytes = response
        .bytes()
        .await
        .map_err(PluginError::InvalidResponse)?;
    if bytes.is_empty() {
        return Err(PluginError::EmptyDownload);
    }
    let destination = destination.to_owned();
    tokio::task::spawn_blocking(move || {
        let _stage_owner = stage_owner;
        use std::io::Write;
        let parent = destination
            .parent()
            .ok_or(PluginError::InvalidPayload("destination parent"))?;
        let mut stage = tempfile::NamedTempFile::new_in(parent)?;
        stage.write_all(&bytes)?;
        stage.as_file().sync_all()?;
        stage
            .persist(&destination)
            .map_err(|error| PluginError::Io(error.error))?;
        Ok(())
    })
    .await?
}
