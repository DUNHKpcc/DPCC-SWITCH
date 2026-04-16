# Installer Center Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a native installer center under `Settings -> About` that detects and installs local CLI dependencies for `claude`, `codex`, `gemini`, and `opencode`, with automatic install support on Windows/macOS where supported and manual guidance on Linux.

**Architecture:** Implement a dedicated installer subsystem in `src-tauri` with its own commands, detection logic, install plan logic, and progress events. On the frontend, add a separate installer dialog and supporting components that are launched from the About page and styled using existing `cc-switch` UI primitives.

**Tech Stack:** React 18, TypeScript, Vitest, Tauri 2, Rust, serde, tokio, existing Radix UI wrappers

---

### Task 1: Add Installer Backend Types And Detection

**Files:**
- Create: `src-tauri/src/services/installer/mod.rs`
- Create: `src-tauri/src/services/installer/types.rs`
- Create: `src-tauri/src/services/installer/detect.rs`
- Modify: `src-tauri/src/services/mod.rs`
- Test: `src-tauri/src/services/installer/detect.rs`

- [ ] **Step 1: Write the failing Rust detection tests**

```rust
#[cfg(test)]
mod tests {
    use super::{
        detect_dependency_from_output, InstallerDependencyKind, InstallerDependencyName,
        InstallerDependencyState,
    };

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
}
```

- [ ] **Step 2: Run the detection tests to verify they fail**

Run: `cd src-tauri && cargo test detects_missing_binary_as_missing -- --exact`
Expected: FAIL with unresolved imports or missing `detect_dependency_from_output`

- [ ] **Step 3: Write the installer types and detection implementation**

```rust
// src-tauri/src/services/installer/types.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallerDependencyKind {
    Core,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallerDependencyName {
    Node,
    Npm,
    Git,
    Claude,
    Codex,
    Gemini,
    Opencode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallerDependencyState {
    Installed,
    Missing,
    Outdated,
    Broken,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallerDependencyStatus {
    pub name: InstallerDependencyName,
    pub kind: InstallerDependencyKind,
    pub state: InstallerDependencyState,
    pub version: Option<String>,
    pub path: Option<String>,
    pub message: Option<String>,
    pub auto_install_supported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallerEnvironment {
    pub platform: String,
    pub auto_install_supported: bool,
    pub dependencies: Vec<InstallerDependencyStatus>,
    pub last_checked_at: String,
    pub ready_count: usize,
    pub total_count: usize,
}
```

```rust
// src-tauri/src/services/installer/detect.rs
use std::process::Command;

use chrono::Utc;

use super::types::{
    InstallerDependencyKind, InstallerDependencyName, InstallerDependencyState,
    InstallerDependencyStatus, InstallerEnvironment,
};

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

pub fn detect_installer_environment() -> InstallerEnvironment {
    let platform = std::env::consts::OS.to_string();
    let dependencies = vec![
        detect_binary("node", InstallerDependencyName::Node, InstallerDependencyKind::Core),
        detect_binary("npm", InstallerDependencyName::Npm, InstallerDependencyKind::Core),
        detect_binary("git", InstallerDependencyName::Git, InstallerDependencyKind::Core),
        detect_binary("claude", InstallerDependencyName::Claude, InstallerDependencyKind::Tool),
        detect_binary("codex", InstallerDependencyName::Codex, InstallerDependencyKind::Tool),
        detect_binary("gemini", InstallerDependencyName::Gemini, InstallerDependencyKind::Tool),
        detect_binary("opencode", InstallerDependencyName::Opencode, InstallerDependencyKind::Tool),
    ];

    let ready_count = dependencies
        .iter()
        .filter(|item| item.state == InstallerDependencyState::Installed)
        .count();

    InstallerEnvironment {
        platform: platform.clone(),
        auto_install_supported: platform == "windows" || platform == "macos" || platform == "darwin",
        total_count: dependencies.len(),
        ready_count,
        dependencies,
        last_checked_at: Utc::now().to_rfc3339(),
    }
}

fn detect_binary(
    binary: &str,
    name: InstallerDependencyName,
    kind: InstallerDependencyKind,
) -> InstallerDependencyStatus {
    let auto_install_supported = match (std::env::consts::OS, binary) {
        ("linux", _) => false,
        ("macos", "git") => false,
        ("darwin", "git") => false,
        _ => true,
    };

    let version_output = Command::new(binary).arg("--version").output();
    let path_output = if cfg!(target_os = "windows") {
        Command::new("where").arg(binary).output()
    } else {
        Command::new("which").arg(binary).output()
    };

    match (version_output, path_output) {
        (Ok(version), Ok(path)) if version.status.success() && path.status.success() => {
            let version_text = String::from_utf8_lossy(&version.stdout).trim().to_string();
            let path_text = String::from_utf8_lossy(&path.stdout)
                .lines()
                .next()
                .unwrap_or_default()
                .trim()
                .to_string();

            detect_dependency_from_output(
                name,
                kind,
                Some(version_text),
                Some(path_text),
                None,
                auto_install_supported,
            )
        }
        _ => detect_dependency_from_output(
            name,
            kind,
            None,
            None,
            Some(format!("{binary} was not found on PATH.")),
            auto_install_supported,
        ),
    }
}
```

