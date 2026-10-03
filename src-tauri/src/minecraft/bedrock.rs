use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct BedrockClientStatus {
    pub is_installed: bool,
    pub version: Option<String>,
    pub edition: String, // "retail" | "preview" | "not_installed"
    pub package_family_name: Option<String>,
    pub install_location: Option<String>,
    pub worlds_path: Option<String>,
    pub resource_packs_path: Option<String>,
    pub behavior_packs_path: Option<String>,
    pub platform: String,
    pub store_url: String,
}

#[derive(Debug, Deserialize)]
struct AppxPackageInfo {
    #[serde(rename = "Name")]
    name: Option<String>,
    #[serde(rename = "Version")]
    version: Option<String>,
    #[serde(rename = "PackageFamilyName")]
    package_family_name: Option<String>,
    #[serde(rename = "InstallLocation")]
    install_location: Option<String>,
}

/// Detects Bedrock Edition installation and data directories
pub fn detect_bedrock_status() -> BedrockClientStatus {
    let platform = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "android") {
        "android"
    } else {
        "other"
    };

    let store_url = if platform == "windows" {
        "ms-windows-store://pdp/?ProductId=9NBLGGH2JHXJ".to_string()
    } else if platform == "android" {
        "market://details?id=com.mojang.minecraftpe".to_string()
    } else {
        "https://www.minecraft.net/download".to_string()
    };

    if platform != "windows" {
        return BedrockClientStatus {
            is_installed: false,
            version: None,
            edition: "not_installed".to_string(),
            package_family_name: None,
            install_location: None,
            worlds_path: None,
            resource_packs_path: None,
            behavior_packs_path: None,
            platform: platform.to_string(),
            store_url,
        };
    }

    #[cfg(target_os = "windows")]
    {
        let mut is_installed = false;
        let mut version = None;
        let mut edition = "not_installed".to_string();
        let mut package_family_name = None;
        let mut install_location = None;

        // 1. Try querying AppxPackage via PowerShell (with 0x08000000 CREATE_NO_WINDOW)
        let mut ps_cmd = Command::new("powershell");
        ps_cmd
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg("Get-AppxPackage -Name '*Minecraft*' | Select-Object Name,Version,PackageFamilyName,InstallLocation | ConvertTo-Json -Compress");

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            ps_cmd.creation_flags(0x08000000);
        }

        if let Ok(output) = ps_cmd.output() {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !text.is_empty() {
                    // Could be a single object or an array of objects
                    if let Ok(pkg) = serde_json::from_str::<AppxPackageInfo>(&text) {
                        apply_package_info(&pkg, &mut is_installed, &mut version, &mut edition, &mut package_family_name, &mut install_location);
                    } else if let Ok(pkgs) = serde_json::from_str::<Vec<AppxPackageInfo>>(&text) {
                        if let Some(first) = pkgs.first() {
                            apply_package_info(first, &mut is_installed, &mut version, &mut edition, &mut package_family_name, &mut install_location);
                        }
                    }
                }
            }
        }

        // 2. Check Windows Registry protocol handler `HKCR\minecraft` as secondary check
        if !is_installed {
            let mut reg_cmd = Command::new("reg");
            reg_cmd.args(["query", "HKCR\\minecraft"]);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                reg_cmd.creation_flags(0x08000000);
            }
            if let Ok(out) = reg_cmd.output() {
                if out.status.success() {
                    is_installed = true;
                    if edition == "not_installed" {
                        edition = "retail".to_string();
                    }
                }
            }
        }

        // 3. Locate Bedrock storage directories (Worlds, Resource Packs, Behavior Packs)
        let (worlds_path, resource_packs_path, behavior_packs_path) = find_storage_paths();

        // If worlds folder actually exists on disk, Bedrock is definitely installed/played
        if worlds_path.is_some() {
            is_installed = true;
            if edition == "not_installed" {
                edition = "retail".to_string();
            }
        }

        BedrockClientStatus {
            is_installed,
            version,
            edition,
            package_family_name,
            install_location,
            worlds_path,
            resource_packs_path,
            behavior_packs_path,
            platform: platform.to_string(),
            store_url,
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        BedrockClientStatus {
            is_installed: false,
            version: None,
            edition: "not_installed".to_string(),
            package_family_name: None,
            install_location: None,
            worlds_path: None,
            resource_packs_path: None,
            behavior_packs_path: None,
            platform: platform.to_string(),
            store_url,
        }
    }
}

fn apply_package_info(
    pkg: &AppxPackageInfo,
    is_installed: &mut bool,
    version: &mut Option<String>,
    edition: &mut String,
    package_family_name: &mut Option<String>,
    install_location: &mut Option<String>,
) {
    *is_installed = true;
    *version = pkg.version.clone();
    *package_family_name = pkg.package_family_name.clone();
    *install_location = pkg.install_location.clone();

    if let Some(ref name) = pkg.name {
        if name.contains("Beta") || name.contains("Preview") {
            *edition = "preview".to_string();
        } else {
            *edition = "retail".to_string();
        }
    } else {
        *edition = "retail".to_string();
    }
}

