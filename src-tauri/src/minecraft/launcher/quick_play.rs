use super::arguments::is_version_at_least;
use super::models::QuickPlayOptions;
use std::path::Path;

pub fn apply_quick_play_arguments(
    cmd_args: &mut Vec<String>,
    game_version: &str,
    instance_dir: &Path,
    quick_play: Option<&QuickPlayOptions>,
) {
    let Some(qp) = quick_play else {
        return;
    };

    if let Some(ref server) = qp.server {
        let server = server.trim();
        if !server.is_empty() {
            if is_version_at_least(game_version, 1, 20) {
                cmd_args.push("--quickPlayMultiplayer".into());
                cmd_args.push(server.to_string());
            } else {
                let (host, port) = match server.rsplit_once(':') {
                    Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) => (h, p),
                    _ => (server, "25565"),
                };
                cmd_args.push("--server".into());
                cmd_args.push(host.to_string());
                cmd_args.push("--port".into());
                cmd_args.push(port.to_string());
            }
        }
    } else if let Some(ref world) = qp.world {
        let world = world.trim();
        if !world.is_empty() {
            if is_version_at_least(game_version, 1, 20) {
                let final_world_arg = if world.is_ascii() {
                    world.to_string()
                } else {
                    let saves_dir = instance_dir.join("saves");
                    let target_path = saves_dir.join(world);
                    let junction_name = "qp_world";
                    let junction_path = saves_dir.join(junction_name);

                    if junction_path.exists() {
                        #[cfg(target_os = "windows")]
                        let _ = std::fs::remove_dir(&junction_path);
                        #[cfg(not(target_os = "windows"))]
                        let _ = std::fs::remove_file(&junction_path);
                    }

                    #[cfg(target_os = "windows")]
                    {
                        use std::os::windows::process::CommandExt;
                        let _ = std::process::Command::new("cmd")
                            .args(["/C", "mklink", "/J"])
                            .arg(&junction_path)
                            .arg(&target_path)
                            .creation_flags(0x08000000) // CREATE_NO_WINDOW
                            .output();
                    }

                    #[cfg(not(target_os = "windows"))]
                    {
                        let _ = std::os::unix::fs::symlink(&target_path, &junction_path);
                    }

                    if junction_path.exists() {
                        junction_name.to_string()
                    } else {
                        world.to_string()
                    }
                };

                cmd_args.push("--quickPlaySingleplayer".into());
                cmd_args.push(final_world_arg);
            } else {
                eprintln!("[Launcher] Warning: Quick Play Singleplayer is only supported on Minecraft 1.20+");
            }
        }
    }
}

pub fn cleanup_quick_play_junction(instance_dir: &Path) {
    let qp_junction = instance_dir.join("saves").join("qp_world");
    #[cfg(target_os = "windows")]
    if qp_junction.exists() {
        let _ = std::fs::remove_dir(&qp_junction);
    }
    #[cfg(not(target_os = "windows"))]
    if std::fs::symlink_metadata(&qp_junction).is_ok() {
        if std::fs::remove_file(&qp_junction).is_err() {
            let _ = std::fs::remove_dir(&qp_junction);
        }
    }
}
