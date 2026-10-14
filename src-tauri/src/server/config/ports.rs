use super::models::{ConfigError, ServerConfig, ServerCoreType};

/// Internal loopback port offset (+10000, or -10000 for high ports) so consecutive
/// public ports like 25565, 25566, 25567 never collide with each other.
pub fn internal_port_for(port: u16) -> u16 {
    if port <= 55535 {
        port + 10000
    } else {
        port - 10000
    }
}

/// Port the game server itself runs on, behind Ingot's wake-up proxy on the public port.
pub fn internal_port(server: &ServerConfig) -> u16 {
    internal_port_for(server.port)
}

/// Ports a server listens on: the public one players join and the internal one
pub fn ports_of(server: &ServerConfig) -> [u16; 2] {
    [server.port, internal_port(server)]
}

/// Another server already using `port` or its internal port
pub fn port_owner<'a>(
    servers: &'a [ServerConfig],
    server_id: &str,
    port: u16,
) -> Option<&'a ServerConfig> {
    let wanted = [port, internal_port_for(port)];
    servers
        .iter()
        .filter(|s| s.id != server_id)
        .find(|s| ports_of(s).iter().any(|p| wanted.contains(p)))
}

/// Errors when `port` belongs to another server
pub fn check_port(servers: &[ServerConfig], server_id: &str, port: u16) -> Result<(), String> {
    check_port_typed(servers, server_id, port).map_err(|e| e.to_string())
}

/// Typed validation for port availability
pub fn check_port_typed(
    servers: &[ServerConfig],
    server_id: &str,
    port: u16,
) -> Result<(), ConfigError> {
    if port < 1024 {
        return Err(ConfigError::InvalidPortRange);
    }
    match port_owner(servers, server_id, port) {
        Some(other) => Err(ConfigError::PortInUse {
            port,
            owner: other.name.clone(),
            suggested: free_port(servers, server_id),
        }),
        None => Ok(()),
    }
}

/// Lowest free port starting from base depending on core type (19132 for Bedrock, 25565 for Java)
pub fn free_port_for_core(servers: &[ServerConfig], server_id: &str, core: &ServerCoreType) -> u16 {
    let mut port = if *core == ServerCoreType::Bedrock {
        19132
    } else {
        25565
    };
    while port_owner(servers, server_id, port).is_some() && port < u16::MAX - 1 {
        port += 1;
    }
    port
}

/// Lowest free port from 25565
pub fn free_port(servers: &[ServerConfig], server_id: &str) -> u16 {
    free_port_for_core(servers, server_id, &ServerCoreType::Paper)
}