```rust
// src-tauri/src/services/installer/mod.rs
pub mod detect;
pub mod types;

pub use detect::detect_installer_environment;
pub use types::{
    InstallerDependencyKind, InstallerDependencyName, InstallerDependencyState,
    InstallerDependencyStatus, InstallerEnvironment,
};
```

```rust
// src-tauri/src/services/mod.rs
pub mod installer;
```

- [ ] **Step 4: Run the detection tests to verify they pass**

Run: `cd src-tauri && cargo test detects_missing_binary_as_missing -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/mod.rs \
  src-tauri/src/services/installer/mod.rs \
  src-tauri/src/services/installer/types.rs \
  src-tauri/src/services/installer/detect.rs
git commit -m "feat: add installer environment detection"
```

### Task 2: Add Install Planning And Manual Commands

**Files:**
- Create: `src-tauri/src/services/installer/install.rs`
- Modify: `src-tauri/src/services/installer/mod.rs`
- Test: `src-tauri/src/services/installer/install.rs`

- [ ] **Step 1: Write the failing install planning tests**

```rust
#[cfg(test)]
mod tests {
    use super::{build_install_plan, get_manual_install_commands, ManualInstallCommandGroup};
    use crate::services::installer::{
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
            status(InstallerDependencyName::Node, InstallerDependencyKind::Core, InstallerDependencyState::Missing),
            status(InstallerDependencyName::Codex, InstallerDependencyKind::Tool, InstallerDependencyState::Missing),
        ]);

        assert_eq!(
            plan,
            vec![InstallerDependencyName::Node, InstallerDependencyName::Codex]
        );
    }

    #[test]
    fn install_plan_treats_missing_npm_as_node_install() {
        let plan = build_install_plan(&[
            status(InstallerDependencyName::Npm, InstallerDependencyKind::Core, InstallerDependencyState::Missing),
            status(InstallerDependencyName::Gemini, InstallerDependencyKind::Tool, InstallerDependencyState::Missing),
        ]);

        assert_eq!(
            plan,
            vec![InstallerDependencyName::Node, InstallerDependencyName::Gemini]
        );
    }

    #[test]
    fn linux_manual_commands_include_all_tools() {
        let commands = get_manual_install_commands("linux");

        assert!(commands.iter().any(|item| item.name == InstallerDependencyName::Claude));
        assert!(commands.iter().any(|item| item.name == InstallerDependencyName::Codex));
        assert!(commands.iter().any(|item| item.name == InstallerDependencyName::Gemini));
        assert!(commands.iter().any(|item| item.name == InstallerDependencyName::Opencode));
    }
}
```

- [ ] **Step 2: Run the install planning tests to verify they fail**

Run: `cd src-tauri && cargo test install_plan_puts_node_before_tooling -- --exact`
Expected: FAIL with missing `build_install_plan` and `get_manual_install_commands`

- [ ] **Step 3: Implement the install planner and manual command groups**

