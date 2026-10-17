use std::collections::HashMap;

use serde::Deserialize;
use serde_json::json;

use super::http::{nonempty, optional, required};
use super::{Channel, Dep, ProjectInfo, Release, Source, SourceClient, SourceError, TargetFile};

#[derive(Deserialize)]
struct Version {
    id: String,
    project_id: String,
    version_number: String,
    version_type: String,
    game_versions: Vec<String>,
    loaders: Vec<String>,
    files: Vec<File>,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct File {
    filename: String,
    url: String,
    size: u64,
    primary: bool,
    hashes: HashMap<String, String>,
}

#[derive(Deserialize)]
struct Dependency {
    dependency_type: String,
    project_id: Option<String>,
    version_id: Option<String>,
}

#[derive(Deserialize)]
struct Project {
    id: String,
    title: String,
    slug: String,
    icon_url: Option<String>,
}

impl Version {
    fn supports(&self, game: &str, loaders: &[&str]) -> bool {
        self.game_versions.iter().any(|version| version == game)
            && (loaders.is_empty()
                || self
                    .loaders
                    .iter()
                    .any(|loader| loaders.contains(&loader.as_str())))
    }

    fn release(self) -> Result<Release, SourceError> {
        for value in [&self.id, &self.project_id, &self.version_number] {
            nonempty(value, Source::Modrinth, "version identity")?;
        }
        let channel = match self.version_type.as_str() {
            "release" => Channel::Release,
            "beta" => Channel::Beta,
            "alpha" => Channel::Alpha,
            _ => {
                return Err(SourceError::InvalidPayload {
                    provider: Source::Modrinth,
                    field: "version_type",
                })
            }
        };
        let file = self
            .files
            .iter()
            .find(|file| file.primary)
            .or(self.files.first());
        let file = match file {
            Some(file) => {
                nonempty(&file.filename, Source::Modrinth, "filename")?;
                nonempty(&file.url, Source::Modrinth, "download URL")?;
                Some(TargetFile {
                    version_id: self.id.clone(),
                    version_number: self.version_number.clone(),
                    file_name: file.filename.clone(),
                    url: file.url.clone(),
                    sha1: file
                        .hashes
                        .get("sha1")
                        .filter(|hash| !hash.is_empty())
                        .cloned(),
                    sha256: None,
                    bytes: file.size as f64,
                })
            }
            None => None,
        };
        let mut deps = Vec::new();
        for dependency in self.dependencies {
            let required = match dependency.dependency_type.as_str() {
                "required" => true,
                "incompatible" => false,
                _ => continue,
            };
            let project = dependency
                .project_id
                .filter(|value| !value.trim().is_empty());
            let version_id = dependency
                .version_id
                .filter(|value| !value.trim().is_empty());
            if project.is_none() && version_id.is_none() {
                return Err(SourceError::InvalidPayload {
                    provider: Source::Modrinth,
                    field: "dependency identity",
                });
            }
            deps.push(Dep {
                required,
                project,
                version_id,
            });
        }
        Ok(Release {
            source: Source::Modrinth,
            project: self.project_id,
            version_id: self.id,
            version_number: self.version_number,
            channel,
            file,
            deps,
        })
    }
}

impl SourceClient<'_> {
    pub async fn modrinth_identify(
        &self,
        hashes: &[String],
        game: &str,
        loaders: &[&str],
    ) -> Result<HashMap<String, (Release, bool)>, SourceError> {
        if hashes.is_empty() {
            return Ok(HashMap::new());
        }
        let request = self
            .http
            .post(format!("{}/version_files", self.modrinth))
            .json(&json!({"hashes": hashes, "algorithm": "sha1"}));
        let versions: HashMap<String, Version> = required(request, Source::Modrinth).await?;
        let mut found = HashMap::new();
        for (hash, version) in versions {
            let supports = version.supports(game, loaders);
            found.insert(hash, (version.release()?, supports));
        }
        Ok(found)
    }

    pub async fn modrinth_target(
        &self,
        project: &str,
        game: &str,
        loaders: &[&str],
    ) -> Result<Option<Release>, SourceError> {
        if loaders.is_empty() {
            return Ok(None);
        }
        let request = self
            .http
            .get(format!("{}/project/{project}/version", self.modrinth))
            .query(&[
                ("loaders", json!(loaders).to_string()),
                ("game_versions", json!([game]).to_string()),
            ]);
        let Some(versions): Option<Vec<Version>> = optional(request, Source::Modrinth).await?
        else {
            return Ok(None);
        };
        let mut releases = Vec::new();
        for version in versions {
            let supports = version.supports(game, loaders);
            let release = version.release()?;
            if supports {
                releases.push(release);
            }
        }
        let stable = releases
            .iter()
            .position(|release| release.channel == Channel::Release)
            .unwrap_or(0);
        if releases.is_empty() {
            return Ok(None);
        }
        Ok(Some(releases.remove(stable)))
    }

    pub async fn modrinth_project_of(&self, version: &str) -> Result<Option<String>, SourceError> {
        let request = self
            .http
            .get(format!("{}/version/{version}", self.modrinth));
        let Some(version): Option<Version> = optional(request, Source::Modrinth).await? else {
            return Ok(None);
        };
        Ok(Some(version.release()?.project))
    }

    pub async fn modrinth_projects(
        &self,
        ids: &[String],
    ) -> Result<HashMap<String, ProjectInfo>, SourceError> {
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        let request = self
            .http
            .get(format!("{}/projects", self.modrinth))
            .query(&[("ids", json!(ids).to_string())]);
        let projects: Vec<Project> = required(request, Source::Modrinth).await?;
        let mut found = HashMap::new();
        for project in projects {
            nonempty(&project.id, Source::Modrinth, "project id")?;
            nonempty(&project.title, Source::Modrinth, "project title")?;
            nonempty(&project.slug, Source::Modrinth, "project slug")?;
            let info = ProjectInfo {
                title: project.title,
                icon_url: project.icon_url.filter(|url| !url.is_empty()),
                page_url: Some(format!("https://modrinth.com/project/{}", project.slug)),
            };
            found.insert(project.id, info);
        }
        Ok(found)
    }
}
