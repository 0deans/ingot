use crate::minecraft::instance::InstanceConfig;
use crate::settings::SyncSettings;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EffectiveSync {
    pub options: bool,
    pub servers: bool,
    pub resource_packs: bool,
    pub command_history: bool,
    pub creative_hotbars: bool,
}

impl EffectiveSync {
    pub fn has_any_enabled(&self) -> bool {
        self.options
            || self.servers
            || self.resource_packs
            || self.command_history
            || self.creative_hotbars
    }
}

pub fn resolve_effective_sync(instance: &InstanceConfig, global: &SyncSettings) -> EffectiveSync {
    EffectiveSync {
        options: instance.sync_options.unwrap_or(global.sync_options),
        servers: instance.sync_servers.unwrap_or(global.sync_servers),
        resource_packs: instance
            .sync_resource_packs
            .unwrap_or(global.sync_resource_packs),
        command_history: instance
            .sync_command_history
            .unwrap_or(global.sync_command_history),
        creative_hotbars: instance
            .sync_creative_hotbars
            .unwrap_or(global.sync_creative_hotbars),
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BidirectionalSyncAction {
    CopySharedToInstance { backup_instance: bool },
    CopyInstanceToShared { backup_shared: bool },
    DoNothing,
}

pub fn decide_bidirectional_sync_action(
    shared_exists: bool,
    instance_exists: bool,
    shared_modified: SystemTime,
    instance_modified: SystemTime,
) -> BidirectionalSyncAction {
    if shared_exists && !instance_exists {
        BidirectionalSyncAction::CopySharedToInstance {
            backup_instance: false,
        }
    } else if !shared_exists && instance_exists {
        BidirectionalSyncAction::CopyInstanceToShared {
            backup_shared: false,
        }
    } else if shared_exists && instance_exists {
        if shared_modified > instance_modified {
            BidirectionalSyncAction::CopySharedToInstance {
                backup_instance: true,
            }
        } else if instance_modified > shared_modified {
            BidirectionalSyncAction::CopyInstanceToShared {
                backup_shared: true,
            }
        } else {
            BidirectionalSyncAction::DoNothing
        }
    } else {
        BidirectionalSyncAction::DoNothing
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictCheckInputs {
    pub has_last_synced: bool,
    pub effective: EffectiveSync,
    pub has_instance_options: bool,
    pub has_instance_servers: bool,
    pub has_shared_options: bool,
    pub has_shared_servers: bool,
}

pub fn evaluate_conflict(inputs: &ConflictCheckInputs) -> bool {
    if !inputs.effective.has_any_enabled() || inputs.has_last_synced {
        return false;
    }
    let options_conflict =
        inputs.effective.options && inputs.has_instance_options && inputs.has_shared_options;
    let servers_conflict =
        inputs.effective.servers && inputs.has_instance_servers && inputs.has_shared_servers;
    options_conflict || servers_conflict
}