```rust
// src-tauri/src/services/installer/install.rs
use serde::{Deserialize, Serialize};

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
```

```rust
// src-tauri/src/services/installer/mod.rs
pub mod install;

pub use install::{
    build_install_plan, get_manual_install_commands, install_missing_dependencies,
    InstallExecutionStep, InstallProgressStage, InstallerRunResult,
    ManualInstallCommandGroup,
};
```

- [ ] **Step 4: Run the install planning tests to verify they pass**

Run: `cd src-tauri && cargo test install_plan_puts_node_before_tooling -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/services/installer/mod.rs \
  src-tauri/src/services/installer/install.rs
git commit -m "feat: add installer plan and manual commands"
```

### Task 3: Add Tauri Installer Commands And Progress Events

**Files:**
- Create: `src-tauri/src/commands/installer.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/services/installer/install.rs`
- Test: `src-tauri/src/services/installer/install.rs`

- [ ] **Step 1: Write the failing backend progress tests**

```rust
#[cfg(test)]
mod progress_tests {
    use super::{normalize_install_result, InstallExecutionStep, InstallProgressStage};
    use crate::services::installer::InstallerDependencyName;

    #[test]
    fn normalize_install_result_collects_completed_and_failed_dependencies() {
        let result = normalize_install_result(vec![
            InstallExecutionStep {
                name: InstallerDependencyName::Node,
                stage: InstallProgressStage::Completed,
                message: "Installed node".to_string(),
            },
            InstallExecutionStep {
                name: InstallerDependencyName::Codex,
                stage: InstallProgressStage::Failed,
                message: "codex install failed".to_string(),
            },
        ]);

        assert_eq!(result.completed_dependencies, vec![InstallerDependencyName::Node]);
        assert_eq!(result.failed_dependencies, vec![InstallerDependencyName::Codex]);
    }
}
```

- [ ] **Step 2: Run the backend progress tests to verify they fail**

Run: `cd src-tauri && cargo test normalize_install_result_collects_completed_and_failed_dependencies -- --exact`
Expected: FAIL with missing `normalize_install_result` or install result types

- [ ] **Step 3: Implement install execution result types and Tauri commands**

