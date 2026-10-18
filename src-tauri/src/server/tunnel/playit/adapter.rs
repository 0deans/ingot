use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::sync::watch;

use super::PlayitError;

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub(crate) trait PlayitChild: Send + Sync {
    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, std::io::Error>;
    fn kill<'a>(&'a mut self) -> BoxFuture<'a, Result<(), std::io::Error>>;
    fn wait<'a>(&'a mut self) -> BoxFuture<'a, Result<std::process::ExitStatus, std::io::Error>>;
    fn take_stdout(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Unpin + Send>>;
    fn take_stderr(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Unpin + Send>>;
}

pub(crate) struct NativeChild(pub(crate) tokio::process::Child);

impl PlayitChild for NativeChild {
    fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, std::io::Error> {
        self.0.try_wait()
    }

    fn kill<'a>(&'a mut self) -> BoxFuture<'a, Result<(), std::io::Error>> {
        Box::pin(async move { self.0.kill().await })
    }

    fn wait<'a>(&'a mut self) -> BoxFuture<'a, Result<std::process::ExitStatus, std::io::Error>> {
        Box::pin(async move { self.0.wait().await })
    }

    fn take_stdout(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.0
            .stdout
            .take()
            .map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>)
    }

    fn take_stderr(&mut self) -> Option<Box<dyn tokio::io::AsyncRead + Unpin + Send>> {
        self.0
            .stderr
            .take()
            .map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>)
    }
}

pub(crate) trait PlayitAdapter: Send + Sync {
    fn ensure_binary<'a>(
        &'a self,
        app_data_dir: &'a Path,
        client: &'a reqwest::Client,
        cancel_rx: watch::Receiver<bool>,
    ) -> BoxFuture<'a, Result<PathBuf, PlayitError>>;

    fn claim_setup<'a>(
        &'a self,
        client: &'a reqwest::Client,
        code: &'a str,
    ) -> BoxFuture<'a, Result<String, PlayitError>>;

    fn claim_exchange<'a>(
        &'a self,
        client: &'a reqwest::Client,
        code: &'a str,
    ) -> BoxFuture<'a, Result<String, PlayitError>>;

    fn spawn_process(
        &self,
        binary_path: &Path,
        secret: &str,
        playit_dir: &Path,
        sandbox_dir: Option<&Path>,
    ) -> Result<Box<dyn PlayitChild>, PlayitError>;

    fn fetch_rundata<'a>(
        &'a self,
        client: &'a reqwest::Client,
        secret: &'a str,
    ) -> BoxFuture<'a, Result<serde_json::Value, PlayitError>>;
}

#[derive(Default)]
pub(crate) struct NativePlayitAdapter;

impl PlayitAdapter for NativePlayitAdapter {
    fn ensure_binary<'a>(
        &'a self,
        app_data_dir: &'a Path,
        client: &'a reqwest::Client,
        cancel_rx: watch::Receiver<bool>,
    ) -> BoxFuture<'a, Result<PathBuf, PlayitError>> {
        Box::pin(super::binary::ensure_playit_binary_with_dir(
            app_data_dir,
            client,
            cancel_rx,
        ))
    }

    fn claim_setup<'a>(
        &'a self,
        client: &'a reqwest::Client,
        code: &'a str,
    ) -> BoxFuture<'a, Result<String, PlayitError>> {
        Box::pin(async move {
            let version = format!("ingot {}", env!("CARGO_PKG_VERSION"));
            let body = serde_json::json!({
                "code": code,
                "agent_type": "self-managed",
                "version": version
            });
            let data = super::api::api_call(client, "/claim/setup", body, None).await?;
            data.as_str()
                .map(str::to_string)
                .ok_or(PlayitError::InvalidResponse)
        })
    }

    fn claim_exchange<'a>(
        &'a self,
        client: &'a reqwest::Client,
        code: &'a str,
    ) -> BoxFuture<'a, Result<String, PlayitError>> {
        Box::pin(async move {
            let body = serde_json::json!({ "code": code });
            let data = super::api::api_call(client, "/claim/exchange", body, None).await?;
            if let Some(secret) = data["secret_key"].as_str() {
                return Ok(secret.to_string());
            }
            Err(PlayitError::InvalidResponse)
        })
    }

    fn spawn_process(
        &self,
        binary_path: &Path,
        secret: &str,
        playit_dir: &Path,
        sandbox_dir: Option<&Path>,
    ) -> Result<Box<dyn PlayitChild>, PlayitError> {
        let mut cmd =
            super::process::build_agent_command(binary_path, secret, playit_dir, sandbox_dir)?;
        cmd.stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);

        #[cfg(windows)]
        {
            cmd.creation_flags(0x08000000);
        }

        let child = cmd.spawn()?;
        Ok(Box::new(NativeChild(child)))
    }

    fn fetch_rundata<'a>(
        &'a self,
        client: &'a reqwest::Client,
        secret: &'a str,
    ) -> BoxFuture<'a, Result<serde_json::Value, PlayitError>> {
        Box::pin(async move {
            super::api::api_call(
                client,
                "/v1/agents/rundata",
                serde_json::json!({}),
                Some(secret),
            )
            .await
        })
    }
}
