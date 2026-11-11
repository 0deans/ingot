//! Archive and directory orchestration for extracting mod and pack content metadata.

mod parsers;

use std::fs::File;
use std::io::Read;
use std::path::Path;

use self::parsers::{
    encode_png_data_url, pack_name_from_file_name, parse_bukkit_plugin_yml, parse_fabric_mod_json,
    parse_forge_mods_toml, parse_mcmod_info, parse_pack_mcmeta_description, parse_quilt_mod_json,
    ManifestParse, MAX_ICON_BYTES,
};
use super::models::ContentMetadata;

pub fn read_content_metadata(path: &Path, is_pack: bool) -> Option<ContentMetadata> {
    if is_pack {
        read_pack_metadata(path)
    } else {
        read_jar_metadata(path)
    }
}

fn read_zip_text<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &str,
) -> Option<String> {
    let mut entry = zip.by_name(name).ok()?;
    let mut text = String::new();
    entry.read_to_string(&mut text).ok()?;
    Some(text)
}

fn read_zip_icon<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    icon_path: &str,
) -> Option<String> {
    let clean = icon_path.trim_start_matches('/');
    let mut entry = zip.by_name(clean).ok()?;
    if entry.size() > MAX_ICON_BYTES as u64 {
        return None;
    }
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes).ok()?;
    encode_png_data_url(&bytes)
}

fn apply_manifest_outcome<R: Read + std::io::Seek>(
    zip: &mut zip::ZipArchive<R>,
    outcome: ManifestParse,
) -> Option<Option<ContentMetadata>> {
    match outcome {
        ManifestParse::Parsed(mut parsed) => {
            parsed.metadata.icon_data_url = parsed
                .icon_candidates
                .iter()
                .find_map(|candidate| read_zip_icon(zip, candidate));
            Some(Some(parsed.metadata))
        }
        ManifestParse::MissingRequired => Some(None),
        ManifestParse::NotMatched => None,
    }
}

fn read_jar_metadata(path: &Path) -> Option<ContentMetadata> {
    let file = File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(file).ok()?;

    // 1. Fabric mod
    if let Some(text) = read_zip_text(&mut zip, "fabric.mod.json") {
        if let Some(result) = apply_manifest_outcome(&mut zip, parse_fabric_mod_json(&text)) {
            return result;
        }
    }

    // 2. Quilt mod
    if let Some(text) = read_zip_text(&mut zip, "quilt.mod.json") {
        if let Some(result) = apply_manifest_outcome(&mut zip, parse_quilt_mod_json(&text)) {
            return result;
        }
    }

    // 3. Forge (mods.toml) / NeoForge (neoforge.mods.toml)
    if let Some(toml_text) = read_zip_text(&mut zip, "META-INF/neoforge.mods.toml")
        .or_else(|| read_zip_text(&mut zip, "META-INF/mods.toml"))
    {
        let manifest_mf = read_zip_text(&mut zip, "META-INF/MANIFEST.MF");
        if let Some(result) = apply_manifest_outcome(
            &mut zip,
            parse_forge_mods_toml(&toml_text, manifest_mf.as_deref()),
        ) {
            return result;
        }
    }

    // 4. Legacy Forge mcmod.info
    if let Some(text) = read_zip_text(&mut zip, "mcmod.info") {
        if let Some(result) = apply_manifest_outcome(&mut zip, parse_mcmod_info(&text)) {
            return result;
        }
    }

    // 5. Bukkit / Spigot / Paper plugins
    let yaml = read_zip_text(&mut zip, "paper-plugin.yml")
        .or_else(|| read_zip_text(&mut zip, "plugin.yml"))?;
    parse_bukkit_plugin_yml(&yaml)
}

fn read_pack_metadata(path: &Path) -> Option<ContentMetadata> {
    let (description, icon_data_url, strip_zip_suffix) = if path.is_dir() {
        let mcmeta_text = std::fs::read_to_string(path.join("pack.mcmeta")).ok()?;
        let description = parse_pack_mcmeta_description(&mcmeta_text)?;
        let icon_data_url = std::fs::read(path.join("pack.png"))
            .ok()
            .and_then(|bytes| encode_png_data_url(&bytes));
        (description, icon_data_url, false)
    } else {
        let file = File::open(path).ok()?;
        let mut zip = zip::ZipArchive::new(file).ok()?;
        let mcmeta_text = read_zip_text(&mut zip, "pack.mcmeta")?;
        let description = parse_pack_mcmeta_description(&mcmeta_text)?;
        let icon_data_url = read_zip_icon(&mut zip, "pack.png");
        (description, icon_data_url, true)
    };

    let raw_name = path.file_name().map(|n| n.to_string_lossy().into_owned());
    let name = pack_name_from_file_name(raw_name.as_deref(), strip_zip_suffix);

    Some(ContentMetadata {
        name,
        mod_id: None,
        version: None,
        description,
        authors: Vec::new(),
        icon_data_url,
        dependencies: Vec::new(),
    })
}
