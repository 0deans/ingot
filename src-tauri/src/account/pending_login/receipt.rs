//! Non-secret identity linking one pending grant to its durable account intent.
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GrantReceipt {
    scope: uuid::Uuid,
    ticket: uuid::Uuid,
}

impl GrantReceipt {
    pub(super) fn new(scope: uuid::Uuid, ticket: uuid::Uuid) -> Self {
        Self { scope, ticket }
    }

    pub(crate) fn belongs_to(&self, path: &Path) -> bool {
        !self.ticket.is_nil() && self.scope == scope_id(path)
    }

    pub(super) fn scope(&self) -> uuid::Uuid {
        self.scope
    }

    pub(super) fn ticket(&self) -> uuid::Uuid {
        self.ticket
    }
}

pub(super) fn scope_id(path: &Path) -> uuid::Uuid {
    uuid::Uuid::new_v3(
        &uuid::Uuid::NAMESPACE_URL,
        path.to_string_lossy().as_bytes(),
    )
}