```rust
// src-tauri/src/services/installer/install.rs
use tauri::{AppHandle, Emitter};

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

pub fn normalize_install_result(steps: Vec<InstallExecutionStep>) -> InstallerRunResult {
    let completed_dependencies = steps
        .iter()
        .filter(|step| step.stage == InstallProgressStage::Completed)
        .map(|step| step.name)
        .collect::<Vec<_>>();
    let failed_dependencies = steps
        .iter()
        .filter(|step| step.stage == InstallProgressStage::Failed)
        .map(|step| step.name)
        .collect::<Vec<_>>();
    let manual_dependencies = steps
        .iter()
        .filter(|step| step.stage == InstallProgressStage::Manual)
        .map(|step| step.name)
        .collect::<Vec<_>>();

    InstallerRunResult {
        steps,
        completed_dependencies,
        failed_dependencies,
        manual_dependencies,
        status_message: "Installer run completed.".to_string(),
    }
}

async fn run_install_command(command: &str, args: &[&str]) -> Result<(), String> {
    let status = tokio::process::Command::new(command)
        .args(args)
        .status()
        .await
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
            "windows" => Err("Node MSI flow must be implemented with a downloaded installer package.".to_string()),
            "macos" | "darwin" => Err("Node PKG flow must be implemented with a downloaded installer package.".to_string()),
            _ => Err("Node auto-install is not supported on this platform.".to_string()),
        },
        InstallerDependencyName::Git => match platform {
            "windows" => run_install_command("winget", &["install", "--id", "Git.Git", "-e", "--source", "winget"])
                .await
                .map(|_| "Installed Git with winget.".to_string()),
            "macos" | "darwin" => Err("Git requires manual install on macOS.".to_string()),
            _ => Err("Git auto-install is not supported on this platform.".to_string()),
        },
        InstallerDependencyName::Claude => {
            if platform == "windows" {
                run_install_command(
                    "powershell",
                    &["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", "irm https://claude.ai/install.ps1 | iex"],
                )
                .await
                .map(|_| "Installed Claude Code.".to_string())
            } else {
                run_install_command("sh", &["-lc", "curl -fsSL https://claude.ai/install.sh | bash"])
                    .await
                    .map(|_| "Installed Claude Code.".to_string())
            }
        }
        InstallerDependencyName::Codex => run_install_command("npm", &["i", "-g", "@openai/codex@latest"])
            .await
            .map(|_| "Installed Codex.".to_string()),
        InstallerDependencyName::Gemini => run_install_command("npm", &["i", "-g", "@google/gemini-cli@latest"])
            .await
            .map(|_| "Installed Gemini CLI.".to_string()),
        InstallerDependencyName::Opencode => {
            if platform == "windows" {
                Err("OpenCode auto-install is not supported on Windows in v1.".to_string())
            } else {
                run_install_command("sh", &["-lc", "curl -fsSL https://opencode.ai/install | bash"])
                    .await
                    .map(|_| "Installed OpenCode.".to_string())
            }
        }
        InstallerDependencyName::Npm => Ok("npm is satisfied by the Node.js installation.".to_string()),
    }
}

pub async fn install_missing_dependencies(app: &AppHandle) -> Result<InstallerRunResult, String> {
    let environment = super::detect::detect_installer_environment();
    let plan = build_install_plan(&environment.dependencies);
    let mut steps = Vec::new();
    let platform = std::env::consts::OS;

    for dependency in plan {
        let queued = InstallExecutionStep {
            name: dependency,
            stage: InstallProgressStage::Queued,
            message: format!("Preparing {:?} installation...", dependency),
        };
        let _ = app.emit("installer-progress", &queued);
        steps.push(queued);

        let outcome = install_dependency(dependency, platform).await;
        let finished = InstallExecutionStep {
            name: dependency,
            stage: match &outcome {
                Ok(_) => InstallProgressStage::Completed,
                Err(error) if error.contains("manual") || error.contains("not supported") => {
                    InstallProgressStage::Manual
                }
                Err(_) => InstallProgressStage::Failed,
            },
            message: outcome.unwrap_or_else(|error| error),
        };
        let _ = app.emit("installer-progress", &finished);
        steps.push(finished);
    }

    Ok(normalize_install_result(steps))
}
```

```rust
// src-tauri/src/commands/installer.rs
#![allow(non_snake_case)]

use tauri::AppHandle;

#[tauri::command]
pub async fn detect_installer_environment(
) -> Result<crate::services::installer::InstallerEnvironment, String> {
    Ok(crate::services::installer::detect_installer_environment())
}

#[tauri::command]
pub async fn install_missing_dependencies(
    app: AppHandle,
) -> Result<crate::services::installer::InstallerRunResult, String> {
    crate::services::installer::install_missing_dependencies(&app).await
}

#[tauri::command]
pub async fn get_manual_install_commands(
) -> Result<Vec<crate::services::installer::ManualInstallCommandGroup>, String> {
    Ok(crate::services::installer::get_manual_install_commands(
        std::env::consts::OS,
    ))
}
```

```rust
// src-tauri/src/commands/mod.rs
mod installer;
pub use installer::*;
```

```rust
// src-tauri/src/lib.rs (inside generate_handler!)
commands::detect_installer_environment,
commands::install_missing_dependencies,
commands::get_manual_install_commands,
```

- [ ] **Step 4: Run the backend progress tests to verify they pass**

Run: `cd src-tauri && cargo test normalize_install_result_collects_completed_and_failed_dependencies -- --exact`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/mod.rs \
  src-tauri/src/commands/installer.rs \
  src-tauri/src/lib.rs \
  src-tauri/src/services/installer/install.rs
git commit -m "feat: add installer tauri commands"
```

### Task 4: Add Frontend Installer Types, API, And Dialog UI

**Files:**
- Create: `src/types/installer.ts`
- Create: `src/lib/api/installer.ts`
- Create: `src/components/settings/InstallerDependencyCard.tsx`
- Create: `src/components/settings/InstallerActions.tsx`
- Create: `src/components/settings/InstallerProgressPanel.tsx`
- Create: `src/components/settings/InstallerCenterDialog.tsx`
- Test: `src/components/settings/InstallerCenterDialog.test.tsx`

- [ ] **Step 1: Write the failing frontend dialog tests**

```tsx
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { InstallerCenterDialog } from "@/components/settings/InstallerCenterDialog";

