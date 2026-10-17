mod discovery;
mod error;
mod extract;
mod install;
mod rules;

pub(crate) use discovery::NativeProbe;
pub use error::JavaError;
pub(crate) use install::{ensure_java_runtime, ArchiveFormat, RuntimeRequest};
pub use rules::get_required_java_version;
pub type JavaProgressFn = std::sync::Arc<dyn Fn(u64, u64, &str) + Send + Sync>;

#[cfg(test)]
mod tests;
