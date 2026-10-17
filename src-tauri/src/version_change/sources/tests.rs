use std::error::Error;

use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use super::*;

struct Fixture {
    endpoint: String,
    task: JoinHandle<Vec<String>>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Fixture {
    async fn new(responses: Vec<(u16, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let size = stream.read(&mut buffer).await.unwrap();
                    assert!(size > 0);
                    bytes.extend_from_slice(&buffer[..size]);
                    let Some(offset) = bytes.windows(4).position(|window| window == b"\r\n\r\n")
                    else {
                        continue;
                    };
                    let headers = String::from_utf8(bytes[..offset].to_vec()).unwrap();
                    let length = headers
                        .lines()
                        .find_map(|line| {
                            let (name, value) = line.split_once(':')?;
                            name.eq_ignore_ascii_case("content-length")
                                .then(|| value.trim().parse::<usize>().unwrap())
                        })
                        .unwrap_or(0);
                    if bytes.len() < offset + 4 + length {
                        continue;
                    }
                    requests.push(headers.lines().next().unwrap().to_owned());
                    break;
                }
                let response = format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                stream.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        Self { endpoint, task }
    }

    fn sources<'a>(&self, client: &'a reqwest::Client) -> SourceClient<'a> {
        SourceClient::with_endpoints(client, &self.endpoint, &self.endpoint, &self.endpoint)
    }
}

fn modrinth_version(id: &str, channel: &str) -> serde_json::Value {
    json!({ "id": id, "project_id": "project", "version_number": id, "version_type": channel,
        "game_versions": ["1.21"], "loaders": ["fabric"], "dependencies": [],
        "files": [{ "filename": "mod.jar", "url": "https://cdn.modrinth.com/mod.jar", "size": 42, "primary": true, "hashes": {"sha1": "abc"} }] })
}

fn cf_file(id: u32, channel: u32, url: serde_json::Value) -> serde_json::Value {
    json!({ "id": id, "displayName": format!("Version {id}"), "fileName": "mod.jar", "downloadUrl": url,
        "fileLength": 42, "fileDate": format!("2026-01-{id:02}"), "releaseType": channel,
        "gameVersions": ["1.21", "Fabric"], "hashes": [{"algo": 1,"value":"abc"}], "dependencies": [], "fileFingerprint": 123 })
}

#[test]
fn curseforge_fingerprint_ignores_only_protocol_whitespace() {
    assert_eq!(cf_fingerprint(b"a b\tc\r\nd"), cf_fingerprint(b"abcd"));
    assert_eq!(cf_fingerprint(b""), 0x5bd1_5e36);
    assert_ne!(cf_fingerprint(b"abcd"), cf_fingerprint(b"abce"));
}

#[tokio::test]
async fn distinguishes_absence_from_http_and_parse_failures() {
    let client = reqwest::Client::new();
    for (status, body) in [(404, "{}"), (200, "[]")] {
        let fixture = Fixture::new(vec![(status, body.into())]).await;
        assert!(fixture
            .sources(&client)
            .modrinth_target("project", "1.21", &["fabric"])
            .await
            .unwrap()
            .is_none());
    }
    for status in [401, 403, 429, 500] {
        let fixture = Fixture::new(vec![(status, "secret body".into())]).await;
        let error = fixture
            .sources(&client)
            .modrinth_target("project", "1.21", &["fabric"])
            .await
            .unwrap_err();
        assert!(
            matches!(error, SourceError::HttpStatus { provider: Source::Modrinth, status: actual } if actual.as_u16() == status)
        );
        assert!(!format!("{error:?}").contains("secret body"));
    }
    for body in ["not-json", "{}", "[{}]"] {
        let fixture = Fixture::new(vec![(200, body.into())]).await;
        let error = fixture
            .sources(&client)
            .modrinth_target("project", "1.21", &["fabric"])
            .await
            .unwrap_err();
        assert!(matches!(error, SourceError::Decode { .. }));
        assert!(error.source().is_some());
    }
}

#[tokio::test]
async fn transport_failure_preserves_cause_without_request_url() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}/private-token", listener.local_addr().unwrap());
    drop(listener);
    let client = reqwest::Client::new();
    let sources = SourceClient::with_endpoints(&client, &endpoint, &endpoint, &endpoint);
    let error = sources
        .modrinth_target("project", "1.21", &["fabric"])
        .await
        .unwrap_err();
    assert!(matches!(error, SourceError::Transport { .. }));
    assert!(error.source().is_some());
    assert!(!format!("{error:?}").contains("private-token"));
}

