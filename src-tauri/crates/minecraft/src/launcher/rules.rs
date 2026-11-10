use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OsRule {
    pub name: Option<String>,
    pub version: Option<String>,
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleEntry {
    pub action: String, // "allow" or "disallow"
    pub os: Option<OsRule>,
}

/// Returns Mojang's identifier for the current host operating system.
pub fn current_os_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    }
}

/// Returns Mojang's identifier for the current host CPU architecture.
pub fn current_arch_name() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x64"
    } else if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        std::env::consts::ARCH
    }
}

/// Evaluates Mojang rules for the current operating system and architecture.
pub fn check_rules(rules: &[RuleEntry]) -> bool {
    check_rules_for_target(rules, current_os_name(), current_arch_name())
}

/// Evaluates Mojang rules against a specific operating system and architecture target.
pub fn check_rules_for_target(rules: &[RuleEntry], target_os: &str, target_arch: &str) -> bool {
    let mut allowed = false;

    for rule in rules {
        let matches = match &rule.os {
            Some(os_rule) => {
                let os_match = os_rule
                    .name
                    .as_deref()
                    .map(|n| n == target_os)
                    .unwrap_or(true);
                let arch_match = os_rule
                    .arch
                    .as_deref()
                    .map(|a| a == target_arch)
                    .unwrap_or(true);
                os_match && arch_match
            }
            None => true,
        };

        if matches {
            allowed = rule.action == "allow";
        }
    }

    allowed
}

/// Determines whether a library should be included based on its optional rule set.
pub fn should_include_library_rules(rules: Option<&[RuleEntry]>) -> bool {
    match rules {
        Some(rules) => check_rules(rules),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rules_disallow_by_default() {
        assert!(!check_rules_for_target(&[], "windows", "x64"));
    }

    #[test]
    fn single_allow_rule_without_os_allows_all_targets() {
        let rules = vec![RuleEntry {
            action: "allow".into(),
            os: None,
        }];
        assert!(check_rules_for_target(&rules, "windows", "x64"));
        assert!(check_rules_for_target(&rules, "osx", "arm64"));
        assert!(check_rules_for_target(&rules, "linux", "x64"));
    }

    #[test]
    fn os_specific_allow_rule_selects_matching_os_only() {
        let rules = vec![RuleEntry {
            action: "allow".into(),
            os: Some(OsRule {
                name: Some("windows".into()),
                version: None,
                arch: None,
            }),
        }];
        assert!(check_rules_for_target(&rules, "windows", "x64"));
        assert!(!check_rules_for_target(&rules, "osx", "x64"));
        assert!(!check_rules_for_target(&rules, "linux", "x64"));
    }

    #[test]
    fn sequential_rules_override_earlier_decisions() {
        // Typical Mojang pattern: allow everywhere, but disallow on OSX
        let rules = vec![
            RuleEntry {
                action: "allow".into(),
                os: None,
            },
            RuleEntry {
                action: "disallow".into(),
                os: Some(OsRule {
                    name: Some("osx".into()),
                    version: None,
                    arch: None,
                }),
            },
        ];
        assert!(check_rules_for_target(&rules, "windows", "x64"));
        assert!(check_rules_for_target(&rules, "linux", "x64"));
        assert!(!check_rules_for_target(&rules, "osx", "x64"));
    }

    #[test]
    fn arch_specific_rule_evaluates_correctly() {
        let rules = vec![RuleEntry {
            action: "allow".into(),
            os: Some(OsRule {
                name: Some("osx".into()),
                version: None,
                arch: Some("arm64".into()),
            }),
        }];
        assert!(check_rules_for_target(&rules, "osx", "arm64"));
        assert!(!check_rules_for_target(&rules, "osx", "x64"));
        assert!(!check_rules_for_target(&rules, "windows", "arm64"));
    }

    #[test]
    fn should_include_library_rules_returns_true_when_none() {
        assert!(should_include_library_rules(None));
    }
}
