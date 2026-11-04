use std::path::Path;

use super::error::InstanceContentError;
use super::models::ContentCategory;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentFileName(String);

impl ContentFileName {
    pub fn parse(raw: &str, category: ContentCategory) -> Result<Self, InstanceContentError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(InstanceContentError::invalid_file_name(
                raw,
                "File name cannot be empty",
            ));
        }

        if trimmed.starts_with('.') {
            return Err(InstanceContentError::invalid_file_name(
                raw,
                "Hidden files are not allowed",
            ));
        }

        if trimmed.contains(['/', '\\', ':']) || trimmed.contains("..") {
            return Err(InstanceContentError::invalid_file_name(
                raw,
                "Path traversal and directory separators are not allowed",
            ));
        }

        if Path::new(trimmed).file_name().and_then(|f| f.to_str()) != Some(trimmed) {
            return Err(InstanceContentError::invalid_file_name(
                raw,
                "Not a valid single file or folder name",
            ));
        }

        let base = trimmed.strip_suffix(".disabled").unwrap_or(trimmed);
        let base_lower = base.to_ascii_lowercase();

        match category {
            ContentCategory::Mod => {
                if !base_lower.ends_with(".jar") {
                    return Err(InstanceContentError::invalid_file_name(
                        raw,
                        "Mod files must end with .jar or .jar.disabled",
                    ));
                }
            }
            ContentCategory::ResourcePack | ContentCategory::ShaderPack => {
                let is_zip = base_lower.ends_with(".zip");
                let has_ext = base.contains('.');
                // Resource packs and shaders can be .zip files or directory folders (without extension or arbitrary folder name)
                if has_ext && !is_zip {
                    return Err(InstanceContentError::invalid_file_name(
                        raw,
                        "Pack archives must end with .zip or be unpacked folders",
                    ));
                }
            }
            ContentCategory::DataPack => {
                let is_zip = base_lower.ends_with(".zip");
                let has_ext = base.contains('.');
                if has_ext && !is_zip {
                    return Err(InstanceContentError::invalid_file_name(
                        raw,
                        "Datapack archives must end with .zip or be unpacked folders",
                    ));
                }
            }
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_disabled(&self) -> bool {
        self.0.ends_with(".disabled")
    }

    pub fn base_name(&self) -> &str {
        self.0.strip_suffix(".disabled").unwrap_or(&self.0)
    }

    pub fn with_enabled_state(&self, enabled: bool) -> Self {
        if enabled {
            Self(self.base_name().to_string())
        } else if self.is_disabled() {
            self.clone()
        } else {
            Self(format!("{}.disabled", self.0))
        }
    }
}

impl AsRef<str> for ContentFileName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ContentFileName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
