use super::execute::{
    backup_file_if_exists, copy_file_atomic, file_exists, sync_file_bidirectional,
    sync_resourcepacks_dir,
};
use super::models::{SyncCategory, SyncError, SyncReport};
use super::policy::EffectiveSync;
use std::fs;
use std::path::Path;
pub fn execute_sync_before_launch(
    shared: &Path,
    instance_dir: &Path,
    eff: &EffectiveSync,
) -> Result<(), SyncError> {
    if eff.options {
        sync_file_bidirectional(
            &shared.join("options.txt"),
            &instance_dir.join("options.txt"),
        )?;
    }
    if eff.servers {
        sync_file_bidirectional(
            &shared.join("servers.dat"),
            &instance_dir.join("servers.dat"),
        )?;
    }
    if eff.command_history {
        sync_file_bidirectional(
            &shared.join("command_history.txt"),
            &instance_dir.join("command_history.txt"),
        )?;
    }
    if eff.creative_hotbars {
        sync_file_bidirectional(&shared.join("hotbar.nbt"), &instance_dir.join("hotbar.nbt"))?;
    }
    if eff.resource_packs {
        let shared_packs = shared.join("resourcepacks");
        let instance_packs = instance_dir.join("resourcepacks");
        fs::create_dir_all(&instance_packs)?;
        sync_resourcepacks_dir(&shared_packs, &instance_packs)?;
    }
    Ok(())
}

pub fn execute_sync_after_exit(
    shared: &Path,
    instance_dir: &Path,
    eff: &EffectiveSync,
) -> Result<(), SyncError> {
    if eff.options {
        let src = instance_dir.join("options.txt");
        if file_exists(&src)? {
            let dst = shared.join("options.txt");
            backup_file_if_exists(&dst)?;
            copy_file_atomic(&src, &dst)?;
        }
    }
    if eff.servers {
        let src = instance_dir.join("servers.dat");
        if file_exists(&src)? {
            let dst = shared.join("servers.dat");
            backup_file_if_exists(&dst)?;
            copy_file_atomic(&src, &dst)?;
        }
    }
    if eff.command_history {
        let src = instance_dir.join("command_history.txt");
        if file_exists(&src)? {
            let dst = shared.join("command_history.txt");
            backup_file_if_exists(&dst)?;
            copy_file_atomic(&src, &dst)?;
        }
    }
    if eff.creative_hotbars {
        let src = instance_dir.join("hotbar.nbt");
        if file_exists(&src)? {
            let dst = shared.join("hotbar.nbt");
            backup_file_if_exists(&dst)?;
            copy_file_atomic(&src, &dst)?;
        }
    }
    if eff.resource_packs {
        let shared_packs = shared.join("resourcepacks");
        let instance_packs = instance_dir.join("resourcepacks");
        fs::create_dir_all(&shared_packs)?;
        sync_resourcepacks_dir(&instance_packs, &shared_packs)?;
    }
    Ok(())
}

pub fn export_instance_category_files(
    shared: &Path,
    instance_dir: &Path,
    category: SyncCategory,
) -> Result<SyncReport, SyncError> {
    let mut report = SyncReport {
        options_synced: false,
        servers_synced: false,
        resource_packs_count: 0,
        command_history_synced: false,
        creative_hotbars_synced: false,
        message: String::new(),
    };

    let do_all = category == SyncCategory::All;

    if do_all || category == SyncCategory::Options {
        let opt_src = instance_dir.join("options.txt");
        if file_exists(&opt_src)? {
            let dst = shared.join("options.txt");
            backup_file_if_exists(&dst)?;
            copy_file_atomic(&opt_src, &dst)?;
            report.options_synced = true;
        }
    }

    if do_all || category == SyncCategory::Servers {
        let srv_src = instance_dir.join("servers.dat");
        if file_exists(&srv_src)? {
            let dst = shared.join("servers.dat");
            backup_file_if_exists(&dst)?;
            copy_file_atomic(&srv_src, &dst)?;
            report.servers_synced = true;
        }
    }

    if do_all || category == SyncCategory::CommandHistory {
        let cmd_src = instance_dir.join("command_history.txt");
        if file_exists(&cmd_src)? {
            let dst = shared.join("command_history.txt");
            backup_file_if_exists(&dst)?;
            copy_file_atomic(&cmd_src, &dst)?;
            report.command_history_synced = true;
        }
    }

    if do_all || category == SyncCategory::Hotbar {
        let hot_src = instance_dir.join("hotbar.nbt");
        if file_exists(&hot_src)? {
            let dst = shared.join("hotbar.nbt");
            backup_file_if_exists(&dst)?;
            copy_file_atomic(&hot_src, &dst)?;
            report.creative_hotbars_synced = true;
        }
    }

    if do_all || category == SyncCategory::ResourcePacks {
        let inst_packs = instance_dir.join("resourcepacks");
        let shared_packs = shared.join("resourcepacks");
        report.resource_packs_count = sync_resourcepacks_dir(&inst_packs, &shared_packs)?;
    }

    report.message = format!(
        "Successfully exported {} to shared sync storage.",
        category.as_str()
    );
    Ok(report)
}

pub fn import_shared_files_to_instance(
    shared: &Path,
    instance_dir: &Path,
) -> Result<SyncReport, SyncError> {
    let mut report = SyncReport {
        options_synced: false,
        servers_synced: false,
        resource_packs_count: 0,
        command_history_synced: false,
        creative_hotbars_synced: false,
        message: "Successfully imported shared sync data into instance.".to_string(),
    };

    let opt_shared = shared.join("options.txt");
    if file_exists(&opt_shared)? {
        let dst = instance_dir.join("options.txt");
        backup_file_if_exists(&dst)?;
        copy_file_atomic(&opt_shared, &dst)?;
        report.options_synced = true;
    }

    let srv_shared = shared.join("servers.dat");
    if file_exists(&srv_shared)? {
        let dst = instance_dir.join("servers.dat");
        backup_file_if_exists(&dst)?;
        copy_file_atomic(&srv_shared, &dst)?;
        report.servers_synced = true;
    }

    let cmd_shared = shared.join("command_history.txt");
    if file_exists(&cmd_shared)? {
        let dst = instance_dir.join("command_history.txt");
        backup_file_if_exists(&dst)?;
        copy_file_atomic(&cmd_shared, &dst)?;
        report.command_history_synced = true;
    }

    let hot_shared = shared.join("hotbar.nbt");
    if file_exists(&hot_shared)? {
        let dst = instance_dir.join("hotbar.nbt");
        backup_file_if_exists(&dst)?;
        copy_file_atomic(&hot_shared, &dst)?;
        report.creative_hotbars_synced = true;
    }

    let shared_packs = shared.join("resourcepacks");
    let inst_packs = instance_dir.join("resourcepacks");
    report.resource_packs_count = sync_resourcepacks_dir(&shared_packs, &inst_packs)?;

    Ok(report)
}
