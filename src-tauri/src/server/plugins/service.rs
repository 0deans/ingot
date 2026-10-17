mod discovery;
mod download;
mod installation;
mod installed;
mod publication;

#[cfg(test)]
pub(crate) use discovery::versions_with_provider;
pub use discovery::{search, versions};
pub use download::download;
#[cfg(test)]
pub(crate) use installation::install_with_provider;
pub use installation::{check_updates, install, restore_missing};
pub use installed::{folder, list_installed, remove, set_enabled};
pub use publication::publish_installed_plugin;