/// Locates Bedrock GDK or UWP storage paths on Windows
fn find_storage_paths() -> (Option<String>, Option<String>, Option<String>) {
    let appdata = std::env::var("APPDATA").ok().map(PathBuf::from);
    let localappdata = std::env::var("LOCALAPPDATA").ok().map(PathBuf::from);

    // 1. Try modern GDK path: %APPDATA%\Minecraft Bedrock\Users\
    if let Some(ref appdata_dir) = appdata {
        let gdk_root = appdata_dir.join("Minecraft Bedrock");
        if gdk_root.exists() {
            let users_dir = gdk_root.join("Users");
            let mut detected_worlds: Option<PathBuf> = None;

            if users_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&users_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() {
                            let w_path = path.join("games").join("com.mojang").join("minecraftWorlds");
                            if w_path.exists() {
                                detected_worlds = Some(w_path);
                                break;
                            }
                        }
                    }
                }
            }

            let shared_mojang = users_dir.join("Shared").join("games").join("com.mojang");
            let rp_path = shared_mojang.join("resource_packs");
            let bp_path = shared_mojang.join("behavior_packs");

            let worlds = detected_worlds.map(|p| p.to_string_lossy().to_string());
            let rp = if rp_path.exists() { Some(rp_path.to_string_lossy().to_string()) } else { None };
            let bp = if bp_path.exists() { Some(bp_path.to_string_lossy().to_string()) } else { None };

            if worlds.is_some() || rp.is_some() || bp.is_some() {
                return (worlds, rp, bp);
            }
        }
    }

    // 2. Try legacy UWP path: %LOCALAPPDATA%\Packages\Microsoft.MinecraftUWP_8wekyb3d8bbwe\LocalState\games\com.mojang\
    if let Some(ref local_dir) = localappdata {
        let uwp_mojang = local_dir
            .join("Packages")
            .join("Microsoft.MinecraftUWP_8wekyb3d8bbwe")
            .join("LocalState")
            .join("games")
            .join("com.mojang");

        if uwp_mojang.exists() {
            let worlds = uwp_mojang.join("minecraftWorlds");
            let rp = uwp_mojang.join("resource_packs");
            let bp = uwp_mojang.join("behavior_packs");

            return (
                if worlds.exists() { Some(worlds.to_string_lossy().to_string()) } else { None },
                if rp.exists() { Some(rp.to_string_lossy().to_string()) } else { None },
                if bp.exists() { Some(bp.to_string_lossy().to_string()) } else { None },
            );
        }
    }

    (None, None, None)
}

/// Launches Minecraft Bedrock Edition
pub fn launch_bedrock() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/c", "start", "minecraft:"]);
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
        cmd.spawn()
            .map_err(|e| format!("Failed to launch Minecraft Bedrock: {e}"))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Minecraft Bedrock launch is only supported on Windows".to_string())
    }
}

/// Opens the Microsoft Store page for Minecraft Bedrock Edition
pub fn install_bedrock() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/c", "start", "ms-windows-store://pdp/?ProductId=9NBLGGH2JHXJ"]);
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
        cmd.spawn()
            .map_err(|e| format!("Failed to open Microsoft Store: {e}"))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Microsoft Store is only available on Windows".to_string())
    }
}

/// Opens a specific Bedrock directory in File Explorer
pub fn open_bedrock_folder(kind: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let status = detect_bedrock_status();
        let target_path = match kind {
            "screenshots" => {
                let folder = crate::minecraft::screenshots::get_primary_bedrock_screenshots_folder();
                Some(folder.to_string_lossy().to_string())
            }
            "worlds" => status.worlds_path.or_else(|| {
                // Default fallback GDK path
                std::env::var("APPDATA").ok().map(|a| {
                    format!("{a}\\Minecraft Bedrock\\Users")
                })
            }),
            "resource_packs" => status.resource_packs_path.or_else(|| {
                std::env::var("APPDATA").ok().map(|a| {
                    format!("{a}\\Minecraft Bedrock\\Users\\Shared\\games\\com.mojang\\resource_packs")
                })
            }),
            "behavior_packs" => status.behavior_packs_path.or_else(|| {
                std::env::var("APPDATA").ok().map(|a| {
                    format!("{a}\\Minecraft Bedrock\\Users\\Shared\\games\\com.mojang\\behavior_packs")
                })
            }),
            _ => {
                // Default root
                std::env::var("APPDATA").ok().map(|a| {
                    format!("{a}\\Minecraft Bedrock")
                })
            }
        };

        if let Some(path_str) = target_path {
            let p = Path::new(&path_str);
            if !p.exists() {
                let _ = std::fs::create_dir_all(p);
            }
            let mut cmd = Command::new("explorer");
            cmd.arg(&path_str);
            cmd.spawn().map_err(|e| format!("Failed to open folder: {e}"))?;
            Ok(())
        } else {
            Err("Could not determine Bedrock folder location".to_string())
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("Opening Bedrock folder is only supported on Windows".to_string())
    }
}
