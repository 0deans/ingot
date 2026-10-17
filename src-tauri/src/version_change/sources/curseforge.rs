use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;

use super::http::{nonempty, optional, required};
use super::{
    Channel, Dep, ProjectInfo, Release, Scope, Source, SourceClient, SourceError, TargetFile,
};

#[derive(Deserialize)]
struct Data<T> {
    data: T,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Matches {
    exact_matches: Vec<ExactMatch>,
}

#[derive(Deserialize)]
struct ExactMatch {
    id: u64,
    file: File,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct File {
    id: u64,
    display_name: String,
    file_name: String,
    download_url: Option<String>,
    file_length: u64,
    file_date: String,
    release_type: u32,
    game_versions: Vec<String>,
    hashes: Vec<Hash>,
    dependencies: Vec<Dependency>,
    file_fingerprint: Option<u32>,
}

#[derive(Deserialize)]
struct Hash {
    algo: u32,
    value: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Dependency {
    mod_id: u64,
    relation_type: u32,
}

#[derive(Deserialize)]
struct Project {
    id: u64,
    name: String,
    logo: Option<Logo>,
    links: Links,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Logo {
    thumbnail_url: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Links {
    website_url: Option<String>,
}

fn project_id(value: &str) -> Result<u64, SourceError> {
    value
        .parse::<u64>()
        .ok()
        .filter(|id| *id > 0)
        .ok_or(SourceError::InvalidProjectId {
            provider: Source::Curseforge,
        })
}

fn authorized(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request.header("x-api-key", crate::minecraft::content::CURSEFORGE_API_KEY)
}

impl File {
    fn supports(&self, game: &str, scope: &Scope) -> bool {
        self.game_versions.iter().any(|version| version == game)
            && (scope.cf_loader_names.is_empty()
                || self.game_versions.iter().any(|version| {
                    scope
                        .cf_loader_names
                        .iter()
                        .any(|loader| version.eq_ignore_ascii_case(loader))
                }))
    }

    fn release(self, project: u64) -> Result<Release, SourceError> {
        if self.id == 0 || project == 0 {
            return Err(SourceError::InvalidPayload {
                provider: Source::Curseforge,
                field: "release identity",
            });
        }
        nonempty(&self.display_name, Source::Curseforge, "displayName")?;
        let channel = match self.release_type {
            1 => Channel::Release,
            2 => Channel::Beta,
            3 => Channel::Alpha,
            _ => {
                return Err(SourceError::InvalidPayload {
                    provider: Source::Curseforge,
                    field: "releaseType",
                })
            }
        };
        let version_id = self.id.to_string();
        let url = self.download_url.filter(|url| !url.trim().is_empty());
        let file = match url {
            Some(url) => {
                nonempty(&self.file_name, Source::Curseforge, "fileName")?;
                let sha1 = self
                    .hashes
                    .iter()
                    .find(|hash| hash.algo == 1)
                    .map(|hash| hash.value.clone())
                    .filter(|hash| !hash.is_empty());
                Some(TargetFile {
                    version_id: version_id.clone(),
                    version_number: self.display_name.clone(),
                    file_name: self.file_name,
                    url,
                    sha1,
                    sha256: None,
                    bytes: self.file_length as f64,
                })
            }
            None => None,
        };
        let mut deps = Vec::new();
        for dependency in self.dependencies {
            let required = match dependency.relation_type {
                3 => true,
                5 => false,
                _ => continue,
            };
            if dependency.mod_id == 0 {
                return Err(SourceError::InvalidPayload {
                    provider: Source::Curseforge,
                    field: "dependency identity",
                });
            }
            deps.push(Dep {
                required,
                project: Some(dependency.mod_id.to_string()),
                version_id: None,
            });
        }
        Ok(Release {
            source: Source::Curseforge,
            project: project.to_string(),
            version_id,
            version_number: self.display_name,
            channel,
            file,
            deps,
        })
    }
}

impl SourceClient<'_> {
    pub async fn cf_identify(
        &self,
        fingerprints: &[u32],
        game: &str,
        scope: &Scope,
    ) -> Result<HashMap<u32, (Release, bool)>, SourceError> {
        if fingerprints.is_empty() {
            return Ok(HashMap::new());
        }
        let request = authorized(
            self.http
                .post(format!("{}/fingerprints/432", self.curseforge)),
        )
        .json(&json!({ "fingerprints": fingerprints }));
        let found: Data<Matches> = required(request, Source::Curseforge).await?;
        let mut matches = HashMap::new();
        for entry in found.data.exact_matches {
            let fingerprint = entry
                .file
                .file_fingerprint
                .ok_or(SourceError::InvalidPayload {
                    provider: Source::Curseforge,
                    field: "fileFingerprint",
                })?;
            let supports = entry.file.supports(game, scope);
            matches.insert(fingerprint, (entry.file.release(entry.id)?, supports));
        }
        Ok(matches)
    }

    pub async fn cf_target(
        &self,
        project: &str,
        game: &str,
        scope: &Scope,
    ) -> Result<Option<Release>, SourceError> {
        let project = project_id(project)?;
        let mut query = vec![
            ("gameVersion", game.to_owned()),
            ("pageSize", "50".to_owned()),
        ];
        if let Some(loader) = scope.cf_loader {
            query.push(("modLoaderType", loader.to_string()));
        }
        let request = authorized(
            self.http
                .get(format!("{}/mods/{project}/files", self.curseforge))
                .query(&query),
        );
        let Some(found): Option<Data<Vec<File>>> = optional(request, Source::Curseforge).await?
        else {
            return Ok(None);
        };
        let mut files = found.data;
        files.sort_by(|a, b| b.file_date.cmp(&a.file_date));
        let mut releases = Vec::new();
        for file in files {
            let supports = file.supports(game, scope);
            let release = file.release(project)?;
            if supports {
                releases.push(release);
            }
        }
        for channel in [Channel::Release, Channel::Beta, Channel::Alpha] {
            if let Some(index) = releases
                .iter()
                .position(|release| release.channel == channel)
            {
                return Ok(Some(releases.remove(index)));
            }
        }
        Ok(None)
    }

    pub async fn cf_projects(
        &self,
        ids: &[String],
    ) -> Result<HashMap<String, ProjectInfo>, SourceError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let numeric: Vec<u64> = ids
            .iter()
            .map(|id| project_id(id))
            .collect::<Result<_, _>>()?;
        let request = authorized(self.http.post(format!("{}/mods", self.curseforge)))
            .json(&json!({ "modIds": numeric }));
        let found: Data<Vec<Project>> = required(request, Source::Curseforge).await?;
        let mut projects = HashMap::new();
        for project in found.data {
            if project.id == 0 {
                return Err(SourceError::InvalidPayload {
                    provider: Source::Curseforge,
                    field: "project id",
                });
            }
            nonempty(&project.name, Source::Curseforge, "project name")?;
            let info = ProjectInfo {
                title: project.name,
                icon_url: project.logo.and_then(|logo| logo.thumbnail_url),
                page_url: project.links.website_url,
            };
            projects.insert(project.id.to_string(), info);
        }
        Ok(projects)
    }
}
