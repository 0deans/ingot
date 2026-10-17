use crate::minecraft::version::{check_rules, ArgumentValue, VersionArguments};
use std::path::Path;

pub fn is_version_at_least(version_str: &str, target_major: u32, target_minor: u32) -> bool {
    let parts: Vec<&str> = version_str.split('.').collect();
    if parts.len() >= 2 {
        if let (Ok(major), Ok(minor)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
            if major > target_major {
                return true;
            }
            if major == target_major && minor >= target_minor {
                return true;
            }
            return false;
        }
    }
    if let Some(first_two) = version_str.get(0..2) {
        if let Ok(year) = first_two.parse::<u32>() {
            if year >= 23 {
                return true;
            }
        }
    }
    false
}

pub struct JvmArgsConfig<'a> {
    pub min_ram_mb: u32,
    pub max_ram_mb: u32,
    pub natives_dir: &'a Path,
    pub libraries_dir: &'a Path,
    pub classpath_str: &'a str,
    pub cp_separator: &'a str,
    pub custom_jvm_args: Option<&'a [String]>,
    pub authlib_agent_path: Option<&'a Path>,
    pub version_arguments: Option<&'a VersionArguments>,
    pub main_class: &'a str,
}

pub fn build_jvm_arguments(cfg: &JvmArgsConfig) -> Vec<String> {
    let mut cmd_args: Vec<String> = Vec::new();

    // JVM Memory args
    cmd_args.push(format!("-Xms{}M", cfg.min_ram_mb));
    cmd_args.push(format!("-Xmx{}M", cfg.max_ram_mb));

    // Common performance JVM flags
    cmd_args.push("-XX:+UnlockExperimentalVMOptions".into());
    cmd_args.push("-XX:+UseG1GC".into());
    cmd_args.push("-XX:G1NewSizePercent=20".into());
    cmd_args.push("-XX:G1ReservePercent=20".into());
    cmd_args.push("-XX:MaxGCPauseMillis=50".into());
    cmd_args.push("-XX:G1HeapRegionSize=32M".into());

    // Ely authlib injector if provided
    if let Some(agent_path) = cfg.authlib_agent_path {
        if agent_path.exists() {
            cmd_args.push("-Dauthlibinjector.noLogFile".into());
            cmd_args.push(format!(
                "-javaagent:{}={}",
                agent_path.to_string_lossy(),
                "ely.by"
            ));
        }
    }

    // Custom JVM args from instance
    if let Some(custom) = cfg.custom_jvm_args {
        cmd_args.extend(custom.iter().cloned());
    }

    let replace_jvm_vars = |arg: &str| -> String {
        arg.replace("${natives_directory}", &cfg.natives_dir.to_string_lossy())
            .replace("${launcher_name}", "ingot")
            .replace("${launcher_version}", env!("CARGO_PKG_VERSION"))
            .replace("${classpath}", cfg.classpath_str)
            .replace("${library_directory}", &cfg.libraries_dir.to_string_lossy())
            .replace("${classpath_separator}", cfg.cp_separator)
    };

    let mut has_manifest_jvm = false;
    let mut classpath_handled_by_manifest = false;

    if let Some(args_obj) = cfg.version_arguments {
        if let Some(ref jvm_args) = args_obj.jvm {
            has_manifest_jvm = true;
            for arg in jvm_args {
                match arg {
                    ArgumentValue::Simple(s) => {
                        let replaced = replace_jvm_vars(s);
                        if replaced == "-cp" || replaced == "${classpath}" || s == "${classpath}" {
                            classpath_handled_by_manifest = true;
                        }
                        cmd_args.push(replaced);
                    }
                    ArgumentValue::Structured { rules, value } => {
                        let allowed = rules.as_ref().map(|r| check_rules(r)).unwrap_or(true);
                        if allowed {
                            if let Some(s) = value.as_str() {
                                let replaced = replace_jvm_vars(s);
                                if replaced == "-cp"
                                    || replaced == "${classpath}"
                                    || s == "${classpath}"
                                {
                                    classpath_handled_by_manifest = true;
                                }
                                cmd_args.push(replaced);
                            } else if let Some(arr) = value.as_array() {
                                for item in arr {
                                    if let Some(s) = item.as_str() {
                                        let replaced = replace_jvm_vars(s);
                                        if replaced == "-cp"
                                            || replaced == "${classpath}"
                                            || s == "${classpath}"
                                        {
                                            classpath_handled_by_manifest = true;
                                        }
                                        cmd_args.push(replaced);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if !has_manifest_jvm {
        cmd_args.push(format!(
            "-Djava.library.path={}",
            cfg.natives_dir.to_string_lossy()
        ));
        cmd_args.push("-Dminecraft.launcher.brand=ingot".into());
        cmd_args.push(format!(
            "-Dminecraft.launcher.version={}",
            env!("CARGO_PKG_VERSION")
        ));
        #[cfg(target_os = "windows")]
        cmd_args.push(
            "-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump"
                .into(),
        );
        #[cfg(target_os = "macos")]
        cmd_args.push("-XstartOnFirstThread".into());
    }

    if !classpath_handled_by_manifest {
        cmd_args.push("-cp".into());
        cmd_args.push(cfg.classpath_str.to_string());
    }

    cmd_args.push(cfg.main_class.to_string());

    cmd_args
}

pub struct GameArgsConfig<'a> {
    pub player_name: &'a str,
    pub game_version: &'a str,
    pub instance_dir: &'a Path,
    pub assets_dir: &'a Path,
    pub asset_index_id: &'a str,
    pub uuid_str: &'a str,
    pub access_token: &'a str,
    pub user_type: &'a str,
    pub xuid: Option<&'a str>,
    pub version_arguments: Option<&'a VersionArguments>,
    pub legacy_arguments: Option<&'a str>,
    pub is_fullscreen: bool,
    pub window_width: u32,
    pub window_height: u32,
}

pub fn build_game_arguments(cfg: &GameArgsConfig) -> Vec<String> {
    let mut cmd_args: Vec<String> = Vec::new();

    let replace_vars = |arg: &str| -> String {
        arg.replace("${auth_player_name}", cfg.player_name)
            .replace("${version_name}", cfg.game_version)
            .replace("${game_directory}", &cfg.instance_dir.to_string_lossy())
            .replace("${assets_root}", &cfg.assets_dir.to_string_lossy())
            .replace("${assets_index_name}", cfg.asset_index_id)
            .replace("${auth_uuid}", cfg.uuid_str)
            .replace("${auth_access_token}", cfg.access_token)
            .replace("${user_type}", cfg.user_type)
            .replace("${version_type}", "release")
            .replace("${clientid}", cfg.uuid_str)
            .replace("${auth_xuid}", cfg.xuid.unwrap_or("0"))
    };

    if let Some(args_obj) = cfg.version_arguments {
        if let Some(ref game_args) = args_obj.game {
            for arg in game_args {
                match arg {
                    ArgumentValue::Simple(s) => {
                        let replaced = replace_vars(s);
                        if !replaced.contains("${") {
                            cmd_args.push(replaced);
                        }
                    }
                    ArgumentValue::Structured { rules, value } => {
                        let allowed = rules.as_ref().map(|r| check_rules(r)).unwrap_or(true);
                        if allowed {
                            if let Some(s) = value.as_str() {
                                let replaced = replace_vars(s);
                                if !replaced.contains("${") && replaced != "--demo" {
                                    cmd_args.push(replaced);
                                }
                            } else if let Some(arr) = value.as_array() {
                                let mut replaced_group = Vec::new();
                                let mut has_unresolved = false;
                                for item in arr {
                                    if let Some(s) = item.as_str() {
                                        let replaced = replace_vars(s);
                                        if replaced.contains("${") {
                                            has_unresolved = true;
                                            break;
                                        }
                                        replaced_group.push(replaced);
                                    }
                                }
                                if !has_unresolved {
                                    cmd_args.extend(replaced_group);
                                }
                            }
                        }
                    }
                }
            }
        }
    } else if let Some(legacy_args) = cfg.legacy_arguments {
        for part in legacy_args.split_whitespace() {
            let replaced = replace_vars(part);
            if !replaced.contains("${") {
                cmd_args.push(replaced);
            }
        }
    }

    apply_window_settings(
        &mut cmd_args,
        cfg.is_fullscreen,
        cfg.window_width,
        cfg.window_height,
    );

    cmd_args
}

pub fn apply_window_settings(
    cmd_args: &mut Vec<String>,
    is_fullscreen: bool,
    width: u32,
    height: u32,
) {
    if is_fullscreen {
        if !cmd_args.iter().any(|a| a == "--fullscreen") {
            cmd_args.push("--fullscreen".into());
        }
    } else {
        if !cmd_args.iter().any(|a| a == "--width") {
            cmd_args.push("--width".into());
            cmd_args.push(width.to_string());
        }
        if !cmd_args.iter().any(|a| a == "--height") {
            cmd_args.push("--height".into());
            cmd_args.push(height.to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn version_comparison_logic() {
        assert!(is_version_at_least("1.20.1", 1, 20));
        assert!(is_version_at_least("1.21", 1, 20));
        assert!(!is_version_at_least("1.19.4", 1, 20));
        assert!(is_version_at_least("24w14a", 1, 20));
    }

    #[test]
    fn apply_window_settings_fullscreen() {
        let mut args = Vec::new();
        apply_window_settings(&mut args, true, 800, 600);
        assert_eq!(args, vec!["--fullscreen"]);
    }

    #[test]
    fn apply_window_settings_windowed() {
        let mut args = Vec::new();
        apply_window_settings(&mut args, false, 1280, 720);
        assert_eq!(args, vec!["--width", "1280", "--height", "720"]);
    }

    #[test]
    fn build_jvm_arguments_includes_ram_and_main_class() {
        let nat = PathBuf::from("/nat");
        let lib = PathBuf::from("/lib");
        let cfg = JvmArgsConfig {
            min_ram_mb: 2048,
            max_ram_mb: 4096,
            natives_dir: &nat,
            libraries_dir: &lib,
            classpath_str: "/lib/a.jar",
            cp_separator: ";",
            custom_jvm_args: Some(&["-Dcustom=true".into()]),
            authlib_agent_path: None,
            version_arguments: None,
            main_class: "net.minecraft.client.main.Main",
        };
        let args = build_jvm_arguments(&cfg);
        assert!(args.contains(&"-Xms2048M".to_string()));
        assert!(args.contains(&"-Xmx4096M".to_string()));
        assert!(args.contains(&"-Dcustom=true".to_string()));
        assert!(args.contains(&"-cp".to_string()));
        assert!(args.contains(&"/lib/a.jar".to_string()));
        assert_eq!(args.last().unwrap(), "net.minecraft.client.main.Main");
    }

    #[test]
    fn build_game_arguments_substitutes_standard_variables() {
        let inst = PathBuf::from("/inst");
        let assets = PathBuf::from("/assets");
        let cfg = GameArgsConfig {
            player_name: "Steve",
            game_version: "1.20.1",
            instance_dir: &inst,
            assets_dir: &assets,
            asset_index_id: "1.20",
            uuid_str: "1234-uuid",
            access_token: "token123",
            user_type: "mojang",
            xuid: None,
            version_arguments: None,
            legacy_arguments: Some(
                "--username ${auth_player_name} --version ${version_name} --gameDir ${game_directory}",
            ),
            is_fullscreen: false,
            window_width: 854,
            window_height: 480,
        };
        let args = build_game_arguments(&cfg);
        assert!(args.contains(&"--username".to_string()));
        assert!(args.contains(&"Steve".to_string()));
        assert!(args.contains(&"--version".to_string()));
        assert!(args.contains(&"1.20.1".to_string()));
        assert!(args.contains(&"--width".to_string()));
        assert!(args.contains(&"854".to_string()));
    }
}
