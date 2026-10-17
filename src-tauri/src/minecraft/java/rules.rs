/// Resolves the required Java major version for a given Minecraft version.
pub fn get_required_java_version(game_version: &str, meta_major_version: Option<u32>) -> u32 {
    if let Some(v) = meta_major_version {
        if v > 0 {
            return v;
        }
    }

    let parts: Vec<&str> = game_version.split('.').collect();
    if !parts.is_empty() {
        if let Ok(first) = parts[0].parse::<u32>() {
            if first >= 25 {
                return 25;
            } else if first >= 20 {
                return 21;
            } else if first == 1 && parts.len() >= 2 {
                if let Ok(minor) = parts[1].parse::<u32>() {
                    if minor >= 21 {
                        return 21;
                    } else if minor == 20 {
                        let patch = parts
                            .get(2)
                            .and_then(|p| p.parse::<u32>().ok())
                            .unwrap_or(0);
                        if patch >= 5 {
                            return 21;
                        } else {
                            return 17;
                        }
                    } else if minor >= 18 {
                        return 17;
                    } else if minor == 17 {
                        return 16;
                    } else {
                        return 8;
                    }
                }
            }
        }
    }

    // Check for snapshot naming like "26w04a"
    if game_version.starts_with("26w") || game_version.starts_with("25w") {
        return 25;
    }

    21
}
