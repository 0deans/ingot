//! Binary VDF codec and non-Steam shortcut management for Steam `shortcuts.vdf`.

mod codec;
mod shortcuts;

pub use codec::{VdfEntry, VdfValue};
pub(super) use shortcuts::upsert_shortcuts_file;
pub use shortcuts::{
    compute_steam_app_id, crc32_ieee, parse_shortcuts_vdf, quote_steam_path,
    serialize_shortcuts_vdf, upsert_steam_shortcut, SteamShortcutSpec,
};
