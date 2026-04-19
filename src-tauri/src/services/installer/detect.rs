use std::process::Command;

use chrono::Utc;

use super::node_runtime::{detect_node_install_capability, NodeInstallCapability};
use super::types::{
    InstallerDependencyKind, InstallerDependencyName, InstallerDependencyState,
    InstallerDependencyStatus, InstallerEnvironment,
};

pub fn auto_install_supported_on_platform(
    platform: &str,
    name: InstallerDependencyName,
) -> bool {
    match name {
        InstallerDependencyName::Node | InstallerDependencyName::Npm => false,
        InstallerDependencyName::Git => platform == "windows",
        InstallerDependencyName::Opencode => platform != "windows",
        InstallerDependencyName::Pnpm
        | InstallerDependencyName::Claude
        | InstallerDependencyName::Codex
        | InstallerDependencyName::Gemini => true,
    }
}

pub fn detect_dependency_from_output(
    name: InstallerDependencyName,
    kind: InstallerDependencyKind,
    version: Option<String>,
    path: Option<String>,
    message: Option<String>,
    auto_install_supported: bool,
) -> InstallerDependencyStatus {
    let state = if version.is_some() && path.is_some() {
        InstallerDependencyState::Installed
    } else if version.is_some() || path.is_some() {
        InstallerDependencyState::Broken
    } else if !auto_install_supported && message.is_some() {
        InstallerDependencyState::Manual
    } else {
        InstallerDependencyState::Missing
    };

    InstallerDependencyStatus {
        name,
        kind,
        state,
        version,
        path,
        message,
        auto_install_supported,
    }
}

pub fn finalize_detected_dependencies(
    dependencies: Vec<InstallerDependencyStatus>,
    capability: &NodeInstallCapability,
) -> Vec<InstallerDependencyStatus> {
    let node_installed = dependencies.iter().any(|dependency| {
        dependency.name == InstallerDependencyName::Node
            && dependency.state == InstallerDependencyState::Installed
    });
    let npm_installed = dependencies.iter().any(|dependency| {
        dependency.name == InstallerDependencyName::Npm
            && dependency.state == InstallerDependencyState::Installed
    });

    dependencies
        .into_iter()
        .map(|mut dependency| {
            match dependency.name {
                InstallerDependencyName::Node
                    if dependency.state == InstallerDependencyState::Missing && npm_installed =>
                {
                    dependency.state = InstallerDependencyState::Broken;
                    dependency.auto_install_supported = false;
                    dependency.message = Some(
                        "npm is available on PATH, but node is missing. Reinstall Node.js to repair the runtime."
                            .to_string(),
                    );
                }
                InstallerDependencyName::Npm
                    if dependency.state == InstallerDependencyState::Missing && node_installed =>
                {
                    dependency.state = InstallerDependencyState::Broken;
                    dependency.auto_install_supported = false;
                    dependency.message = Some(
                        "Node.js is available on PATH, but npm is missing. Reinstall Node.js to repair npm."
                            .to_string(),
                    );
                }
                InstallerDependencyName::Node | InstallerDependencyName::Npm
                    if dependency.state != InstallerDependencyState::Installed
                        && dependency.state != InstallerDependencyState::Broken =>
                {
                    dependency.state = capability.fallback_state;
                    dependency.auto_install_supported = capability.auto_install_supported;
                    dependency.message = capability.message.clone();
                }
                InstallerDependencyName::Pnpm
                | InstallerDependencyName::Codex
                | InstallerDependencyName::Gemini
                    if dependency.state != InstallerDependencyState::Installed && !npm_installed =>
                {
                    if dependency.state != InstallerDependencyState::Broken {
                        dependency.state = InstallerDependencyState::Manual;
                    }
                    dependency.auto_install_supported = false;
                    dependency.message = Some(
                        "npm is not available on PATH. Install Node.js first, then retry this tool."
                            .to_string(),
                    );
                }
                _ => {}
            }

            dependency
        })
        .collect()
}

fn dependency_binary_name(name: InstallerDependencyName) -> &'static str {
    match name {
        InstallerDependencyName::Node => "node",
        InstallerDependencyName::Npm => "npm",
        InstallerDependencyName::Pnpm => "pnpm",
        InstallerDependencyName::Git => "git",
        InstallerDependencyName::Claude => "claude",
        InstallerDependencyName::Codex => "codex",
        InstallerDependencyName::Gemini => "gemini",
        InstallerDependencyName::Opencode => "opencode",
    }
}

fn dependency_kind(name: InstallerDependencyName) -> InstallerDependencyKind {
    match name {
        InstallerDependencyName::Node
        | InstallerDependencyName::Npm
        | InstallerDependencyName::Pnpm
        | InstallerDependencyName::Git => InstallerDependencyKind::Core,
        InstallerDependencyName::Claude
        | InstallerDependencyName::Codex
        | InstallerDependencyName::Gemini
        | InstallerDependencyName::Opencode => InstallerDependencyKind::Tool,
    }
}

