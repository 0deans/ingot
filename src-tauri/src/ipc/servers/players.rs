//! IPC procedures for server player queries.
use super::{get_server_process_manager, is_server_running, server_with_config};
use crate::ipc::ApiError;
use crate::server::live::{KnownPlayer, PlayerDetails};
use crate::server::{self, ServerCoreType};
use tauri::Runtime;

#[taurpc::procedures(path = "servers.players")]
pub trait ServerPlayersApi {
    /// Live details of all online players (console queries; server must be running).
    async fn get_online_players(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<PlayerDetails>, ApiError>;

    async fn get_server_online_players(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<String>, ApiError>;

    /// Live data for online players, last saved state otherwise.
    async fn get_player_details(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        name: String,
    ) -> Result<PlayerDetails, ApiError>;

    /// Everyone with a save file, newest first.
    async fn get_known_players(
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<KnownPlayer>, ApiError>;
}

#[derive(Clone)]
pub struct ServerPlayersApiImpl;

#[taurpc::resolvers]
impl ServerPlayersApi for ServerPlayersApiImpl {
    async fn get_online_players(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<PlayerDetails>, ApiError> {
        server::live::online_players(get_server_process_manager(&app_handle), &server_id)
            .await
            .map_err(ApiError::from)
    }

    async fn get_server_online_players(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<String>, ApiError> {
        Ok(get_server_process_manager(&app_handle)
            .get_server_online_players(&server_id)
            .await)
    }

    async fn get_player_details(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
        name: String,
    ) -> Result<PlayerDetails, ApiError> {
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        if config.core == ServerCoreType::Bedrock {
            if is_server_running(get_server_process_manager(&app_handle), &server_id).await {
                if let Ok(details) =
                    server::live::query_player(get_server_process_manager(&app_handle), &server_id, &name).await
                {
                    return Ok(details);
                }
            }
            return Ok(server::live::offline_bedrock_player(&dir, &name));
        }

        if is_server_running(get_server_process_manager(&app_handle), &server_id).await {
            if let Ok(details) =
                server::live::query_player(get_server_process_manager(&app_handle), &server_id, &name).await
            {
                return Ok(details);
            }
        }
        let world = dir.join(server::map::level_name(&dir));
        tauri::async_runtime::spawn_blocking(move || {
            server::live::offline_player(&dir, &world, &name)
        })
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
        .map_err(ApiError::from)
    }

    async fn get_known_players(
        self,
        app_handle: tauri::AppHandle<impl Runtime>,
        server_id: String,
    ) -> Result<Vec<KnownPlayer>, ApiError> {
        let (dir, config) = server_with_config(&app_handle, &server_id)?;
        let mut known = if config.core == ServerCoreType::Bedrock {
            tauri::async_runtime::spawn_blocking(move || server::live::known_bedrock_players(&dir))
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?
        } else {
            let world = dir.join(server::map::level_name(&dir));
            tauri::async_runtime::spawn_blocking(move || server::live::known_players(&dir, &world))
                .await
                .map_err(|e| ApiError::internal(e.to_string()))?
        };
        if is_server_running(get_server_process_manager(&app_handle), &server_id).await {
            if let Ok(online) =
                server::live::list_online(get_server_process_manager(&app_handle), &server_id).await
            {
                for (name, uuid) in online {
                    match known
                        .iter_mut()
                        .find(|k| k.name.eq_ignore_ascii_case(&name))
                    {
                        Some(k) => k.online = true,
                        None => known.insert(
                            0,
                            KnownPlayer {
                                name,
                                uuid: uuid.unwrap_or_default(),
                                online: true,
                                last_seen: None,
                            },
                        ),
                    }
                }
            }
        }
        Ok(known)
    }
}
