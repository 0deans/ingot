use std::collections::HashMap;

use serde::Deserialize;

use super::http::{nonempty, optional};
use super::{Channel, Dep, Release, Source, SourceClient, SourceError, TargetFile};

#[derive(Deserialize)]
struct Versions {
    result: Vec<Version>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Version {
    name: String,
    channel: ChannelInfo,
    downloads: HashMap<String, Download>,
    plugin_dependencies: HashMap<String, Vec<Dependency>>,
}
#[derive(Deserialize)]
struct ChannelInfo {
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Download {
    download_url: Option<String>,
    file_info: Option<FileInfo>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileInfo {
    name: String,
    sha256_hash: Option<String>,
    size_bytes: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Dependency {
    required: bool,
    name: String,
    external_url: Option<String>,
}

impl Version {
    fn release(mut self, project: &str) -> Result<Option<Release>, SourceError> {
        nonempty(&self.name, Source::Hangar, "version name")?;
        let Some(download) = self.downloads.remove("PAPER") else {
            return Ok(None);
        };
        let channel = if self.channel.name.eq_ignore_ascii_case("release") {
            Channel::Release
        } else {
            Channel::Beta
        };
        let file = match download.download_url.filter(|url| !url.trim().is_empty()) {
            Some(url) => {
                let info = download.file_info.ok_or(SourceError::InvalidPayload {
                    provider: Source::Hangar,
                    field: "fileInfo",
                })?;
                nonempty(&info.name, Source::Hangar, "file name")?;
                Some(TargetFile {
                    version_id: self.name.clone(),
                    version_number: self.name.clone(),
                    file_name: info.name,
                    url,
                    sha1: None,
                    sha256: info.sha256_hash.filter(|hash| !hash.is_empty()),
                    bytes: info.size_bytes as f64,
                })
            }
            None => None,
        };
        let mut deps = Vec::new();
        for dependency in self.plugin_dependencies.remove("PAPER").unwrap_or_default() {
            if !dependency.required || dependency.external_url.is_some() {
                continue;
            }
            nonempty(&dependency.name, Source::Hangar, "dependency name")?;
            deps.push(Dep {
                required: true,
                project: Some(dependency.name),
                version_id: None,
            });
        }
        Ok(Some(Release {
            source: Source::Hangar,
            project: project.to_owned(),
            version_id: self.name.clone(),
            version_number: self.name,
            channel,
            file,
            deps,
        }))
    }
}

impl SourceClient<'_> {
    pub async fn hangar_versions(
        &self,
        project: &str,
        game: &str,
    ) -> Result<Vec<Release>, SourceError> {
        let request = self
            .http
            .get(format!("{}/projects/{project}/versions", self.hangar))
            .query(&[
                ("platform", "PAPER"),
                ("platformVersion", game),
                ("limit", "25"),
            ]);
        let Some(found): Option<Versions> = optional(request, Source::Hangar).await? else {
            return Ok(Vec::new());
        };
        let mut releases = Vec::new();
        for version in found.result {
            if let Some(release) = version.release(project)? {
                releases.push(release);
            }
        }
        Ok(releases)
    }
}
