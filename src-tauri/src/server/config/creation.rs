use super::{
    error::ConfigError,
    models::{ServerConfig, ServerCoreType},
    ports::{check_port_typed, free_port_for_core, internal_port_for},
};

pub(crate) struct ServerCreation {
    pub(crate) name: String,
    pub(crate) core: ServerCoreType,
    pub(crate) game_version: String,
    pub(crate) build_number: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) memory_min_mb: Option<u32>,
    pub(crate) memory_max_mb: Option<u32>,
}

pub(crate) fn prepare_server_config(
    servers: &[ServerConfig],
    request: ServerCreation,
    id: String,
    now: u64,
) -> Result<ServerConfig, ConfigError> {
    let ServerCreation {
        name,
        core,
        game_version,
        build_number,
        port,
        memory_min_mb,
        memory_max_mb,
    } = request;
    let selected_port = match port {
        Some(p) => {
            check_port_typed(servers, &id, p)?;
            p
        }
        None => free_port_for_core(servers, &id, &core),
    };

    let is_bedrock = core == ServerCoreType::Bedrock;
    let config = ServerConfig {
        id: id.clone(),
        name,
        core,
        game_version,
        build_number,
        port: selected_port,
        memory_min_mb: memory_min_mb.unwrap_or(if is_bedrock { 1024 } else { 2048 }),
        memory_max_mb: memory_max_mb.unwrap_or(if is_bedrock { 2048 } else { 4096 }),
        icon: None,
        java_path: None,
        jvm_args: None,
        auto_start: Some(false),
        sleep_enabled: Some(!is_bedrock),
        idle_timeout_seconds: Some(600),
        internal_port: Some(internal_port_for(selected_port)),
        playit_enabled: Some(false),
        playit_secret_key: None,
        created_at: now,
        last_run_at: None,
    };

    validate_server_settings(&config)?;
    Ok(config)
}

pub(crate) fn validate_server_settings(config: &ServerConfig) -> Result<(), ConfigError> {
    if config.name.trim().is_empty() {
        return Err(ConfigError::EmptyName);
    }
    if config.memory_min_mb == 0 || config.memory_max_mb < config.memory_min_mb {
        return Err(ConfigError::InvalidMemory {
            minimum_mb: config.memory_min_mb,
            maximum_mb: config.memory_max_mb,
        });
    }
    Ok(())
}
