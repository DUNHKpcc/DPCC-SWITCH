use std::process::Command;
use std::{env, ffi::OsString, path::Path};

use super::types::{
    InstallerDependencyKind, InstallerDependencyName, InstallerDependencyState,
    InstallerDependencyStatus,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeInstallCapability {
    pub auto_install_supported: bool,
    pub fallback_state: InstallerDependencyState,
    pub message: Option<String>,
    pub brew_binary: Option<String>,
    pub winget_available: bool,
    pub windows_elevated: bool,
}

pub fn resolve_node_install_capability(
    platform: &str,
    winget_available: bool,
    windows_elevated: bool,
    brew_binary: Option<String>,
) -> NodeInstallCapability {
    match platform {
        "windows" => {
            if winget_available && windows_elevated {
                NodeInstallCapability {
                    auto_install_supported: true,
                    fallback_state: InstallerDependencyState::Missing,
                    message: Some("Node.js will be installed with winget.".to_string()),
                    brew_binary,
                    winget_available,
                    windows_elevated,
                }
            } else {
                NodeInstallCapability {
                    auto_install_supported: false,
                    fallback_state: InstallerDependencyState::Manual,
                    message: Some(
                        "Node.js auto-install on Windows requires winget and DPCC-SWITCH to be reopened as administrator."
                            .to_string(),
                    ),
                    brew_binary,
                    winget_available,
                    windows_elevated,
                }
            }
        }
        "macos" | "darwin" => {
            let message = if brew_binary.is_some() {
                "Node.js will be installed with Homebrew.".to_string()
            } else {
                "Homebrew is missing. DPCC-SWITCH will install Homebrew from the domestic mirror before installing Node.js."
                    .to_string()
            };

            NodeInstallCapability {
                auto_install_supported: true,
                fallback_state: InstallerDependencyState::Missing,
                message: Some(message),
                brew_binary,
                winget_available,
                windows_elevated,
            }
        }
        _ => NodeInstallCapability {
            auto_install_supported: false,
            fallback_state: InstallerDependencyState::Manual,
            message: Some("Node.js auto-install is not supported on this platform.".to_string()),
            brew_binary,
            winget_available,
            windows_elevated,
        },
    }
}

pub fn detect_node_install_capability() -> NodeInstallCapability {
    let platform = std::env::consts::OS;
    let brew_binary = detect_homebrew_binary();
    let winget_available = detect_binary_available("winget");
    let windows_elevated = detect_windows_elevation();

    resolve_node_install_capability(platform, winget_available, windows_elevated, brew_binary)
}

pub fn detect_binary_available(binary: &str) -> bool {
    let output = if cfg!(target_os = "windows") {
        Command::new("where").arg(binary).output()
    } else {
        Command::new("which").arg(binary).output()
    };

    output
        .ok()
        .is_some_and(|output| output.status.success())
}

pub fn detect_homebrew_binary() -> Option<String> {
    for candidate in ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"] {
        if std::path::Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }

    if detect_binary_available("brew") {
        Some("brew".to_string())
    } else {
        None
    }
}

pub fn detect_windows_elevation() -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("net").arg("session").output().is_ok_and(|output| output.status.success())
    }

    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

pub fn build_windows_node_install_command() -> (&'static str, Vec<&'static str>) {
    (
        "winget",
        vec![
            "install",
            "--id",
            "OpenJS.NodeJS.LTS",
            "-e",
            "--source",
            "winget",
            "--accept-package-agreements",
            "--accept-source-agreements",
        ],
    )
}

pub fn build_homebrew_bootstrap_script() -> String {
    [
        "set -euo pipefail",
        "export NONINTERACTIVE=1",
        "export HOMEBREW_BREW_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git",
        "export HOMEBREW_CORE_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git",
        "export HOMEBREW_API_DOMAIN=https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api",
        "tmpdir=\"$(mktemp -d)\"",
        "trap 'rm -rf \"$tmpdir\"' EXIT",
        "git clone --depth=1 https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git \"$tmpdir/install\"",
        "/bin/bash \"$tmpdir/install/install.sh\"",
    ]
    .join("\n")
}

