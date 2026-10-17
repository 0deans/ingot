use std::path::{Path, PathBuf};

/// Returns the platform-default classpath separator (`;` on Windows, `:` on Unix).
pub fn default_classpath_separator() -> &'static str {
    if cfg!(target_os = "windows") {
        ";"
    } else {
        ":"
    }
}

/// Converts a Maven coordinate like `com.mojang:authlib:1.5.25` into a relative file path.
pub fn maven_coordinate_to_path(name: &str, classifier: Option<&str>) -> Option<String> {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let class_suffix = classifier.map(|c| format!("-{}", c)).unwrap_or_default();
    let ext = parts.get(3).unwrap_or(&"jar");

    Some(format!(
        "{}/{}/{}/{}-{}{}.{}",
        group, artifact, version, artifact, version, class_suffix, ext
    ))
}

/// Resolves the absolute disk destination for a library artifact.
pub fn resolve_library_path(
    name: &str,
    artifact_path: Option<&str>,
    classifier: Option<&str>,
    libraries_dir: &Path,
) -> Option<PathBuf> {
    let rel = artifact_path
        .map(|s| s.to_string())
        .or_else(|| maven_coordinate_to_path(name, classifier))?;
    Some(libraries_dir.join(rel))
}

/// Formats a sequence of paths into a classpath string using the specified separator.
pub fn build_classpath_string<'a, I>(entries: I, separator: &str) -> String
where
    I: IntoIterator<Item = &'a Path>,
{
    entries
        .into_iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<String>>()
        .join(separator)
}

/// Assembles the complete runtime classpath by appending the client JAR to library paths.
pub fn assemble_classpath(
    library_entries: &[PathBuf],
    client_jar: Option<&Path>,
    separator: &str,
) -> String {
    let mut all: Vec<&Path> = library_entries.iter().map(|p| p.as_path()).collect();
    if let Some(jar) = client_jar {
        all.push(jar);
    }
    build_classpath_string(all, separator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_classpath_with_custom_separator() {
        let p1 = PathBuf::from("/lib/a.jar");
        let p2 = PathBuf::from("/lib/b.jar");
        let formatted = build_classpath_string([p1.as_path(), p2.as_path()], ":");
        assert_eq!(formatted, "/lib/a.jar:/lib/b.jar");
    }

    #[test]
    fn assemble_classpath_appends_client_jar_at_the_end() {
        let p1 = PathBuf::from("/lib/a.jar");
        let client = PathBuf::from("/client.jar");
        let cp = assemble_classpath(&[p1], Some(&client), ";");
        assert_eq!(cp, "/lib/a.jar;/client.jar");
    }

    #[test]
    fn maven_coordinate_to_path_converts_standard_coordinates() {
        let path = maven_coordinate_to_path("com.mojang:authlib:1.5.25", None).unwrap();
        assert_eq!(path, "com/mojang/authlib/1.5.25/authlib-1.5.25.jar");
    }

    #[test]
    fn maven_coordinate_to_path_handles_classifier_and_extension() {
        let path = maven_coordinate_to_path(
            "org.lwjgl:lwjgl:3.3.1:natives-windows:jar",
            Some("natives-windows"),
        );
        assert!(path.is_some());
        let p = path.unwrap();
        assert!(p.contains("natives-windows"));
    }

    #[test]
    fn maven_coordinate_to_path_returns_none_for_malformed_input() {
        assert!(maven_coordinate_to_path("invalid-coord", None).is_none());
    }

    #[test]
    fn resolve_library_path_prefers_artifact_path() {
        let lib_dir = Path::new("/minecraft/libraries");
        let resolved = resolve_library_path(
            "com.example:lib:1.0",
            Some("override/path/custom.jar"),
            None,
            lib_dir,
        )
        .unwrap();
        assert_eq!(resolved, lib_dir.join("override/path/custom.jar"));
    }
}
