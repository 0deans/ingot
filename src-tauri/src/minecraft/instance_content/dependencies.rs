use std::collections::HashMap;

use super::models::{ContentDependencyIssue, ContentState, InstalledContent};

pub fn check_dependencies(items: &[InstalledContent]) -> Vec<ContentDependencyIssue> {
    let mut known_mods: HashMap<String, ContentState> = HashMap::new();

    for item in items {
        if let Some(mod_id) = &item.metadata.mod_id {
            let lower = mod_id.to_ascii_lowercase();
            // If any jar for this mod id is enabled, consider it enabled
            match known_mods.get(&lower) {
                Some(ContentState::Enabled) => {}
                _ => {
                    known_mods.insert(lower, item.state);
                }
            }
        }
    }

    let mut issues = Vec::new();

    for item in items {
        if item.state != ContentState::Enabled {
            continue;
        }

        let mod_id = item.metadata.mod_id.as_deref().unwrap_or(&item.file_name);

        for dep in &item.metadata.dependencies {
            if !dep.required {
                continue;
            }

            let dep_lower = dep.id.to_ascii_lowercase();
            match known_mods.get(&dep_lower) {
                None => {
                    issues.push(ContentDependencyIssue {
                        mod_id: mod_id.to_string(),
                        mod_name: item.metadata.name.clone(),
                        missing_dependency_id: dep.id.clone(),
                        disabled: false,
                    });
                }
                Some(ContentState::Disabled) => {
                    issues.push(ContentDependencyIssue {
                        mod_id: mod_id.to_string(),
                        mod_name: item.metadata.name.clone(),
                        missing_dependency_id: dep.id.clone(),
                        disabled: true,
                    });
                }
                Some(ContentState::Enabled) => {}
            }
        }
    }

    issues.sort_by(|a, b| a.mod_name.cmp(&b.mod_name));
    issues
}
