pub mod archive;
pub mod fs;
pub mod metadata;
pub mod paths;
pub mod process;
pub mod process_tree;

pub use archive::{extract_zip_entries, ArchiveError};
pub use fs::{
    copy_file_atomic, is_link_or_reparse_point, persist_staged_file, write_file_atomic,
    write_file_atomic_with_parent_sync,
};
pub use metadata::{DeletionFiles, MetadataError, MetadataRepository};
pub use paths::{safe_enclosed_filename, safe_enclosed_path};
pub use process::{is_alive, process_start, Entry};

/// User agent for all HTTP requests; the version comes from Cargo.toml at build time.
pub const USER_AGENT: &str = concat!(
    "Ingot-Launcher/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/0deans/ingot)"
);
