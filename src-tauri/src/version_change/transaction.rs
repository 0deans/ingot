mod apply;
mod paths;
mod recovery;
mod restoration_files;
mod swap;

pub(super) use paths::checked_path;
#[cfg(test)]
mod path_tests;

pub use apply::{apply, apply_typed, apply_typed_with_commit, apply_with_commit};
pub use paths::{copy_path, copy_path_contained, move_path, remove_path};
#[cfg(test)]
pub(crate) use paths::{is_contained, strip_contained_root};
pub use recovery::{
    recover_interrupted_transaction, recover_interrupted_transaction_with_commit, undo, undo_typed,
    undo_typed_with_commit, undo_with_commit,
};
pub use restoration_files::restore;
pub use swap::swap;
