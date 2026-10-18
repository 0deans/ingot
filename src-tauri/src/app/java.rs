use crate::minecraft::java::{
    ArchiveFormat, JavaError, JavaProgressFn, NativeProbe, RuntimeRequest,
};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Manager, Runtime};

pub(crate) async fn ensure_java_runtime<R: Runtime>(
    app: &tauri::AppHandle<R>,
    client: &reqwest::Client,
    major_version: u32,
    custom_path: Option<&str>,
    progress: Option<JavaProgressFn>,
) -> Result<PathBuf, JavaError> {
    let data_dir = app.path().app_data_dir().map_err(std::io::Error::other)?;
    ensure_java_runtime_in_dir(data_dir, client, major_version, custom_path, progress).await
}

pub(crate) async fn ensure_java_runtime_in_dir(
    data_dir: PathBuf,
    client: &reqwest::Client,
    major_version: u32,
    custom_path: Option<&str>,
    progress: Option<JavaProgressFn>,
) -> Result<PathBuf, JavaError> {
    let (os, archive_format) = if cfg!(target_os = "windows") {
        ("windows", ArchiveFormat::Zip)
    } else if cfg!(target_os = "macos") {
        ("mac", ArchiveFormat::TarGzip)
    } else {
        ("linux", ArchiveFormat::TarGzip)
    };
    let arch = if cfg!(any(target_arch = "aarch64", target_arch = "arm")) {
        "aarch64"
    } else {
        "x64"
    };
    let download_url = format!(
        "https://api.adoptium.net/v3/binary/latest/{major_version}/ga/{os}/{arch}/jdk/hotspot/normal/eclipse"
    );
    let request = RuntimeRequest {
        data_dir,
        major_version,
        custom_path: custom_path.map(PathBuf::from),
        system_roots: system_roots(),
        download_url,
        archive_format,
    };
    crate::minecraft::java::ensure_java_runtime(
        request,
        client,
        Arc::new(NativeProbe::new()),
        progress,
    )
    .await
}

fn system_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(java_home) = std::env::var_os("JAVA_HOME") {
        roots.push(PathBuf::from(java_home));
    }
    #[cfg(target_os = "windows")]
    roots.extend([
        PathBuf::from("C:\\Program Files\\Eclipse Adoptium"),
        PathBuf::from("C:\\Program Files\\Java"),
        PathBuf::from("C:\\Program Files\\Microsoft"),
        PathBuf::from("C:\\Program Files\\BellSoft"),
        PathBuf::from("C:\\Program Files\\Amazon Corretto"),
    ]);
    roots
}