pub fn evaluate_node_runtime_verification(
    node_version: Option<String>,
    npm_version: Option<String>,
) -> InstallerDependencyStatus {
    match (node_version, npm_version) {
        (Some(node_version), Some(_npm_version)) => InstallerDependencyStatus {
            name: InstallerDependencyName::Node,
            kind: InstallerDependencyKind::Core,
            state: InstallerDependencyState::Installed,
            version: Some(node_version),
            path: None,
            message: None,
            auto_install_supported: true,
        },
        (Some(node_version), None) => InstallerDependencyStatus {
            name: InstallerDependencyName::Node,
            kind: InstallerDependencyKind::Core,
            state: InstallerDependencyState::Broken,
            version: Some(node_version),
            path: None,
            message: Some(
                "Node.js is available on PATH, but npm is missing. Reinstall Node.js to repair npm."
                    .to_string(),
            ),
            auto_install_supported: true,
        },
        _ => InstallerDependencyStatus {
            name: InstallerDependencyName::Node,
            kind: InstallerDependencyKind::Core,
            state: InstallerDependencyState::Missing,
            version: None,
            path: None,
            message: Some(
                "Node.js install command finished, but node and npm are still missing on PATH."
                    .to_string(),
            ),
            auto_install_supported: true,
        },
    }
}

pub fn verification_search_paths(
    platform: &str,
    capability: &NodeInstallCapability,
) -> Vec<String> {
    let mut paths = Vec::new();

    match platform {
        "windows" => {
            paths.push("C:\\Program Files\\nodejs".to_string());
        }
        "macos" | "darwin" => {
            if let Some(brew_binary) = capability.brew_binary.as_deref() {
                if let Some(parent) = Path::new(brew_binary).parent() {
                    let parent = parent.to_string_lossy().to_string();
                    if !parent.is_empty() {
                        paths.push(parent);
                    }
                }
            }

            for candidate in ["/opt/homebrew/bin", "/usr/local/bin"] {
                if !paths.iter().any(|path| path == candidate) {
                    paths.push(candidate.to_string());
                }
            }
        }
        _ => {}
    }

    paths
}

pub fn prepend_node_verification_paths_to_process_path(capability: &NodeInstallCapability) {
    let mut entries: Vec<OsString> = verification_search_paths(std::env::consts::OS, capability)
        .into_iter()
        .map(OsString::from)
        .collect();

    let current = env::var_os("PATH").unwrap_or_default();
    entries.extend(env::split_paths(&current).map(|path| path.into_os_string()));

    if let Ok(joined) = env::join_paths(entries) {
        unsafe {
            env::set_var("PATH", joined);
        }
    }
}

async fn run_command(command: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(command)
        .args(args)
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("{command} exited with code {:?}", status.code()))
    }
}

fn shell_escape_single_quotes(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

fn escape_osascript(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "macos")]
fn run_macos_authorized_script(script: &str) -> Result<(), String> {
    let shell_command = format!("/bin/bash -lc {}", shell_escape_single_quotes(script));
    let apple_script = format!(
        r#"do shell script "{}" with administrator privileges"#,
        escape_osascript(&shell_command)
    );
    let status = Command::new("osascript")
        .arg("-e")
        .arg(apple_script)
        .status()
        .map_err(|error| error.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("Homebrew bootstrap authorization failed.".to_string())
    }
}

#[cfg(not(target_os = "macos"))]
fn run_macos_authorized_script(_script: &str) -> Result<(), String> {
    Err("Homebrew bootstrap authorization is only available on macOS builds.".to_string())
}

