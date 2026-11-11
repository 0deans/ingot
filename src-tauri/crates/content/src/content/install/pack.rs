//! Modpack format detection, planning, and staging orchestration.
pub(crate) use super::support::PreparedModpack;
use super::{
    atlauncher_pack, curseforge_pack,
    curseforge_resolver::{CurseForgeEndpoints, DEFAULT_CURSEFORGE_CDN_BASE},
    ftb_pack, modrinth_pack, support, technic_pack,
};
use crate::content::error::ContentError;
use ingot_minecraft::launcher::progress::ProgressCallback;
use std::{io::Cursor, path::Path};

enum Downloads {
    Modrinth(Vec<super::ModrinthIndexFile>),
    CurseForge(Vec<super::CurseForgeManifestFile>),
    Ftb(Vec<ftb_pack::FtbVersionFile>),
    TechnicSolder(Vec<technic_pack::SolderMod>),
    AtLauncher(Vec<atlauncher_pack::AtLauncherMod>),
    None,
}

struct Plan {
    prepared: PreparedModpack,
    downloads: Downloads,
}

fn plan(
    root: &Path,
    bytes: Vec<u8>,
    source: &str,
    filename: &str,
    requested_name: &str,
    on_progress: Option<&ProgressCallback>,
) -> Result<Plan, ContentError> {
    let src = source.trim().to_lowercase();

    if src == "ftb" || filename.starts_with("ftb-") {
        let (prepared, files) = ftb_pack::plan_ftb(root, &bytes, requested_name)?;
        return Ok(Plan {
            prepared,
            downloads: Downloads::Ftb(files),
        });
    }

    if src == "technic" {
        let (prepared, solder_mods) =
            technic_pack::plan_technic(root, bytes, filename, requested_name, on_progress)?;
        let downloads = match solder_mods {
            Some(mods) => Downloads::TechnicSolder(mods),
            None => Downloads::None,
        };
        return Ok(Plan {
            prepared,
            downloads,
        });
    }

    if src == "atlauncher" || src == "atl" {
        let (prepared, atl_mods) = atlauncher_pack::plan_atlauncher(root, bytes, requested_name)?;
        let downloads = match atl_mods {
            Some(mods) => Downloads::AtLauncher(mods),
            None => Downloads::None,
        };
        return Ok(Plan {
            prepared,
            downloads,
        });
    }

    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(ContentError::Zip)?;
    let is_modrinth = src == "modrinth"
        || filename.ends_with(".mrpack")
        || zip.by_name("modrinth.index.json").is_ok();
    if is_modrinth {
        let (prepared, files) = modrinth_pack::plan_modrinth(root, zip, requested_name)?;
        return Ok(Plan {
            prepared,
            downloads: Downloads::Modrinth(files),
        });
    }

    let (prepared, files) = curseforge_pack::plan_curseforge(root, zip, requested_name)?;
    Ok(Plan {
        prepared,
        downloads: Downloads::CurseForge(files),
    })
}

pub(crate) struct PrepareModpackParams<'a> {
    pub(crate) client: &'a reqwest::Client,
    pub(crate) curseforge_base: &'a str,
    pub(crate) root: &'a Path,
    pub(crate) bytes: Vec<u8>,
    pub(crate) source: &'a str,
    pub(crate) filename: &'a str,
    pub(crate) name: &'a str,
    pub(crate) manifest_url: Option<&'a str>,
    pub(crate) on_progress: Option<&'a ProgressCallback>,
}

pub(crate) async fn prepare_modpack(
    params: PrepareModpackParams<'_>,
) -> Result<PreparedModpack, ContentError> {
    prepare_modpack_with_cdn(params, DEFAULT_CURSEFORGE_CDN_BASE).await
}

pub(super) async fn prepare_modpack_with_cdn(
    params: PrepareModpackParams<'_>,
    curseforge_cdn_base: &str,
) -> Result<PreparedModpack, ContentError> {
    let client = params.client;
    let endpoints = CurseForgeEndpoints {
        api_base: params.curseforge_base,
        cdn_base: curseforge_cdn_base,
    };
    let manifest_url = params.manifest_url;
    let on_progress = params.on_progress;
    let root = params.root.to_owned();
    let source = params.source.to_owned();
    let filename = params.filename.to_owned();
    let name = params.name.to_owned();
    let bytes = params.bytes;
    let on_progress_plan = on_progress.cloned();
    let mut plan = tokio::task::spawn_blocking(move || {
        plan(
            &root,
            bytes,
            &source,
            &filename,
            &name,
            on_progress_plan.as_ref(),
        )
    })
    .await
    .map_err(ContentError::Task)??;

    let result = async {
        match plan.downloads {
            Downloads::Modrinth(files) => {
                modrinth_pack::download_modrinth_files(
                    client,
                    &plan.prepared.stage,
                    files,
                    on_progress,
                )
                .await?;
            }
            Downloads::CurseForge(files) => {
                curseforge_pack::download_curseforge_files(
                    client,
                    endpoints,
                    &plan.prepared.stage,
                    files,
                    on_progress,
                )
                .await?;
            }
            Downloads::Ftb(files) => {
                ftb_pack::download_ftb_files(
                    client,
                    endpoints,
                    &plan.prepared.stage,
                    files,
                    on_progress,
                )
                .await?;
            }
            Downloads::TechnicSolder(mods) => {
                let (gv, ldr, lv) = technic_pack::download_solder_mods(
                    client,
                    &plan.prepared.stage,
                    mods,
                    &plan.prepared.game_version,
                    on_progress,
                )
                .await?;
                plan.prepared.game_version = gv;
                plan.prepared.loader = ldr;
                plan.prepared.loader_version = lv;
            }
            Downloads::AtLauncher(mods) => {
                atlauncher_pack::download_atlauncher_mods(
                    client,
                    endpoints,
                    manifest_url,
                    &plan.prepared.stage,
                    mods,
                    on_progress,
                )
                .await?;
            }
            Downloads::None => {
                support::report_finalizing(on_progress, 1);
            }
        }
        Ok(())
    }
    .await;

    if let Err(primary) = result {
        return Err(support::cleanup_stage(plan.prepared.stage, primary));
    }
    Ok(plan.prepared)
}