vi.mock("@/lib/api/installer", () => ({
  installerApi: {
    detectEnvironment: vi.fn().mockResolvedValue({
      platform: "linux",
      autoInstallSupported: false,
      dependencies: [
        {
          name: "node",
          kind: "core",
          state: "missing",
          version: null,
          path: null,
          message: "node was not found on PATH.",
          autoInstallSupported: false,
        },
      ],
      lastCheckedAt: "2026-04-16T00:00:00Z",
      readyCount: 0,
      totalCount: 1,
    }),
    installMissing: vi.fn(),
    getManualCommands: vi.fn().mockResolvedValue([
      { name: "node", title: "Node.js", commands: ["Install Node.js with your package manager or nvm."] },
    ]),
    subscribeProgress: vi.fn(() => () => {}),
  },
}));

test("loads environment status when opened", async () => {
  render(<InstallerCenterDialog open onOpenChange={() => {}} />);

  await waitFor(() => {
    expect(screen.getByText("node")).toBeInTheDocument();
  });
});

test("shows manual commands on linux", async () => {
  const user = userEvent.setup();
  render(<InstallerCenterDialog open onOpenChange={() => {}} />);

  await user.click(await screen.findByRole("button", { name: /manual/i }));

  expect(await screen.findByText(/package manager or nvm/i)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the dialog tests to verify they fail**

Run: `pnpm test:unit src/components/settings/InstallerCenterDialog.test.tsx`
Expected: FAIL with missing component or missing `installerApi`

- [ ] **Step 3: Implement the frontend installer models, API, and dialog**

```ts
// src/types/installer.ts
export type InstallerDependencyKind = "core" | "tool";
export type InstallerDependencyState =
  | "installed"
  | "missing"
  | "outdated"
  | "broken"
  | "manual";
export type InstallProgressStage =
  | "queued"
  | "downloading"
  | "installing"
  | "verifying"
  | "completed"
  | "failed"
  | "manual";

export interface InstallerDependencyStatus {
  name: string;
  kind: InstallerDependencyKind;
  state: InstallerDependencyState;
  version: string | null;
  path: string | null;
  message: string | null;
  autoInstallSupported: boolean;
}

export interface InstallerEnvironment {
  platform: string;
  autoInstallSupported: boolean;
  dependencies: InstallerDependencyStatus[];
  lastCheckedAt: string;
  readyCount: number;
  totalCount: number;
}

export interface ManualInstallCommandGroup {
  name: string;
  title: string;
  commands: string[];
}

export interface InstallExecutionStep {
  name: string;
  stage: InstallProgressStage;
  message: string;
}

export interface InstallerRunResult {
  steps: InstallExecutionStep[];
  completedDependencies: string[];
  failedDependencies: string[];
  manualDependencies: string[];
  statusMessage: string;
}
```

```ts
// src/lib/api/installer.ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  InstallerEnvironment,
  InstallerRunResult,
  ManualInstallCommandGroup,
  InstallExecutionStep,
} from "@/types/installer";

export const installerApi = {
  detectEnvironment(): Promise<InstallerEnvironment> {
    return invoke("detect_installer_environment");
  },
  installMissing(): Promise<InstallerRunResult> {
    return invoke("install_missing_dependencies");
  },
  getManualCommands(): Promise<ManualInstallCommandGroup[]> {
    return invoke("get_manual_install_commands");
  },
  async subscribeProgress(
    handler: (event: InstallExecutionStep) => void,
  ): Promise<UnlistenFn> {
    return listen<InstallExecutionStep>("installer-progress", ({ payload }) => {
      handler(payload);
    });
  },
};
```

```tsx
// src/components/settings/InstallerCenterDialog.tsx
import { useEffect, useMemo, useState } from "react";
import { Loader2, RefreshCw, Wrench, TerminalSquare } from "lucide-react";

import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { installerApi } from "@/lib/api/installer";
import type {
  InstallerEnvironment,
  InstallExecutionStep,
  ManualInstallCommandGroup,
} from "@/types/installer";
import { InstallerDependencyCard } from "./InstallerDependencyCard";
import { InstallerActions } from "./InstallerActions";
import { InstallerProgressPanel } from "./InstallerProgressPanel";

interface InstallerCenterDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function InstallerCenterDialog({
  open,
  onOpenChange,
}: InstallerCenterDialogProps) {
  const [environment, setEnvironment] = useState<InstallerEnvironment | null>(null);
  const [manualCommands, setManualCommands] = useState<ManualInstallCommandGroup[]>([]);
  const [progress, setProgress] = useState<InstallExecutionStep[]>([]);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState(false);

  useEffect(() => {
    if (!open) return;
    let active = true;

    void (async () => {
      setLoading(true);
      try {
        const [env, commands, unlisten] = await Promise.all([
          installerApi.detectEnvironment(),
          installerApi.getManualCommands(),
          installerApi.subscribeProgress((event) => {
            setProgress((current) => [...current, event].slice(-20));
          }),
        ]);

        if (!active) {
          unlisten();
          return;
        }

        setEnvironment(env);
        setManualCommands(commands);
      } finally {
        if (active) setLoading(false);
      }
    })();

    return () => {
      active = false;
    };
  }, [open]);

  const grouped = useMemo(() => {
    const dependencies = environment?.dependencies ?? [];
    return {
      core: dependencies.filter((item) => item.kind === "core"),
      tool: dependencies.filter((item) => item.kind === "tool"),
    };
  }, [environment]);

  async function refreshEnvironment() {
    setLoading(true);
    try {
      setEnvironment(await installerApi.detectEnvironment());
    } finally {
      setLoading(false);
    }
  }

  async function installMissing() {
    setInstalling(true);
    setProgress([]);
    try {
      await installerApi.installMissing();
      await refreshEnvironment();
    } finally {
      setInstalling(false);
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-5xl p-0">
        <DialogHeader>
          <DialogTitle>Environment Check & Install</DialogTitle>
        </DialogHeader>
        <div className="grid gap-6 p-6">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Environment Summary</CardTitle>
              <CardDescription>
                {environment
                  ? `${environment.readyCount}/${environment.totalCount} dependencies ready`
                  : "Loading installer environment..."}
              </CardDescription>
            </CardHeader>
            <CardContent className="flex items-center gap-3 text-sm text-muted-foreground">
              {loading ? <Loader2 className="h-4 w-4 animate-spin" /> : <TerminalSquare className="h-4 w-4" />}
              <span>{environment?.platform ?? "unknown"}</span>
            </CardContent>
          </Card>

          <InstallerActions
            loading={loading}
            installing={installing}
            autoInstallSupported={environment?.autoInstallSupported ?? false}
            onRefresh={refreshEnvironment}
            onInstall={installMissing}
            manualCommands={manualCommands}
          />

          <section className="grid gap-4 lg:grid-cols-2">
            <div className="space-y-3">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Wrench className="h-4 w-4" />
                Core Dependencies
              </div>
              {grouped.core.map((dependency) => (
                <InstallerDependencyCard key={dependency.name} dependency={dependency} />
              ))}
            </div>

            <div className="space-y-3">
              <div className="flex items-center gap-2 text-sm font-medium">
                <RefreshCw className="h-4 w-4" />
                CLI Tools
              </div>
              {grouped.tool.map((dependency) => (
                <InstallerDependencyCard key={dependency.name} dependency={dependency} />
              ))}
            </div>
          </section>

          <InstallerProgressPanel progress={progress} />
        </div>
      </DialogContent>
    </Dialog>
  );
}
```

- [ ] **Step 4: Run the dialog tests to verify they pass**

Run: `pnpm test:unit src/components/settings/InstallerCenterDialog.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/types/installer.ts \
  src/lib/api/installer.ts \
  src/components/settings/InstallerDependencyCard.tsx \
  src/components/settings/InstallerActions.tsx \
  src/components/settings/InstallerProgressPanel.tsx \
  src/components/settings/InstallerCenterDialog.tsx \
  src/components/settings/InstallerCenterDialog.test.tsx
git commit -m "feat: add installer center dialog"
```

### Task 5: Integrate Installer Center Into About Page And Verify End To End

**Files:**
- Modify: `src/components/settings/AboutSection.tsx`
- Modify: `src/i18n/locales/en.json`
- Modify: `src/i18n/locales/zh.json`
- Modify: `src/i18n/locales/ja.json`
- Test: `src/components/settings/AboutSection.test.tsx`

- [ ] **Step 1: Write the failing About integration test**

```tsx
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AboutSection } from "@/components/settings/AboutSection";

test("opens installer center from about section", async () => {
  const user = userEvent.setup();
  render(<AboutSection isPortable={false} />);

  await user.click(screen.getByRole("button", { name: /environment check/i }));

  expect(await screen.findByText(/environment summary/i)).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the About integration test to verify it fails**

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx`
Expected: FAIL because the installer entry button or dialog is missing

- [ ] **Step 3: Integrate the installer entry into AboutSection and add i18n labels**

```tsx
// src/components/settings/AboutSection.tsx
import { useState } from "react";
import { Shield, RefreshCw, Download, Wrench } from "lucide-react";

import { InstallerCenterDialog } from "@/components/settings/InstallerCenterDialog";

// inside AboutSection component
const [installerOpen, setInstallerOpen] = useState(false);

// replace the old static one-click install block with:
<motion.div
  initial={{ opacity: 0, y: 10 }}
  animate={{ opacity: 1, y: 0 }}
  transition={{ duration: 0.3, delay: 0.3 }}
  className="space-y-3"
>
  <h3 className="text-sm font-medium px-1">
    {t("settings.installerCenter")}
  </h3>
  <div className="rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-4 space-y-3 shadow-sm">
    <div className="flex items-start justify-between gap-3">
      <div className="space-y-1">
        <div className="flex items-center gap-2 text-sm font-medium">
          <Shield className="h-4 w-4 text-primary" />
          {t("settings.installerCenter")}
        </div>
        <p className="text-xs text-muted-foreground">
          {t("settings.installerCenterHint")}
        </p>
      </div>
      <Button
        size="sm"
        className="h-8 gap-1.5 text-xs"
        onClick={() => setInstallerOpen(true)}
      >
        <Wrench className="h-3.5 w-3.5" />
        {t("settings.openInstallerCenter")}
      </Button>
    </div>
  </div>
  <InstallerCenterDialog
    open={installerOpen}
    onOpenChange={setInstallerOpen}
  />
</motion.div>
```

```json
// src/i18n/locales/en.json
{
  "settings": {
    "installerCenter": "Environment Check & Install",
    "installerCenterHint": "Detect local dependencies and install supported CLI tools from one place.",
    "openInstallerCenter": "Open Installer"
  }
}
```

```json
// src/i18n/locales/zh.json
{
  "settings": {
    "installerCenter": "环境检测与安装",
    "installerCenterHint": "集中检测本机依赖，并安装受支持的 CLI 工具。",
    "openInstallerCenter": "打开安装中心"
  }
}
```

```json
// src/i18n/locales/ja.json
{
  "settings": {
    "installerCenter": "環境チェックとインストール",
    "installerCenterHint": "ローカル依存関係を検出し、対応する CLI ツールをまとめてインストールします。",
    "openInstallerCenter": "インストーラーを開く"
  }
}
```

- [ ] **Step 4: Run end-to-end verification commands**

Run: `pnpm test:unit src/components/settings/InstallerCenterDialog.test.tsx && pnpm test:unit src/components/settings/AboutSection.test.tsx`
Expected: PASS

Run: `cd src-tauri && cargo test`
Expected: PASS

Run: `pnpm typecheck`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/settings/AboutSection.tsx \
  src/i18n/locales/en.json \
  src/i18n/locales/zh.json \
  src/i18n/locales/ja.json \
  src/components/settings/AboutSection.test.tsx
git commit -m "feat: add installer center to about page"
```
