use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::{AppHandle, Emitter};

use super::types::{
    InstallerDependencyName, InstallerDependencyState, InstallerDependencyStatus,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManualInstallCommandGroup {
    pub name: InstallerDependencyName,
    pub title: String,
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallProgressStage {
    Queued,
    Downloading,
    Installing,
    Verifying,
    Completed,
    Failed,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallExecutionStep {
    pub name: InstallerDependencyName,
    pub stage: InstallProgressStage,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallerRunResult {
    pub steps: Vec<InstallExecutionStep>,
    pub completed_dependencies: Vec<InstallerDependencyName>,
    pub failed_dependencies: Vec<InstallerDependencyName>,
    pub manual_dependencies: Vec<InstallerDependencyName>,
    pub status_message: String,
}

pub fn build_install_plan(
    dependencies: &[InstallerDependencyStatus],
) -> Vec<InstallerDependencyName> {
    let mut needs_node = false;
    let mut targets = Vec::new();

    for dependency in dependencies {
        let pending = matches!(
            dependency.state,
            InstallerDependencyState::Missing | InstallerDependencyState::Outdated
        );

        if !pending {
            continue;
        }

        match dependency.name {
            InstallerDependencyName::Node | InstallerDependencyName::Npm => {
                needs_node = true;
            }
            InstallerDependencyName::Pnpm => targets.push(InstallerDependencyName::Pnpm),
            InstallerDependencyName::Git => targets.push(InstallerDependencyName::Git),
            InstallerDependencyName::Claude => targets.push(InstallerDependencyName::Claude),
            InstallerDependencyName::Codex => targets.push(InstallerDependencyName::Codex),
            InstallerDependencyName::Gemini => targets.push(InstallerDependencyName::Gemini),
            InstallerDependencyName::Opencode => targets.push(InstallerDependencyName::Opencode),
        }
    }

    let mut ordered = Vec::new();
    if needs_node {
        ordered.push(InstallerDependencyName::Node);
    }

    for candidate in [
        InstallerDependencyName::Pnpm,
        InstallerDependencyName::Git,
        InstallerDependencyName::Claude,
        InstallerDependencyName::Codex,
        InstallerDependencyName::Gemini,
        InstallerDependencyName::Opencode,
    ] {
        if targets.contains(&candidate) {
            ordered.push(candidate);
        }
    }

    ordered
}

fn is_pending_dependency(dependency: &InstallerDependencyStatus) -> bool {
    matches!(
        dependency.state,
        InstallerDependencyState::Missing | InstallerDependencyState::Outdated
    )
}

pub fn build_selected_install_plan(
    requested: &[InstallerDependencyName],
    dependencies: &[InstallerDependencyStatus],
) -> Vec<InstallerDependencyName> {
    let mut filtered: Vec<InstallerDependencyStatus> = dependencies
        .iter()
        .filter(|dependency| requested.contains(&dependency.name))
        .cloned()
        .collect();

    let requested_dependency_needs_node = dependencies.iter().any(|dependency| {
        requested.contains(&dependency.name)
            && is_pending_dependency(dependency)
            && matches!(
                dependency.name,
                InstallerDependencyName::Pnpm
                    | InstallerDependencyName::Codex
                    | InstallerDependencyName::Gemini
            )
    });

    let node_or_npm_pending = dependencies.iter().any(|dependency| {
        matches!(
            dependency.name,
            InstallerDependencyName::Node | InstallerDependencyName::Npm
        ) && is_pending_dependency(dependency)
    });

    if requested_dependency_needs_node && node_or_npm_pending {
        filtered.push(InstallerDependencyStatus {
            name: InstallerDependencyName::Node,
            kind: super::types::InstallerDependencyKind::Core,
            state: InstallerDependencyState::Missing,
            version: None,
            path: None,
            message: None,
            auto_install_supported: true,
        });
    }

    build_install_plan(&filtered)
}

pub fn get_manual_install_commands(platform: &str) -> Vec<ManualInstallCommandGroup> {
    let node_command = match platform {
        "linux" => "Install Node.js with your package manager or nvm.",
        "windows" => "Download Node.js LTS from https://nodejs.org/en/download",
        _ => "Download Node.js LTS from https://nodejs.org/en/download",
    };

    let git_command = match platform {
        "linux" => "Install Git with your distro package manager.",
        "macos" | "darwin" => "Install Xcode Command Line Tools or Homebrew Git.",
        _ => "Install Git from https://git-scm.com/downloads",
    };

    vec![
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Node,
            title: "Node.js".to_string(),
            commands: vec![node_command.to_string()],
        },
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Pnpm,
            title: "pnpm".to_string(),
            commands: vec!["npm i -g pnpm@latest".to_string()],
        },
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Git,
            title: "Git".to_string(),
            commands: vec![git_command.to_string()],
        },
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Claude,
            title: "Claude Code".to_string(),
            commands: vec!["curl -fsSL https://claude.ai/install.sh | bash".to_string()],
        },
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Codex,
            title: "Codex".to_string(),
            commands: vec!["npm i -g @openai/codex@latest".to_string()],
        },
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Gemini,
            title: "Gemini CLI".to_string(),
            commands: vec!["npm i -g @google/gemini-cli@latest".to_string()],
        },
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Opencode,
            title: "OpenCode".to_string(),
            commands: vec!["curl -fsSL https://opencode.ai/install | bash".to_string()],
        },
    ]
}

