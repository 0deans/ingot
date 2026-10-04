//! Export procedure metadata without spawning native RPC handlers.
use crate::ipc::{AppApi, AppApiImpl};
#[test]
#[ignore = "Explicit command to regenerate committed TypeScript bindings"]
fn export_bindings() {
    let output = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src/bindings.ts");
    let router = AppApiImpl.into_handler();
    taurpc::Exporter::new().export(&router, output).unwrap();
}

#[test]
fn generated_bindings_match_the_committed_contract() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("bindings.ts");
    let router = AppApiImpl.into_handler();
    taurpc::Exporter::new().export(&router, &output).unwrap();
    let actual = std::fs::read_to_string(output)
        .unwrap()
        .replace("\r\n", "\n");
    let expected = include_str!("../../src/bindings.ts").replace("\r\n", "\n");
    assert_inventory(&actual);
    assert_eq!(
        actual, expected,
        "Regenerate bindings: cargo test --lib --locked ipc_contract::export_bindings -- --ignored --exact"
    );
}

fn assert_inventory(bindings: &str) {
    let arguments_line = bindings
        .lines()
        .find_map(|line| {
            line.strip_prefix("const ARGS_MAP = ")
                .and_then(|json| json.strip_suffix(';'))
        })
        .expect("TauRPC argument map must be exported");
    let arguments: serde_json::Value = serde_json::from_str(arguments_line).unwrap();
    let inventory: serde_json::Value =
        serde_json::from_str(include_str!("../tests/fixtures/ipc-inventory.json")).unwrap();
    let mut recorded = serde_json::Map::new();
    for procedure in inventory["procedures"].as_array().unwrap() {
        let name = procedure["name"].as_str().unwrap();
        assert!(!procedure["destination"].as_str().unwrap().is_empty());
        assert!(
            recorded
                .insert(name.to_owned(), procedure["arguments"].clone())
                .is_none(),
            "Duplicate inventory entry: {name}"
        );
    }
    assert_eq!(
        serde_json::Value::Object(recorded),
        arguments[""],
        "Update the procedure inventory during migration"
    );
}