pub fn detect_dependency_status(name: InstallerDependencyName) -> InstallerDependencyStatus {
    detect_binary(
        dependency_binary_name(name),
        name,
        dependency_kind(name),
    )
}

pub fn detect_installer_environment() -> InstallerEnvironment {
    let platform = std::env::consts::OS.to_string();
    let capability = detect_node_install_capability();
    let dependencies = finalize_detected_dependencies(vec![
        detect_dependency_status(InstallerDependencyName::Node),
        detect_dependency_status(InstallerDependencyName::Npm),
        detect_dependency_status(InstallerDependencyName::Pnpm),
        detect_dependency_status(InstallerDependencyName::Git),
        detect_dependency_status(InstallerDependencyName::Claude),
        detect_dependency_status(InstallerDependencyName::Codex),
        detect_dependency_status(InstallerDependencyName::Gemini),
        detect_dependency_status(InstallerDependencyName::Opencode),
    ], &capability);

    let ready_count = dependencies
        .iter()
        .filter(|dependency| dependency.state == InstallerDependencyState::Installed)
        .count();

    InstallerEnvironment {
        platform: platform.clone(),
        auto_install_supported: dependencies
            .iter()
            .any(|dependency| dependency.auto_install_supported),
        last_checked_at: Utc::now().to_rfc3339(),
        total_count: dependencies.len(),
        ready_count,
        dependencies,
    }
}

