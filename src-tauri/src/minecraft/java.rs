mod discovery;
mod error;
mod extract;
mod install;
mod probe;
mod rules;

pub use error::JavaError;
pub(crate) use install::{ensure_java_runtime, ArchiveFormat, RuntimeRequest};
pub(crate) use probe::NativeProbe;
pub use rules::get_required_java_version;
pub type JavaProgressFn = std::sync::Arc<dyn Fn(u64, u64, &str) + Send + Sync>;

#[cfg(test)]
mod probe_tests;
#[cfg(test)]
mod tests;