pub fn normalize_install_result(steps: Vec<InstallExecutionStep>) -> InstallerRunResult {
    let completed_dependencies = steps
        .iter()
        .filter(|step| step.stage == InstallProgressStage::Completed)
        .map(|step| step.name)
        .collect();
    let failed_dependencies = steps
        .iter()
        .filter(|step| step.stage == InstallProgressStage::Failed)
        .map(|step| step.name)
        .collect();
    let manual_dependencies = steps
        .iter()
        .filter(|step| step.stage == InstallProgressStage::Manual)
        .map(|step| step.name)
        .collect();

    InstallerRunResult {
        steps,
        completed_dependencies,
        failed_dependencies,
        manual_dependencies,
        status_message: "Installer run completed.".to_string(),
    }
}

async fn run_install_command(command: &str, args: &[&str]) -> Result<(), String> {
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

async fn install_dependency(
    dependency: InstallerDependencyName,
    platform: &str,
) -> Result<String, String> {
    match dependency {
        InstallerDependencyName::Node => match platform {
            "windows" => Err(
                "Node MSI flow must be implemented with a downloaded installer package."
                    .to_string(),
            ),
            "macos" | "darwin" => Err(
                "Node PKG flow must be implemented with a downloaded installer package."
                    .to_string(),
            ),
            _ => Err("Node auto-install is not supported on this platform.".to_string()),
        },
        InstallerDependencyName::Git => match platform {
            "windows" => run_install_command(
                "winget",
                &["install", "--id", "Git.Git", "-e", "--source", "winget"],
            )
            .await
            .map(|_| "Installed Git with winget.".to_string()),
            "macos" | "darwin" => Err("Git requires manual install on macOS.".to_string()),
            _ => Err("Git auto-install is not supported on this platform.".to_string()),
        },
        InstallerDependencyName::Claude => {
            if platform == "windows" {
                run_install_command(
                    "powershell",
                    &[
                        "-NoProfile",
                        "-ExecutionPolicy",
                        "Bypass",
                        "-Command",
                        "irm https://claude.ai/install.ps1 | iex",
                    ],
                )
                .await
                .map(|_| "Installed Claude Code.".to_string())
            } else {
                run_install_command("sh", &["-lc", "curl -fsSL https://claude.ai/install.sh | bash"])
                    .await
                    .map(|_| "Installed Claude Code.".to_string())
            }
        }
        InstallerDependencyName::Codex => {
            run_install_command("npm", &["i", "-g", "@openai/codex@latest"])
                .await
                .map(|_| "Installed Codex.".to_string())
        }
        InstallerDependencyName::Gemini => {
            run_install_command("npm", &["i", "-g", "@google/gemini-cli@latest"])
                .await
                .map(|_| "Installed Gemini CLI.".to_string())
        }
        InstallerDependencyName::Opencode => {
            if platform == "windows" {
                Err("OpenCode auto-install is not supported on Windows in v1.".to_string())
            } else {
                run_install_command("sh", &["-lc", "curl -fsSL https://opencode.ai/install | bash"])
                    .await
                    .map(|_| "Installed OpenCode.".to_string())
            }
        }
        InstallerDependencyName::Npm => {
            Ok("npm is satisfied by the Node.js installation.".to_string())
        }
        InstallerDependencyName::Pnpm => {
            run_install_command("npm", &["i", "-g", "pnpm@latest"])
                .await
                .map(|_| "Installed pnpm.".to_string())
        }
    }
}

fn install_stage_from_error(error: &str) -> InstallProgressStage {
    let lower = error.to_lowercase();
    if lower.contains("manual") || lower.contains("not supported") {
        InstallProgressStage::Manual
    } else {
        InstallProgressStage::Failed
    }
}

fn progress_message(name: InstallerDependencyName) -> String {
    format!("Preparing {name:?} installation...")
}

async fn execute_install_plan(
    app: &AppHandle,
    plan: Vec<InstallerDependencyName>,
) -> Result<InstallerRunResult, String> {
    let platform = std::env::consts::OS;
    let mut steps = Vec::new();

    for dependency in plan {
        let queued = InstallExecutionStep {
            name: dependency,
            stage: InstallProgressStage::Queued,
            message: progress_message(dependency),
        };
        let _ = app.emit("installer-progress", &queued);
        steps.push(queued);

        let outcome = install_dependency(dependency, platform).await;
        let finished = match outcome {
            Ok(message) => InstallExecutionStep {
                name: dependency,
                stage: InstallProgressStage::Completed,
                message,
            },
            Err(error) => InstallExecutionStep {
                name: dependency,
                stage: install_stage_from_error(&error),
                message: error,
            },
        };
        let _ = app.emit("installer-progress", &finished);
        steps.push(finished);
    }

    Ok(normalize_install_result(steps))
}

pub async fn install_missing_dependencies(app: &AppHandle) -> Result<InstallerRunResult, String> {
    let environment = super::detect::detect_installer_environment();
    let plan = build_install_plan(&environment.dependencies);
    execute_install_plan(app, plan).await
}

pub async fn install_selected_dependencies(
    app: &AppHandle,
    requested: &[InstallerDependencyName],
) -> Result<InstallerRunResult, String> {
    let environment = super::detect::detect_installer_environment();
    let plan = build_selected_install_plan(requested, &environment.dependencies);
    execute_install_plan(app, plan).await
}

#[cfg(test)]
mod tests {
    use super::{
        build_install_plan, build_selected_install_plan, get_manual_install_commands,
    };
    use crate::services::installer::types::{
        InstallerDependencyKind, InstallerDependencyName, InstallerDependencyState,
        InstallerDependencyStatus,
    };

    fn status(
        name: InstallerDependencyName,
        kind: InstallerDependencyKind,
        state: InstallerDependencyState,
    ) -> InstallerDependencyStatus {
        InstallerDependencyStatus {
            name,
            kind,
            state,
            version: None,
            path: None,
            message: None,
            auto_install_supported: true,
        }
    }

    #[test]
    fn install_plan_puts_node_before_tooling() {
        let plan = build_install_plan(&[
            status(
                InstallerDependencyName::Node,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Missing,
            ),
            status(
                InstallerDependencyName::Codex,
                InstallerDependencyKind::Tool,
                InstallerDependencyState::Missing,
            ),
        ]);

        assert_eq!(
            plan,
            vec![
                InstallerDependencyName::Node,
                InstallerDependencyName::Codex,
            ]
        );
    }

    #[test]
    fn install_plan_treats_missing_npm_as_node_install() {
        let plan = build_install_plan(&[
            status(
                InstallerDependencyName::Npm,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Missing,
            ),
            status(
                InstallerDependencyName::Gemini,
                InstallerDependencyKind::Tool,
                InstallerDependencyState::Missing,
            ),
        ]);

        assert_eq!(
            plan,
            vec![
                InstallerDependencyName::Node,
                InstallerDependencyName::Gemini,
            ]
        );
    }

    #[test]
    fn install_plan_puts_pnpm_after_node_when_both_are_missing() {
        let plan = build_install_plan(&[
            status(
                InstallerDependencyName::Node,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Missing,
            ),
            status(
                InstallerDependencyName::Pnpm,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Missing,
            ),
        ]);

        assert_eq!(
            plan,
            vec![
                InstallerDependencyName::Node,
                InstallerDependencyName::Pnpm,
            ]
        );
    }

    #[test]
    fn selected_install_plan_puts_node_before_requested_pnpm() {
        let plan = build_selected_install_plan(
            &[InstallerDependencyName::Pnpm],
            &[
                status(
                    InstallerDependencyName::Node,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Missing,
                ),
                status(
                    InstallerDependencyName::Pnpm,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Missing,
                ),
            ],
        );

        assert_eq!(
            plan,
            vec![
                InstallerDependencyName::Node,
                InstallerDependencyName::Pnpm,
            ]
        );
    }

    #[test]
    fn selected_install_plan_adds_node_for_npm_backed_tools() {
        let plan = build_selected_install_plan(
            &[InstallerDependencyName::Codex],
            &[
                status(
                    InstallerDependencyName::Npm,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Missing,
                ),
                status(
                    InstallerDependencyName::Codex,
                    InstallerDependencyKind::Tool,
                    InstallerDependencyState::Missing,
                ),
            ],
        );

        assert_eq!(
            plan,
            vec![
                InstallerDependencyName::Node,
                InstallerDependencyName::Codex,
            ]
        );
    }

    #[test]
    fn linux_manual_commands_include_all_tools() {
        let commands = get_manual_install_commands("linux");

        assert!(commands
            .iter()
            .any(|item| item.name == InstallerDependencyName::Claude));
        assert!(commands
            .iter()
            .any(|item| item.name == InstallerDependencyName::Codex));
        assert!(commands
            .iter()
            .any(|item| item.name == InstallerDependencyName::Gemini));
        assert!(commands
            .iter()
            .any(|item| item.name == InstallerDependencyName::Opencode));
        assert!(commands
            .iter()
            .any(|item| item.name == InstallerDependencyName::Pnpm));
    }

    #[test]
    fn normalize_install_result_collects_completed_and_failed_dependencies() {
        let result = super::normalize_install_result(vec![
            super::InstallExecutionStep {
                name: InstallerDependencyName::Node,
                stage: super::InstallProgressStage::Completed,
                message: "Installed Node.js.".to_string(),
            },
            super::InstallExecutionStep {
                name: InstallerDependencyName::Claude,
                stage: super::InstallProgressStage::Failed,
                message: "claude installer exited with code 1".to_string(),
            },
            super::InstallExecutionStep {
                name: InstallerDependencyName::Git,
                stage: super::InstallProgressStage::Manual,
                message: "Git requires manual install on macOS.".to_string(),
            },
        ]);

        assert_eq!(
            result.completed_dependencies,
            vec![InstallerDependencyName::Node]
        );
        assert_eq!(
            result.failed_dependencies,
            vec![InstallerDependencyName::Claude]
        );
        assert_eq!(result.manual_dependencies, vec![InstallerDependencyName::Git]);
        assert_eq!(result.steps.len(), 3);
    }
}