fn detect_binary(
    binary: &str,
    name: InstallerDependencyName,
    kind: InstallerDependencyKind,
) -> InstallerDependencyStatus {
    let auto_install_supported =
        auto_install_supported_on_platform(std::env::consts::OS, name);

    let version_output = Command::new(binary).arg("--version").output();
    let path_output = if cfg!(target_os = "windows") {
        Command::new("where").arg(binary).output()
    } else {
        Command::new("which").arg(binary).output()
    };

    let version = version_output
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|value| !value.is_empty());

    let path = path_output
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| {
            output
                .stdout
                .split(|byte| *byte == b'\n')
                .next()
                .map(|line| String::from_utf8_lossy(line).trim().to_string())
        })
        .filter(|value| !value.is_empty());

    let message = if version.is_some() && path.is_some() {
        None
    } else if version.is_some() || path.is_some() {
        Some(format!(
            "{binary} returned partial detection results. Reinstall the dependency to repair it."
        ))
    } else {
        Some(format!("{binary} was not found on PATH."))
    };

    detect_dependency_from_output(
        name,
        kind,
        version,
        path,
        message,
        auto_install_supported,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        auto_install_supported_on_platform, finalize_detected_dependencies,
        detect_dependency_from_output, InstallerDependencyKind, InstallerDependencyName,
        InstallerDependencyState, InstallerDependencyStatus,
    };
    use super::super::node_runtime::NodeInstallCapability;

    #[test]
    fn detects_missing_binary_as_missing() {
        let status = detect_dependency_from_output(
            InstallerDependencyName::Codex,
            InstallerDependencyKind::Tool,
            None,
            None,
            Some("codex was not found on PATH.".to_string()),
            true,
        );

        assert_eq!(status.state, InstallerDependencyState::Missing);
        assert_eq!(status.version.as_deref(), None);
    }

    #[test]
    fn detects_present_binary_as_installed() {
        let status = detect_dependency_from_output(
            InstallerDependencyName::Node,
            InstallerDependencyKind::Core,
            Some("v22.22.2".to_string()),
            Some("/usr/local/bin/node".to_string()),
            None,
            true,
        );

        assert_eq!(status.state, InstallerDependencyState::Installed);
        assert_eq!(status.path.as_deref(), Some("/usr/local/bin/node"));
    }

    #[test]
    fn marks_manual_only_dependency_as_manual_when_auto_install_is_unsupported() {
        let status = detect_dependency_from_output(
            InstallerDependencyName::Git,
            InstallerDependencyKind::Core,
            None,
            None,
            Some("Manual install required for git on this platform.".to_string()),
            false,
        );

        assert_eq!(status.state, InstallerDependencyState::Manual);
        assert!(!status.auto_install_supported);
    }

    #[test]
    fn marks_partial_detection_results_as_broken() {
        let status = detect_dependency_from_output(
            InstallerDependencyName::Codex,
            InstallerDependencyKind::Tool,
            Some("codex 0.42.0".to_string()),
            None,
            Some("codex returned partial detection results.".to_string()),
            true,
        );

        assert_eq!(status.state, InstallerDependencyState::Broken);
    }

    fn status(
        name: InstallerDependencyName,
        kind: InstallerDependencyKind,
        state: InstallerDependencyState,
        auto_install_supported: bool,
    ) -> InstallerDependencyStatus {
        InstallerDependencyStatus {
            name,
            kind,
            state,
            version: None,
            path: None,
            message: None,
            auto_install_supported,
        }
    }

    #[test]
    fn platform_support_matrix_matches_current_installer_capabilities() {
        assert!(!auto_install_supported_on_platform(
            "macos",
            InstallerDependencyName::Node,
        ));
        assert!(!auto_install_supported_on_platform(
            "linux",
            InstallerDependencyName::Git,
        ));
        assert!(auto_install_supported_on_platform(
            "linux",
            InstallerDependencyName::Claude,
        ));
        assert!(auto_install_supported_on_platform(
            "linux",
            InstallerDependencyName::Codex,
        ));
        assert!(auto_install_supported_on_platform(
            "windows",
            InstallerDependencyName::Git,
        ));
    }

    #[test]
    fn npm_backed_tools_become_manual_when_npm_is_missing() {
        let capability = NodeInstallCapability {
            auto_install_supported: false,
            fallback_state: InstallerDependencyState::Manual,
            message: Some("Node.js auto-install is not supported on this platform.".to_string()),
            brew_binary: None,
            winget_available: false,
            windows_elevated: false,
        };

        let statuses = finalize_detected_dependencies(vec![
            status(
                InstallerDependencyName::Node,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Manual,
                false,
            ),
            status(
                InstallerDependencyName::Npm,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Manual,
                false,
            ),
            status(
                InstallerDependencyName::Codex,
                InstallerDependencyKind::Tool,
                InstallerDependencyState::Missing,
                true,
            ),
            status(
                InstallerDependencyName::Gemini,
                InstallerDependencyKind::Tool,
                InstallerDependencyState::Missing,
                true,
            ),
            status(
                InstallerDependencyName::Pnpm,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Missing,
                true,
            ),
        ], &capability);

        for name in [
            InstallerDependencyName::Codex,
            InstallerDependencyName::Gemini,
            InstallerDependencyName::Pnpm,
        ] {
            let status = statuses
                .iter()
                .find(|status| status.name == name)
                .expect("dependency status");

            assert_eq!(status.state, InstallerDependencyState::Manual);
            assert!(!status.auto_install_supported);
            assert!(status
                .message
                .as_deref()
                .is_some_and(|message| message.contains("npm")));
        }
    }

    #[test]
    fn node_and_npm_become_manual_when_windows_preflight_is_not_executable() {
        let capability = NodeInstallCapability {
            auto_install_supported: false,
            fallback_state: InstallerDependencyState::Manual,
            message: Some(
                "Node.js auto-install on Windows requires winget and DPCC-SWITCH to be reopened as administrator."
                    .to_string(),
            ),
            brew_binary: None,
            winget_available: false,
            windows_elevated: false,
        };

        let statuses = finalize_detected_dependencies(
            vec![
                status(
                    InstallerDependencyName::Node,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Missing,
                    false,
                ),
                status(
                    InstallerDependencyName::Npm,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Missing,
                    false,
                ),
            ],
            &capability,
        );

        for name in [InstallerDependencyName::Node, InstallerDependencyName::Npm] {
            let status = statuses
                .iter()
                .find(|status| status.name == name)
                .expect("dependency status");

            assert_eq!(status.state, InstallerDependencyState::Manual);
            assert!(!status.auto_install_supported);
            assert_eq!(status.message.as_deref(), capability.message.as_deref());
        }
    }

    #[test]
    fn broken_node_keeps_partial_detection_message_when_npm_is_installed() {
        let capability = NodeInstallCapability {
            auto_install_supported: true,
            fallback_state: InstallerDependencyState::Missing,
            message: Some("Node.js will be installed with winget.".to_string()),
            brew_binary: None,
            winget_available: true,
            windows_elevated: true,
        };

        let statuses = finalize_detected_dependencies(
            vec![
                InstallerDependencyStatus {
                    name: InstallerDependencyName::Node,
                    kind: InstallerDependencyKind::Core,
                    state: InstallerDependencyState::Broken,
                    version: Some("v22.0.0".to_string()),
                    path: None,
                    message: Some(
                        "node returned partial detection results. Reinstall the dependency to repair it."
                            .to_string(),
                    ),
                    auto_install_supported: false,
                },
                status(
                    InstallerDependencyName::Npm,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Installed,
                    false,
                ),
            ],
            &capability,
        );

        let node = statuses
            .iter()
            .find(|status| status.name == InstallerDependencyName::Node)
            .expect("node status");

        assert_eq!(node.state, InstallerDependencyState::Broken);
        assert_eq!(
            node.message.as_deref(),
            Some("node returned partial detection results. Reinstall the dependency to repair it.")
        );
    }
}