#[tokio::test]
async fn modrinth_prefers_stable_and_validates_required_identity() {
    let body = json!([
        modrinth_version("beta", "beta"),
        modrinth_version("stable", "release")
    ])
    .to_string();
    let fixture = Fixture::new(vec![(200, body)]).await;
    let client = reqwest::Client::new();
    let release = fixture
        .sources(&client)
        .modrinth_target("project", "1.21", &["fabric"])
        .await
        .unwrap()
        .unwrap();
    assert_eq!(release.version_id, "stable");
    assert_eq!(release.file.unwrap().bytes, 42.0);
    let mut malformed = modrinth_version("", "release");
    malformed["dependencies"] =
        json!([{ "dependency_type": "required", "project_id": null, "version_id": null }]);
    let fixture = Fixture::new(vec![(200, json!([malformed]).to_string())]).await;
    assert!(matches!(
        fixture
            .sources(&client)
            .modrinth_target("project", "1.21", &["fabric"])
            .await,
        Err(SourceError::InvalidPayload { .. })
    ));
}

#[tokio::test]
async fn curseforge_rejects_broken_envelopes_and_preserves_manual_download_releases() {
    let client = reqwest::Client::new();
    let scope = Scope {
        curseforge: true,
        cf_loader: Some(4),
        cf_loader_names: vec!["Fabric"],
        ..Scope::default()
    };
    for body in ["{}", r#"{"data":{}}"#] {
        let fixture = Fixture::new(vec![(200, body.into())]).await;
        assert!(matches!(
            fixture
                .sources(&client)
                .cf_identify(&[123], "1.21", &scope)
                .await,
            Err(SourceError::Decode {
                provider: Source::Curseforge,
                ..
            })
        ));
    }
    let body = json!({ "data": [cf_file(3, 2, json!("https://example.com/beta.jar")), cf_file(1, 1, serde_json::Value::Null)] }).to_string();
    let fixture = Fixture::new(vec![(200, body)]).await;
    let release = fixture
        .sources(&client)
        .cf_target("42", "1.21", &scope)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(release.channel, Channel::Release);
    assert!(release.file.is_none());
    assert!(matches!(
        fixture
            .sources(&client)
            .cf_target("invalid", "1.21", &scope)
            .await,
        Err(SourceError::InvalidProjectId { .. })
    ));
}

#[tokio::test]
async fn hangar_distinguishes_missing_project_from_invalid_response() {
    let client = reqwest::Client::new();
    let fixture = Fixture::new(vec![(404, "{}".into())]).await;
    assert!(fixture
        .sources(&client)
        .hangar_versions("plugin", "1.21")
        .await
        .unwrap()
        .is_empty());
    let fixture = Fixture::new(vec![(200, "{}".into())]).await;
    assert!(matches!(
        fixture
            .sources(&client)
            .hangar_versions("plugin", "1.21")
            .await,
        Err(SourceError::Decode {
            provider: Source::Hangar,
            ..
        })
    ));
    let body = json!({"result":[{"name":"v1","channel":{"name":"Release"},"downloads":{"PAPER":{"downloadUrl":"https://example.com/plugin.jar","fileInfo":{"name":"plugin.jar","sha256Hash":"abc","sizeBytes":42}}},"pluginDependencies":{"PAPER":[{"required":true,"name":"dependency","externalUrl":null}]}}]}).to_string();
    let fixture = Fixture::new(vec![(200, body)]).await;
    let releases = fixture
        .sources(&client)
        .hangar_versions("plugin", "1.21")
        .await
        .unwrap();
    assert_eq!(releases[0].deps[0].project.as_deref(), Some("dependency"));
    assert_eq!(
        releases[0].file.as_ref().unwrap().sha256.as_deref(),
        Some("abc")
    );
}

#[tokio::test]
async fn bulk_queries_do_not_treat_malformed_or_failed_requests_as_empty_results() {
    let client = reqwest::Client::new();
    let fixture = Fixture::new(vec![(200, "[]".into())]).await;
    assert!(matches!(
        fixture
            .sources(&client)
            .modrinth_identify(&["hash".into()], "1.21", &["fabric"])
            .await,
        Err(SourceError::Decode { .. })
    ));
    let fixture = Fixture::new(vec![(503, "{}".into())]).await;
    assert!(matches!(
        fixture.sources(&client).cf_projects(&["42".into()]).await,
        Err(SourceError::HttpStatus { .. })
    ));
    let fixture = Fixture::new(vec![(200, "{}".into())]).await;
    assert!(matches!(
        fixture
            .sources(&client)
            .modrinth_projects(&["project".into()])
            .await,
        Err(SourceError::Decode { .. })
    ));
}

