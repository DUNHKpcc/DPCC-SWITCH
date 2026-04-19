use serde::{Deserialize, Serialize};
use std::process::Command;
use tauri::{AppHandle, Emitter};

use super::node_runtime::{
    detect_node_install_capability, evaluate_node_runtime_verification, install_node_runtime,
    prepend_node_verification_paths_to_process_path,
};
use super::types::{
    InstallerDependencyName, InstallerDependencyState, InstallerDependencyStatus,
    InstallerEnvironment,
};
#[cfg(test)]
use super::types::InstallerDependencyKind;

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
    let npm_backed_tools_can_be_planned = node_runtime_ready_for_npm_backed_tools(dependencies);
    let mut needs_node = false;
    let mut targets = Vec::new();

    for dependency in dependencies {
        if !is_installable_dependency(dependency) {
            continue;
        }

        match dependency.name {
            InstallerDependencyName::Node | InstallerDependencyName::Npm => {
                needs_node = true;
            }
            InstallerDependencyName::Pnpm
                if npm_backed_tools_can_be_planned =>
            {
                targets.push(InstallerDependencyName::Pnpm)
            }
            InstallerDependencyName::Git => targets.push(InstallerDependencyName::Git),
            InstallerDependencyName::Claude => targets.push(InstallerDependencyName::Claude),
            InstallerDependencyName::Codex
                if npm_backed_tools_can_be_planned =>
            {
                targets.push(InstallerDependencyName::Codex)
            }
            InstallerDependencyName::Gemini
                if npm_backed_tools_can_be_planned =>
            {
                targets.push(InstallerDependencyName::Gemini)
            }
            InstallerDependencyName::Opencode => targets.push(InstallerDependencyName::Opencode),
            _ => {}
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

fn is_installable_dependency(dependency: &InstallerDependencyStatus) -> bool {
    dependency.state == InstallerDependencyState::Missing
        && dependency.auto_install_supported
}

fn node_runtime_ready_for_npm_backed_tools(dependencies: &[InstallerDependencyStatus]) -> bool {
    let node_installed = dependencies.iter().any(|dependency| {
        dependency.name == InstallerDependencyName::Node
            && dependency.state == InstallerDependencyState::Installed
    });
    let npm_installed = dependencies.iter().any(|dependency| {
        dependency.name == InstallerDependencyName::Npm
            && dependency.state == InstallerDependencyState::Installed
    });
    let node_runtime_can_be_planned = dependencies.iter().any(|dependency| {
        matches!(
            dependency.name,
            InstallerDependencyName::Node | InstallerDependencyName::Npm
        ) && is_installable_dependency(dependency)
    });

    (node_installed && npm_installed) || node_runtime_can_be_planned
}

pub fn build_selected_install_plan(
    requested: &[InstallerDependencyName],
    dependencies: &[InstallerDependencyStatus],
) -> Vec<InstallerDependencyName> {
    let mut filtered: Vec<InstallerDependencyStatus> = dependencies
        .iter()
        .filter(|dependency| {
            requested.contains(&dependency.name) && is_installable_dependency(dependency)
        })
        .cloned()
        .collect();

    let requested_dependency_needs_node = requested.iter().any(|dependency| {
        matches!(
            dependency,
            InstallerDependencyName::Pnpm
                | InstallerDependencyName::Codex
                | InstallerDependencyName::Gemini
        )
    });

    if requested_dependency_needs_node {
        for runtime_dependency in dependencies.iter().filter(|dependency| {
            matches!(
                dependency.name,
                InstallerDependencyName::Node | InstallerDependencyName::Npm
            )
        }) {
            if !filtered
                .iter()
                .any(|dependency| dependency.name == runtime_dependency.name)
            {
                filtered.push(runtime_dependency.clone());
            }
        }
    }

    build_install_plan(&filtered)
}

fn node_manual_commands_for_environment(environment: &InstallerEnvironment) -> Vec<String> {
    let node_status = environment
        .dependencies
        .iter()
        .find(|dependency| dependency.name == InstallerDependencyName::Node);
    let node_message = node_status.and_then(|dependency| dependency.message.as_deref());

    match environment.platform.as_str() {
        "windows" => {
            let mut commands = Vec::new();

            if node_message
                .is_some_and(|message| message.contains("administrator"))
                || node_status.is_some_and(|dependency| {
                    dependency.state == InstallerDependencyState::Manual
                        || !dependency.auto_install_supported
                })
            {
                commands.push(
                    "Close DPCC-SWITCH, reopen it with Run as administrator, and retry Node.js auto-install."
                        .to_string(),
                );
            }

            if node_message
                .is_some_and(|message| message.contains("winget"))
                || node_status.is_some_and(|dependency| !dependency.auto_install_supported)
            {
                commands.push(
                    "If winget is missing, install App Installer from https://aka.ms/getwinget, then retry."
                        .to_string(),
                );
            }

            commands.push(
                "winget install --id OpenJS.NodeJS.LTS -e --source winget --accept-package-agreements --accept-source-agreements"
                    .to_string(),
            );
            commands.push("Download Node.js LTS from https://nodejs.org/en/download".to_string());
            commands
        }
        "macos" | "darwin" => {
            if node_message
                .is_some_and(|message| message.contains("Homebrew is missing"))
            {
                vec![
                    "export HOMEBREW_BREW_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"
                        .to_string(),
                    "export HOMEBREW_CORE_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"
                        .to_string(),
                    "export HOMEBREW_API_DOMAIN=https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api"
                        .to_string(),
                    "tmpdir=\"$(mktemp -d)\"".to_string(),
                    "trap 'rm -rf \"$tmpdir\"' EXIT".to_string(),
                    "git clone --depth=1 https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git \"$tmpdir/install\""
                        .to_string(),
                    "/bin/bash \"$tmpdir/install/install.sh\"".to_string(),
                    "brew install node".to_string(),
                ]
            } else {
                vec!["brew install node".to_string()]
            }
        }
        _ => vec!["Install Node.js with your package manager or nvm.".to_string()],
    }
}

fn manual_command_groups_from_environment(
    environment: &InstallerEnvironment,
    node_commands: Vec<String>,
) -> Vec<ManualInstallCommandGroup> {
    let git_command = match environment.platform.as_str() {
        "linux" => "Install Git with your distro package manager.",
        "macos" | "darwin" => "Install Xcode Command Line Tools or Homebrew Git.",
        _ => "Install Git from https://git-scm.com/downloads",
    };

    let npm_backed_commands = |install_command: &str| {
        let mut commands = node_commands.clone();
        commands.push(install_command.to_string());
        commands
    };

    vec![
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Node,
            title: "Node.js".to_string(),
            commands: node_commands.clone(),
        },
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Pnpm,
            title: "pnpm".to_string(),
            commands: npm_backed_commands("npm i -g pnpm@latest"),
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
            commands: npm_backed_commands("npm i -g @openai/codex@latest"),
        },
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Gemini,
            title: "Gemini CLI".to_string(),
            commands: npm_backed_commands("npm i -g @google/gemini-cli@latest"),
        },
        ManualInstallCommandGroup {
            name: InstallerDependencyName::Opencode,
            title: "OpenCode".to_string(),
            commands: vec!["curl -fsSL https://opencode.ai/install | bash".to_string()],
        },
    ]
}

