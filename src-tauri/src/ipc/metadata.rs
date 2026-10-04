use std::collections::BTreeMap;

use specta::Types;
use taurpc::{Exportable, TauRpcFunction, TauRpcHandler};

type ExportedTypes = (
    Types,
    BTreeMap<String, Vec<TauRpcFunction>>,
    BTreeMap<String, String>,
);

/// Mirrors TauRPC's metadata collection, without its handler-spawning effect.
#[derive(Default)]
pub(super) struct ApiMetadata {
    types: Types,
    functions: BTreeMap<String, Vec<TauRpcFunction>>,
    arguments: BTreeMap<String, String>,
}

impl ApiMetadata {
    pub(super) fn register<R: tauri::Runtime, H: TauRpcHandler<R>>(&mut self, _handler: H) {
        let path = H::PATH_PREFIX.to_owned();
        // A duplicate in the static registration list is a programming error.
        assert!(
            !self.functions.contains_key(&path),
            "Duplicate TauRPC path: {path}"
        );
        self.functions
            .insert(path.clone(), H::collect_fn_types(&mut self.types));
        self.arguments.insert(path, H::args_map());
    }
}

impl Exportable for ApiMetadata {
    fn generate_types(&self) -> ExportedTypes {
        (
            self.types.clone(),
            self.functions.clone(),
            self.arguments.clone(),
        )
    }
}
