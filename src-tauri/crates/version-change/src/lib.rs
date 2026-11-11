pub(crate) use ingot_artifacts as artifacts;
pub(crate) use ingot_core as core;
pub(crate) use ingot_minecraft as minecraft;
pub(crate) use ingot_server as server;

pub use artifacts::crash;
pub mod dependencies;
pub mod error;
pub mod journal;
pub mod models;
pub mod planner;
pub mod scan;
pub mod service;
pub use artifacts::sources;
pub mod staging;
pub mod targets;
pub mod transaction;

#[cfg(test)]
mod tests;

pub use crash::{first_start_pending, VersionChangeCrash, VersionChangeCrash as CrashReport};
#[cfg(test)]
pub use journal::read_journal;
#[cfg(test)]
pub use journal::Journal;
pub use journal::{
    detect_incomplete_transaction, discard_backup_typed, last_backup, read_journal_state,
    transaction_status, write_journal_typed, JournalState,
};
#[cfg(test)]
pub use models::{backup_dir, work_dir};
pub use models::{
    loader_label, safe_file_name_typed, safe_rel_typed, ContentKind, Folder, ItemAction,
    ItemStatus, PlanItem, PlanRequest, Restored, Target, TargetFile, TargetKind, TransactionPhase,
    VersionBackup, VersionChangeError, VersionPlan, VersionTransactionStatus, WorldInfo,
};
pub use planner::check_typed;
pub use scan::list_worlds;
pub use sources::Source;
#[cfg(test)]
pub use transaction::swap;
pub use transaction::{
    apply_typed, apply_typed_with_commit, recover_interrupted_transaction,
    recover_interrupted_transaction_with_commit, undo_typed, undo_typed_with_commit,
};
