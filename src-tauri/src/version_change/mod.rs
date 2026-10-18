//! Moving an instance or a server to another Minecraft version (or loader) without
//! breaking it.
//!
//! 1. `check` changes nothing: it identifies every mod, plugin, resource pack and shader
//!    (Modrinth by hash, then CurseForge by fingerprint, and Hangar for plugins Ingot
//!    installed) and works out, for the target, what still works, what has an update,
//!    what's missing, which new dependencies are needed and what conflicts.
//! 2. `apply` carries out the reviewed plan all or nothing: new files are downloaded and
//!    verified in a staging folder first, the current files, configs (and worlds, if
//!    asked) are snapshotted, and only then is everything swapped in. A failure part way
//!    puts everything back.
//! 3. `undo` restores the snapshot of the last change exactly.
//! 4. `crash` watches the first start after a change and names the likely culprits.

pub mod crash;
pub mod dependencies;
pub mod journal;
pub mod models;
pub mod planner;
pub mod scan;
pub mod sources;
pub mod staging;
pub mod targets;
pub mod transaction;

#[cfg(test)]
mod tests;

pub use crash::{first_start_pending, VersionChangeCrash, VersionChangeCrash as CrashReport};
pub(crate) use journal::read_journal;
#[cfg(test)]
pub(crate) use journal::write_journal;
#[cfg(test)]
pub(crate) use journal::Journal;
pub use journal::{
    detect_incomplete_transaction, discard_backup, discard_backup_typed, last_backup,
    read_journal_state, transaction_status, write_journal_typed, JournalState,
};
#[cfg(test)]
pub(crate) use models::{backup_dir, safe_file_name, safe_rel, work_dir};
pub use models::{
    loader_label, safe_file_name_typed, safe_rel_typed, ContentKind, Folder, ItemAction,
    ItemStatus, PlanItem, PlanRequest, Restored, Target, TargetFile, TargetKind, TransactionPhase,
    VersionBackup, VersionChangeError, VersionPlan, VersionTransactionStatus, WorldInfo,
};
pub use planner::{check, check_typed};
pub use scan::list_worlds;
pub use sources::Source;
#[cfg(test)]
pub(crate) use transaction::swap;
pub use transaction::{
    apply, apply_typed, apply_typed_with_commit, apply_with_commit,
    recover_interrupted_transaction, recover_interrupted_transaction_with_commit, undo, undo_typed,
    undo_typed_with_commit, undo_with_commit,
};

pub(crate) mod service;