pub async fn install_node_runtime<F>(
    capability: &NodeInstallCapability,
    mut emit_progress: F,
) -> Result<String, String>
where
    F: FnMut(String),
{
    match std::env::consts::OS {
        "windows" => {
            emit_progress("Checking winget availability...".to_string());
            emit_progress("Checking administrator privileges...".to_string());
            if !capability.auto_install_supported {
                return Err(capability.message.clone().unwrap_or_else(|| {
                    "Node.js auto-install on Windows requires winget and administrator privileges."
                        .to_string()
                }));
            }

            emit_progress("Installing Node.js with winget...".to_string());
            let (command, args) = build_windows_node_install_command();
            run_command(command, &args).await?;
            Ok("Installed Node.js with winget.".to_string())
        }
        "macos" | "darwin" => {
            emit_progress("Checking Homebrew availability...".to_string());
            let brew_binary = if let Some(brew_binary) = capability.brew_binary.clone() {
                brew_binary
            } else {
                emit_progress("Installing Homebrew from domestic mirror...".to_string());
                run_macos_authorized_script(&build_homebrew_bootstrap_script())?;
                detect_homebrew_binary().ok_or_else(|| {
                    "Homebrew installation finished, but brew is still missing on PATH."
                        .to_string()
                })?
            };

            emit_progress("Installing Node.js with Homebrew...".to_string());
            run_command(&brew_binary, &["install", "node"]).await?;
            Ok("Installed Node.js with Homebrew.".to_string())
        }
        _ => Err("Node auto-install is not supported on this platform.".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_homebrew_bootstrap_script, build_windows_node_install_command,
        evaluate_node_runtime_verification, resolve_node_install_capability,
        verification_search_paths, NodeInstallCapability,
    };
    use crate::services::installer::types::{InstallerDependencyState, InstallerDependencyStatus};

    #[test]
    fn windows_requires_winget_and_elevation_for_node_auto_install() {
        let capability = resolve_node_install_capability("windows", false, false, None);

        assert_eq!(
            capability,
            NodeInstallCapability {
                auto_install_supported: false,
                fallback_state: InstallerDependencyState::Manual,
                message: Some(
                    "Node.js auto-install on Windows requires winget and DPCC-SWITCH to be reopened as administrator."
                        .to_string(),
                ),
                brew_binary: None,
                winget_available: false,
                windows_elevated: false,
            }
        );
    }

    #[test]
    fn macos_without_brew_stays_auto_installable_through_mirror_bootstrap() {
        let capability = resolve_node_install_capability("macos", false, false, None);

        assert_eq!(
            capability,
            NodeInstallCapability {
                auto_install_supported: true,
                fallback_state: InstallerDependencyState::Missing,
                message: Some(
                    "Homebrew is missing. DPCC-SWITCH will install Homebrew from the domestic mirror before installing Node.js."
                        .to_string(),
                ),
                brew_binary: None,
                winget_available: false,
                windows_elevated: false,
            }
        );
    }

    #[test]
    fn windows_node_install_command_uses_winget_lts_with_accept_flags() {
        let (command, args) = build_windows_node_install_command();

        assert_eq!(command, "winget");
        assert_eq!(
            args,
            vec![
                "install",
                "--id",
                "OpenJS.NodeJS.LTS",
                "-e",
                "--source",
                "winget",
                "--accept-package-agreements",
                "--accept-source-agreements",
            ]
        );
    }

    #[test]
    fn homebrew_bootstrap_script_uses_tuna_mirror_sources() {
        let script = build_homebrew_bootstrap_script();

        assert!(script.contains("https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git"));
        assert!(script.contains(
            "HOMEBREW_BREW_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"
        ));
        assert!(script.contains(
            "HOMEBREW_CORE_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"
        ));
        assert!(script.contains(
            "HOMEBREW_API_DOMAIN=https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api"
        ));
    }

    #[test]
    fn verify_node_runtime_marks_partial_runtime_as_broken() {
        let status =
            evaluate_node_runtime_verification(Some("v22.18.0".to_string()), None::<String>);

        assert_eq!(
            status,
            InstallerDependencyStatus {
                name: crate::services::installer::types::InstallerDependencyName::Node,
                kind: crate::services::installer::types::InstallerDependencyKind::Core,
                state: InstallerDependencyState::Broken,
                version: Some("v22.18.0".to_string()),
                path: None,
                message: Some(
                    "Node.js is available on PATH, but npm is missing. Reinstall Node.js to repair npm."
                        .to_string(),
                ),
                auto_install_supported: true,
            }
        );
    }

    #[test]
    fn windows_verification_search_paths_include_standard_node_install_dir() {
        let capability = resolve_node_install_capability("windows", true, true, None);

        let paths = verification_search_paths("windows", &capability);

        assert!(paths.contains(&"C:\\Program Files\\nodejs".to_string()));
    }

    #[test]
    fn macos_verification_search_paths_include_brew_bin_dir() {
        let capability = resolve_node_install_capability(
            "macos",
            false,
            false,
            Some("/opt/homebrew/bin/brew".to_string()),
        );

        let paths = verification_search_paths("macos", &capability);

        assert!(paths.contains(&"/opt/homebrew/bin".to_string()));
    }

    #[test]
    fn verify_node_runtime_installed_status_does_not_use_fake_path_placeholder() {
        let status = evaluate_node_runtime_verification(
            Some("v22.18.0".to_string()),
            Some("10.8.3".to_string()),
        );

        assert_eq!(
            status,
            InstallerDependencyStatus {
                name: crate::services::installer::types::InstallerDependencyName::Node,
                kind: crate::services::installer::types::InstallerDependencyKind::Core,
                state: InstallerDependencyState::Installed,
                version: Some("v22.18.0".to_string()),
                path: None,
                message: None,
                auto_install_supported: true,
            }
        );
    }
}