pub fn get_manual_install_commands_for_environment(
    environment: &InstallerEnvironment,
) -> Vec<ManualInstallCommandGroup> {
    let node_commands = node_manual_commands_for_environment(environment);
    manual_command_groups_from_environment(environment, node_commands)
}

#[cfg(test)]
pub fn get_manual_install_commands(platform: &str) -> Vec<ManualInstallCommandGroup> {
    let environment = InstallerEnvironment {
        platform: platform.to_string(),
        auto_install_supported: false,
        dependencies: vec![InstallerDependencyStatus {
            name: InstallerDependencyName::Node,
            kind: InstallerDependencyKind::Core,
            state: InstallerDependencyState::Missing,
            version: None,
            path: None,
            message: None,
            auto_install_supported: false,
        }],
        last_checked_at: String::new(),
        ready_count: 0,
        total_count: 1,
    };

    get_manual_install_commands_for_environment(&environment)
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
        InstallerDependencyName::Node => Err(format!(
            "Node runtime installation must be routed through the dedicated helper on {platform}."
        )),
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

fn installing_message(name: InstallerDependencyName) -> String {
    format!("Installing {name:?}...")
}

fn verifying_message(name: InstallerDependencyName) -> String {
    format!("Verifying {name:?} on PATH...")
}

fn record_installing_substep(
    steps: &mut Vec<InstallExecutionStep>,
    name: InstallerDependencyName,
    message: String,
) -> InstallExecutionStep {
    let step = InstallExecutionStep {
        name,
        stage: InstallProgressStage::Installing,
        message,
    };
    steps.push(step.clone());
    step
}

fn verification_result_step(
    name: InstallerDependencyName,
    install_message: String,
    status: InstallerDependencyStatus,
) -> InstallExecutionStep {
    match status.state {
        InstallerDependencyState::Installed => {
            let verification_detail = status
                .path
                .as_deref()
                .map(|path| format!(" Verified on PATH at {path}."))
                .unwrap_or_else(|| " Verified on PATH.".to_string());

            InstallExecutionStep {
                name,
                stage: InstallProgressStage::Completed,
                message: format!("{install_message}{verification_detail}"),
            }
        }
        InstallerDependencyState::Manual => InstallExecutionStep {
            name,
            stage: InstallProgressStage::Manual,
            message: status.message.unwrap_or_else(|| {
                format!(
                    "{name:?} install command finished, but manual setup is still required."
                )
            }),
        },
        _ => InstallExecutionStep {
            name,
            stage: InstallProgressStage::Failed,
            message: status.message.unwrap_or_else(|| {
                format!(
                    "{name:?} install command exited successfully, but the dependency is still unavailable on PATH."
                )
            }),
        },
    }
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

        let installing = InstallExecutionStep {
            name: dependency,
            stage: InstallProgressStage::Installing,
            message: installing_message(dependency),
        };
        let _ = app.emit("installer-progress", &installing);
        steps.push(installing);

        let outcome = if dependency == InstallerDependencyName::Node {
            let capability = detect_node_install_capability();
            let result = install_node_runtime(&capability, |message| {
                let substep = record_installing_substep(&mut steps, dependency, message);
                let _ = app.emit("installer-progress", &substep);
            })
            .await;

            result
        } else {
            install_dependency(dependency, platform).await
        };
        let finished = match outcome {
            Ok(message) => {
                let verifying = InstallExecutionStep {
                    name: dependency,
                    stage: InstallProgressStage::Verifying,
                    message: verifying_message(dependency),
                };
                let _ = app.emit("installer-progress", &verifying);
                steps.push(verifying);

                let status = if dependency == InstallerDependencyName::Node {
                    let capability = detect_node_install_capability();
                    prepend_node_verification_paths_to_process_path(&capability);
                    let node_status =
                        super::detect::detect_dependency_status(InstallerDependencyName::Node);
                    let npm_status =
                        super::detect::detect_dependency_status(InstallerDependencyName::Npm);
                    evaluate_node_runtime_verification(
                        node_status.version.clone(),
                        npm_status.version.clone(),
                    )
                } else {
                    super::detect::detect_installer_environment()
                        .dependencies
                        .into_iter()
                        .find(|status| status.name == dependency)
                        .unwrap_or_else(|| super::detect::detect_dependency_status(dependency))
                };
                verification_result_step(dependency, message, status)
            }
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
        status_with_support(name, kind, state, true)
    }

    fn status_with_support(
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
    fn selected_install_plan_keeps_healthy_node_runtime_for_requested_codex() {
        let plan = build_selected_install_plan(
            &[InstallerDependencyName::Codex],
            &[
                status(
                    InstallerDependencyName::Node,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Installed,
                ),
                status(
                    InstallerDependencyName::Npm,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Installed,
                ),
                status(
                    InstallerDependencyName::Codex,
                    InstallerDependencyKind::Tool,
                    InstallerDependencyState::Missing,
                ),
            ],
        );

        assert_eq!(plan, vec![InstallerDependencyName::Codex]);
    }

    #[test]
    fn install_plan_skips_npm_backed_tools_when_node_runtime_is_broken() {
        let plan = build_install_plan(&[
            status_with_support(
                InstallerDependencyName::Node,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Broken,
                false,
            ),
            status(
                InstallerDependencyName::Npm,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Installed,
            ),
            status(
                InstallerDependencyName::Pnpm,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Missing,
            ),
            status(
                InstallerDependencyName::Codex,
                InstallerDependencyKind::Tool,
                InstallerDependencyState::Missing,
            ),
            status(
                InstallerDependencyName::Gemini,
                InstallerDependencyKind::Tool,
                InstallerDependencyState::Missing,
            ),
        ]);

        assert!(plan.is_empty());
    }

    #[test]
    fn selected_install_plan_skips_npm_backed_tools_when_node_runtime_is_broken() {
        let plan = build_selected_install_plan(
            &[InstallerDependencyName::Codex],
            &[
                status_with_support(
                    InstallerDependencyName::Node,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Broken,
                    false,
                ),
                status(
                    InstallerDependencyName::Npm,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Installed,
                ),
                status(
                    InstallerDependencyName::Codex,
                    InstallerDependencyKind::Tool,
                    InstallerDependencyState::Missing,
                ),
            ],
        );

        assert!(plan.is_empty());
    }

    #[test]
    fn selected_install_plan_skips_npm_backed_tools_when_node_runtime_is_manual_only() {
        let plan = build_selected_install_plan(
            &[InstallerDependencyName::Codex],
            &[
                status_with_support(
                    InstallerDependencyName::Node,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Manual,
                    false,
                ),
                status_with_support(
                    InstallerDependencyName::Npm,
                    InstallerDependencyKind::Core,
                    InstallerDependencyState::Manual,
                    false,
                ),
                status(
                    InstallerDependencyName::Codex,
                    InstallerDependencyKind::Tool,
                    InstallerDependencyState::Missing,
                ),
            ],
        );

        assert!(plan.is_empty());
    }

    #[test]
    fn install_plan_skips_npm_backed_tools_when_npm_is_unavailable() {
        let plan = build_install_plan(&[
            status_with_support(
                InstallerDependencyName::Npm,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Manual,
                false,
            ),
            status(
                InstallerDependencyName::Pnpm,
                InstallerDependencyKind::Core,
                InstallerDependencyState::Missing,
            ),
            status(
                InstallerDependencyName::Gemini,
                InstallerDependencyKind::Tool,
                InstallerDependencyState::Missing,
            ),
        ]);

        assert!(plan.is_empty());
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
    fn linux_manual_commands_include_node_first_for_npm_backed_tools() {
        let commands = get_manual_install_commands("linux");
        let codex = commands
            .iter()
            .find(|item| item.name == InstallerDependencyName::Codex)
            .expect("codex manual command group");
        let gemini = commands
            .iter()
            .find(|item| item.name == InstallerDependencyName::Gemini)
            .expect("gemini manual command group");
        let pnpm = commands
            .iter()
            .find(|item| item.name == InstallerDependencyName::Pnpm)
            .expect("pnpm manual command group");

        assert!(codex
            .commands
            .first()
            .is_some_and(|command| command.contains("Node.js")));
        assert!(gemini
            .commands
            .first()
            .is_some_and(|command| command.contains("Node.js")));
        assert!(pnpm
            .commands
            .first()
            .is_some_and(|command| command.contains("Node.js")));
    }

    #[test]
    fn windows_node_manual_commands_explain_admin_retry_when_not_elevated() {
        let environment = super::super::types::InstallerEnvironment {
            platform: "windows".to_string(),
            auto_install_supported: false,
            dependencies: vec![InstallerDependencyStatus {
                name: InstallerDependencyName::Node,
                kind: InstallerDependencyKind::Core,
                state: InstallerDependencyState::Manual,
                version: None,
                path: None,
                message: Some(
                    "Node.js auto-install on Windows requires winget and DPCC-SWITCH to be reopened as administrator."
                        .to_string(),
                ),
                auto_install_supported: false,
            }],
            last_checked_at: "2026-04-19T00:00:00Z".to_string(),
            ready_count: 0,
            total_count: 1,
        };

        let commands = super::get_manual_install_commands_for_environment(&environment);
        let node = commands
            .iter()
            .find(|item| item.name == InstallerDependencyName::Node)
            .expect("node manual command group");

        assert!(node
            .commands
            .iter()
            .any(|command| command.contains("Run as administrator")));
        assert!(node
            .commands
            .iter()
            .any(|command| command.contains("https://nodejs.org/en/download")));
    }

    #[test]
    fn macos_node_manual_commands_include_tuna_bootstrap_when_brew_is_missing() {
        let environment = super::super::types::InstallerEnvironment {
            platform: "macos".to_string(),
            auto_install_supported: true,
            dependencies: vec![InstallerDependencyStatus {
                name: InstallerDependencyName::Node,
                kind: InstallerDependencyKind::Core,
                state: InstallerDependencyState::Missing,
                version: None,
                path: None,
                message: Some(
                    "Homebrew is missing. DPCC-SWITCH will install Homebrew from the domestic mirror before installing Node.js."
                        .to_string(),
                ),
                auto_install_supported: true,
            }],
            last_checked_at: "2026-04-19T00:00:00Z".to_string(),
            ready_count: 0,
            total_count: 1,
        };

        let commands = super::get_manual_install_commands_for_environment(&environment);
        let node = commands
            .iter()
            .find(|item| item.name == InstallerDependencyName::Node)
            .expect("node manual command group");

        assert!(node
            .commands
            .iter()
            .any(|command| command.contains("mirrors.tuna.tsinghua.edu.cn")));
        assert!(node
            .commands
            .iter()
            .any(|command| command.contains("brew install node")));
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

    #[test]
    fn verification_result_step_marks_completed_only_after_re_detection() {
        let step = super::verification_result_step(
            InstallerDependencyName::Codex,
            "Installed Codex.".to_string(),
            InstallerDependencyStatus {
                name: InstallerDependencyName::Codex,
                kind: InstallerDependencyKind::Tool,
                state: InstallerDependencyState::Installed,
                version: Some("0.42.0".to_string()),
                path: Some("/usr/local/bin/codex".to_string()),
                message: None,
                auto_install_supported: true,
            },
        );

        assert_eq!(step.stage, super::InstallProgressStage::Completed);
        assert!(step.message.contains("Verified on PATH"));
    }

    #[test]
    fn record_installing_substep_appends_step_immediately() {
        let mut steps = Vec::new();

        let step = super::record_installing_substep(
            &mut steps,
            InstallerDependencyName::Node,
            "Checking Homebrew availability...".to_string(),
        );

        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0], step);
        assert_eq!(steps[0].stage, super::InstallProgressStage::Installing);
    }

    #[test]
    fn verification_result_step_fails_when_binary_is_still_missing() {
        let step = super::verification_result_step(
            InstallerDependencyName::Claude,
            "Installed Claude Code.".to_string(),
            InstallerDependencyStatus {
                name: InstallerDependencyName::Claude,
                kind: InstallerDependencyKind::Tool,
                state: InstallerDependencyState::Missing,
                version: None,
                path: None,
                message: Some("claude was not found on PATH.".to_string()),
                auto_install_supported: true,
            },
        );

        assert_eq!(step.stage, super::InstallProgressStage::Failed);
        assert_eq!(step.message, "claude was not found on PATH.");
    }

    #[test]
    fn verification_result_step_surfaces_broken_runtime_message() {
        let step = super::verification_result_step(
            InstallerDependencyName::Node,
            "Installed Node.js with Homebrew.".to_string(),
            InstallerDependencyStatus {
                name: InstallerDependencyName::Node,
                kind: InstallerDependencyKind::Core,
                state: InstallerDependencyState::Broken,
                version: Some("v22.18.0".to_string()),
                path: None,
                message: Some(
                    "Node.js is available on PATH, but npm is missing. Reinstall Node.js to repair npm."
                        .to_string(),
                ),
                auto_install_supported: true,
            },
        );

        assert_eq!(step.stage, super::InstallProgressStage::Failed);
        assert_eq!(
            step.message,
            "Node.js is available on PATH, but npm is missing. Reinstall Node.js to repair npm."
        );
    }
}
