//! Instance icon resolution, built-in preset rendering, and format conversion for OS and Steam shortcuts.

mod convert;
mod placement;
mod presets;

pub use convert::{get_default_loader_icon_ico, get_default_loader_icon_png, png_to_ico};
pub use placement::{resolve_instance_icon, resolve_instance_icon_bytes};
pub use presets::render_preset_icon_png;