#[tokio::test]
async fn identification_and_project_metadata_keep_domain_contracts() {
    let client = reqwest::Client::new();
    let body = json!({ "hash": modrinth_version("current", "release") }).to_string();
    let fixture = Fixture::new(vec![(200, body)]).await;
    let identified = fixture
        .sources(&client)
        .modrinth_identify(&["hash".into()], "1.21", &["forge"])
        .await
        .unwrap();
    let (release, compatible) = &identified["hash"];
    assert!(!compatible);
    assert_eq!(release.project, "project");

    let scope = Scope {
        cf_loader_names: vec!["Fabric"],
        ..Scope::default()
    };
    let file = cf_file(1, 1, json!("https://example.com/mod.jar"));
    let body = json!({ "data": {"exactMatches": [{"id": 42, "file": file}]} }).to_string();
    let fixture = Fixture::new(vec![(200, body)]).await;
    let identified = fixture
        .sources(&client)
        .cf_identify(&[123], "1.21", &scope)
        .await
        .unwrap();
    let (release, compatible) = &identified[&123];
    assert!(*compatible);
    assert_eq!(release.project, "42");
    assert_eq!(release.file.as_ref().unwrap().sha1.as_deref(), Some("abc"));

    let body =
        json!([{ "id":"project", "title":"A project", "slug":"a-project", "icon_url":null }])
            .to_string();
    let fixture = Fixture::new(vec![(200, body)]).await;
    let projects = fixture
        .sources(&client)
        .modrinth_projects(&["project".into()])
        .await
        .unwrap();
    assert_eq!(
        projects["project"].page_url.as_deref(),
        Some("https://modrinth.com/project/a-project")
    );

    let body = json!({"data":[{"id":42,"name":"A project","logo":{"thumbnailUrl":"https://example.com/icon.png"},"links":{"websiteUrl":"https://example.com"}}]}).to_string();
    let fixture = Fixture::new(vec![(200, body)]).await;
    let projects = fixture
        .sources(&client)
        .cf_projects(&["42".into()])
        .await
        .unwrap();
    assert_eq!(projects["42"].title, "A project");
}

#[tokio::test]
async fn missing_versions_do_not_hide_provider_failures() {
    let client = reqwest::Client::new();
    let fixture = Fixture::new(vec![(404, "{}".into())]).await;
    assert!(fixture
        .sources(&client)
        .modrinth_project_of("removed")
        .await
        .unwrap()
        .is_none());
    let fixture = Fixture::new(vec![(404, "{}".into())]).await;
    assert!(fixture
        .sources(&client)
        .cf_target("42", "1.21", &Scope::default())
        .await
        .unwrap()
        .is_none());
    let fixture = Fixture::new(vec![(503, "{}".into())]).await;
    assert!(matches!(
        fixture
            .sources(&client)
            .hangar_versions("project", "1.21")
            .await,
        Err(SourceError::HttpStatus {
            provider: Source::Hangar,
            ..
        })
    ));

    let fixture = Fixture::new(vec![(200, r#"{"data":[{"id":"provider-secret"}]}"#.into())]).await;
    let error = fixture
        .sources(&client)
        .cf_target("42", "1.21", &Scope::default())
        .await
        .unwrap_err();
    assert!(matches!(error, SourceError::Decode { .. }));
    assert!(!format!("{error:?}").contains("provider-secret"));
}

#[tokio::test]
async fn planner_propagates_identification_target_dependency_and_metadata_failures() {
    use crate::version_change::{
        models::sha1_hex, planner::check_with_sources, targets, PlanRequest, TargetKind,
        VersionChangeError,
    };

    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir(directory.path().join("mods")).unwrap();
    std::fs::write(directory.path().join("mods/mod.jar"), b"installed").unwrap();
    let hash = sha1_hex(b"installed");
    let target = targets::instance(directory.path(), "fabric", "fabric");
    let current = modrinth_version("current", "release");
    let identified = json!({ (hash.clone()): current }).to_string();
    let mut old = modrinth_version("old", "release");
    old["game_versions"] = json!(["1.20"]);
    let mut needs_dependency = modrinth_version("current", "release");
    needs_dependency["dependencies"] =
        json!([{ "dependency_type": "required", "project_id": "dependency", "version_id": null }]);
    let cf_empty = json!({ "data": { "exactMatches": [] } }).to_string();
    let stages = vec![
        vec![(503, "{}".into())],
        vec![(200, "{}".into()), (503, "{}".into())],
        vec![
            (200, json!({ (hash.clone()): old }).to_string()),
            (200, cf_empty.clone()),
            (503, "{}".into()),
        ],
        vec![
            (200, json!({ (hash.clone()): needs_dependency }).to_string()),
            (200, cf_empty.clone()),
            (503, "{}".into()),
        ],
        vec![(200, identified), (200, cf_empty), (503, "{}".into())],
    ];
    let client = reqwest::Client::new();
    for responses in stages {
        let fixture = Fixture::new(responses).await;
        let request = PlanRequest {
            target_kind: TargetKind::Instance,
            target_id: "test".into(),
            from_game_version: "1.20".into(),
            from_loader: "fabric".into(),
            from_loader_version: None,
            to_game_version: "1.21".into(),
            to_loader: "fabric".into(),
            to_loader_version: None,
            downgrade: false,
        };
        let result = check_with_sources(&fixture.sources(&client), &target, request).await;
        assert!(matches!(
            result,
            Err(VersionChangeError::Source(SourceError::HttpStatus { .. }))
        ));
    }
}
