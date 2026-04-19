# Node Auto Install Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add real `Node.js` auto-install support for `Windows` and `macOS` while keeping the existing About-page installer UI and verifying `node` plus `npm` before reporting success.

**Architecture:** Add a focused `node_runtime` installer service that owns `winget` / `brew` / elevation / mirror-bootstrap decisions, then reuse that service from `detect.rs`, `install.rs`, and manual-command generation. Keep the frontend on the current dependency-card and progress-panel contract so the UI stays visually unchanged and only reflects richer backend state and messages.

**Tech Stack:** Rust (`tauri`, `std::process::Command`), TypeScript/React (`vitest`, Testing Library), existing installer service/events

---

## File Structure

- Create: `src-tauri/src/services/installer/node_runtime.rs`
  Purpose: detect `winget` / `brew` / Windows elevation, build mirrored Homebrew bootstrap commands, execute the Node install flow, and verify `node` + `npm`.
- Modify: `src-tauri/src/services/installer/mod.rs`
  Purpose: export the new `node_runtime` module.
- Modify: `src-tauri/src/services/installer/detect.rs`
  Purpose: fold machine capability into `Node` / `npm` dependency statuses and keep Node-dependent tools consistent with that capability.
- Modify: `src-tauri/src/services/installer/install.rs`
  Purpose: route `Node` installs through the new runtime helper, emit richer progress messages, generate environment-aware manual commands, and keep plan filtering honest.
- Modify: `src-tauri/src/commands/installer.rs`
  Purpose: make `get_manual_install_commands` use the detected environment instead of a static platform-only table.
- Modify: `src/components/settings/AboutSection.test.tsx`
  Purpose: lock the About page UI contract for Windows admin prompts, macOS mirror bootstrap messaging, and progress text.
- Modify: `src/components/settings/InstallerCenterDialog.test.tsx`
  Purpose: lock the installer-center contract for manual commands and disabled auto-install affordances when preflight is not executable.

## Task 1: Add Node Runtime Capability Detection

**Files:**
- Create: `src-tauri/src/services/installer/node_runtime.rs`
- Modify: `src-tauri/src/services/installer/mod.rs`
- Modify: `src-tauri/src/services/installer/detect.rs`
- Test: `src-tauri/src/services/installer/node_runtime.rs`
- Test: `src-tauri/src/services/installer/detect.rs`

- [ ] **Step 1: Write the failing capability tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::installer::types::InstallerDependencyState;

    #[test]
    fn windows_requires_winget_and_elevation_for_node_auto_install() {
        let capability = resolve_node_install_capability(
            "windows",
            false,
            false,
            None,
        );

        assert!(!capability.auto_install_supported);
        assert_eq!(capability.fallback_state, InstallerDependencyState::Manual);
        assert_eq!(
            capability.message.as_deref(),
            Some(
                "Node.js auto-install on Windows requires winget and DPCC-SWITCH to be reopened as administrator."
            )
        );
    }

    #[test]
    fn macos_without_brew_stays_auto_installable_through_mirror_bootstrap() {
        let capability = resolve_node_install_capability(
            "macos",
            false,
            false,
            None,
        );

        assert!(capability.auto_install_supported);
        assert_eq!(capability.fallback_state, InstallerDependencyState::Missing);
        assert_eq!(
            capability.message.as_deref(),
            Some(
                "Homebrew is missing. DPCC-SWITCH will install Homebrew from the domestic mirror before installing Node.js."
            )
        );
    }
}
```

Add this detect-side regression in `src-tauri/src/services/installer/detect.rs`:

```rust
#[test]
fn node_and_npm_become_manual_when_windows_preflight_is_not_executable() {
    let capability = crate::services::installer::node_runtime::NodeInstallCapability {
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
            .expect("node runtime status");

        assert_eq!(status.state, InstallerDependencyState::Manual);
        assert!(!status.auto_install_supported);
        assert_eq!(status.message, capability.message);
    }
}
```

- [ ] **Step 2: Run the Rust tests to verify they fail**

Run: `cargo test windows_requires_winget_and_elevation_for_node_auto_install`
Expected: FAIL with `cannot find function 'resolve_node_install_capability' in this scope`

Run: `cargo test node_and_npm_become_manual_when_windows_preflight_is_not_executable`
Expected: FAIL because `finalize_detected_dependencies` does not yet accept a capability argument

- [ ] **Step 3: Implement the capability model and wire it into detect**

Create `src-tauri/src/services/installer/node_runtime.rs` with this core model:

```rust
use crate::services::installer::types::InstallerDependencyState;
use std::process::Command;

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
    brew_binary: Option<&str>,
) -> NodeInstallCapability {
    match platform {
        "windows" if !winget_available || !windows_elevated => NodeInstallCapability {
            auto_install_supported: false,
            fallback_state: InstallerDependencyState::Manual,
            message: Some(
                "Node.js auto-install on Windows requires winget and DPCC-SWITCH to be reopened as administrator."
                    .to_string(),
            ),
            brew_binary: None,
            winget_available,
            windows_elevated,
        },
        "windows" => NodeInstallCapability {
            auto_install_supported: true,
            fallback_state: InstallerDependencyState::Missing,
            message: Some("Node.js will be installed with winget.".to_string()),
            brew_binary: None,
            winget_available,
            windows_elevated,
        },
        "macos" | "darwin" if brew_binary.is_none() => NodeInstallCapability {
            auto_install_supported: true,
            fallback_state: InstallerDependencyState::Missing,
            message: Some(
                "Homebrew is missing. DPCC-SWITCH will install Homebrew from the domestic mirror before installing Node.js."
                    .to_string(),
            ),
            brew_binary: None,
            winget_available: false,
            windows_elevated: false,
        },
        "macos" | "darwin" => NodeInstallCapability {
            auto_install_supported: true,
            fallback_state: InstallerDependencyState::Missing,
            message: Some("Node.js will be installed with Homebrew.".to_string()),
            brew_binary: brew_binary.map(str::to_string),
            winget_available: false,
            windows_elevated: false,
        },
        _ => NodeInstallCapability {
            auto_install_supported: false,
            fallback_state: InstallerDependencyState::Manual,
            message: Some("Node.js auto-install is not supported on this platform.".to_string()),
            brew_binary: None,
            winget_available: false,
            windows_elevated: false,
        },
    }
}
```

Add these detection helpers in the same file:

```rust
pub fn detect_node_install_capability() -> NodeInstallCapability {
    let platform = std::env::consts::OS;
    let winget_available = detect_binary_available("winget");
    let windows_elevated = detect_windows_elevation();
    let brew_binary = detect_homebrew_binary();

    resolve_node_install_capability(
        platform,
        winget_available,
        windows_elevated,
        brew_binary.as_deref(),
    )
}

fn detect_binary_available(binary: &str) -> bool {
    let output = if cfg!(target_os = "windows") {
        Command::new("where").arg(binary).output()
    } else {
        Command::new("which").arg(binary).output()
    };

    output.map(|out| out.status.success()).unwrap_or(false)
}

fn detect_homebrew_binary() -> Option<String> {
    for candidate in ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"] {
        if std::path::Path::new(candidate).exists() {
            return Some(candidate.to_string());
        }
    }

    if detect_binary_available("brew") {
        return Some("brew".to_string());
    }

    None
}

#[cfg(target_os = "windows")]
fn detect_windows_elevation() -> bool {
    Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "[bool](([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))",
        ])
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().eq_ignore_ascii_case("True"))
        .unwrap_or(false)
}

#[cfg(not(target_os = "windows"))]
fn detect_windows_elevation() -> bool {
    false
}
```

Export the module in `src-tauri/src/services/installer/mod.rs`:

```rust
pub mod detect;
pub mod install;
pub mod node_runtime;
pub mod types;
```

Update `src-tauri/src/services/installer/detect.rs` so capability is applied to `Node` and `npm` before dependent-tool normalization:

```rust
use super::node_runtime::NodeInstallCapability;

pub fn finalize_detected_dependencies(
    dependencies: Vec<InstallerDependencyStatus>,
    node_capability: &NodeInstallCapability,
) -> Vec<InstallerDependencyStatus> {
    let mut dependencies: Vec<InstallerDependencyStatus> = dependencies
        .into_iter()
        .map(|mut dependency| {
            if matches!(
                dependency.name,
                InstallerDependencyName::Node | InstallerDependencyName::Npm
            ) && dependency.state != InstallerDependencyState::Installed
            {
                dependency.auto_install_supported = node_capability.auto_install_supported;
                dependency.state = if node_capability.auto_install_supported {
                    InstallerDependencyState::Missing
                } else {
                    node_capability.fallback_state
                };
                dependency.message = node_capability.message.clone();
            }

            dependency
        })
        .collect();

    let npm_installed = dependencies.iter().any(|dependency| {
        dependency.name == InstallerDependencyName::Npm
            && dependency.state == InstallerDependencyState::Installed
    });

    for dependency in &mut dependencies {
        if matches!(
            dependency.name,
            InstallerDependencyName::Pnpm
                | InstallerDependencyName::Codex
                | InstallerDependencyName::Gemini
        ) && dependency.state != InstallerDependencyState::Installed
            && !npm_installed
        {
            dependency.state = InstallerDependencyState::Manual;
            dependency.auto_install_supported = false;
            dependency.message = Some(
                "npm is not available on PATH. Install Node.js first, then retry this tool."
                    .to_string(),
            );
        }
    }

    dependencies
}
```

Call the new helper from `detect_installer_environment()`:

```rust
let node_capability = super::node_runtime::detect_node_install_capability();
let dependencies = finalize_detected_dependencies(
    vec![
        detect_dependency_status(InstallerDependencyName::Node),
        detect_dependency_status(InstallerDependencyName::Npm),
        detect_dependency_status(InstallerDependencyName::Pnpm),
        detect_dependency_status(InstallerDependencyName::Git),
        detect_dependency_status(InstallerDependencyName::Claude),
        detect_dependency_status(InstallerDependencyName::Codex),
        detect_dependency_status(InstallerDependencyName::Gemini),
        detect_dependency_status(InstallerDependencyName::Opencode),
    ],
    &node_capability,
);
```

- [ ] **Step 4: Run the Rust tests to verify they pass**

Run: `cargo test services::installer::node_runtime::tests::`
Expected: PASS

Run: `cargo test node_and_npm_become_manual_when_windows_preflight_is_not_executable`
Expected: PASS

- [ ] **Step 5: Commit the detection slice**

```bash
git add src-tauri/src/services/installer/node_runtime.rs src-tauri/src/services/installer/mod.rs src-tauri/src/services/installer/detect.rs
git commit -m "feat: detect node install capability"
```

### Task 2: Execute and Verify Node Installation

**Files:**
- Modify: `src-tauri/src/services/installer/node_runtime.rs`
- Modify: `src-tauri/src/services/installer/install.rs`
- Test: `src-tauri/src/services/installer/node_runtime.rs`
- Test: `src-tauri/src/services/installer/install.rs`

- [ ] **Step 1: Write the failing install-command and verification tests**

Add these tests to `src-tauri/src/services/installer/node_runtime.rs`:

```rust
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
    assert!(script.contains("HOMEBREW_BREW_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git"));
    assert!(script.contains("HOMEBREW_CORE_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git"));
    assert!(script.contains("HOMEBREW_API_DOMAIN=https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api"));
}

#[test]
fn verify_node_runtime_marks_partial_runtime_as_broken() {
    let status = evaluate_node_runtime_verification(
        Some("v22.18.0".to_string()),
        None,
    );

    assert_eq!(status.state, InstallerDependencyState::Broken);
    assert_eq!(
        status.message.as_deref(),
        Some("Node.js is available on PATH, but npm is missing. Reinstall Node.js to repair npm.")
    );
}
```

Add this integration test to `src-tauri/src/services/installer/install.rs`:

```rust
#[test]
fn verification_result_step_surfaces_broken_runtime_message() {
    let step = super::verification_result_step(
        InstallerDependencyName::Node,
        "Installed Node.js.".to_string(),
        InstallerDependencyStatus {
            name: InstallerDependencyName::Node,
            kind: InstallerDependencyKind::Core,
            state: InstallerDependencyState::Broken,
            version: Some("v22.18.0".to_string()),
            path: Some("/usr/local/bin/node".to_string()),
            message: Some(
                "Node.js is available on PATH, but npm is missing. Reinstall Node.js to repair npm."
                    .to_string(),
            ),
            auto_install_supported: false,
        },
    );

    assert_eq!(step.stage, super::InstallProgressStage::Failed);
    assert_eq!(
        step.message,
        "Node.js is available on PATH, but npm is missing. Reinstall Node.js to repair npm."
    );
}
```

- [ ] **Step 2: Run the Rust tests to verify they fail**

Run: `cargo test windows_node_install_command_uses_winget_lts_with_accept_flags`
Expected: FAIL with `cannot find function 'build_windows_node_install_command'`

Run: `cargo test verify_node_runtime_marks_partial_runtime_as_broken`
Expected: FAIL with `cannot find function 'evaluate_node_runtime_verification'`

- [ ] **Step 3: Implement the Node install commands, mirror bootstrap, and runtime verification**

Add these builders and verification helpers to `src-tauri/src/services/installer/node_runtime.rs`:

```rust
use crate::services::installer::types::{
    InstallerDependencyKind, InstallerDependencyName, InstallerDependencyState,
    InstallerDependencyStatus,
};

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
        "export HOMEBREW_BREW_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git",
        "export HOMEBREW_CORE_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git",
        "export HOMEBREW_API_DOMAIN=https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api",
        "export HOMEBREW_INSTALL_FROM_API=1",
        "git clone --depth=1 https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git /tmp/cc-switch-homebrew-install",
        "NONINTERACTIVE=1 /bin/bash /tmp/cc-switch-homebrew-install/install.sh",
        "rm -rf /tmp/cc-switch-homebrew-install",
    ]
    .join(" && ")
}

pub fn evaluate_node_runtime_verification(
    node_version: Option<String>,
    npm_version: Option<String>,
) -> InstallerDependencyStatus {
    match (node_version, npm_version) {
        (Some(version), Some(_)) => InstallerDependencyStatus {
            name: InstallerDependencyName::Node,
            kind: InstallerDependencyKind::Core,
            state: InstallerDependencyState::Installed,
            version: Some(version),
            path: None,
            message: Some("Node.js and npm are available on PATH.".to_string()),
            auto_install_supported: true,
        },
        (Some(version), None) => InstallerDependencyStatus {
            name: InstallerDependencyName::Node,
            kind: InstallerDependencyKind::Core,
            state: InstallerDependencyState::Broken,
            version: Some(version),
            path: None,
            message: Some(
                "Node.js is available on PATH, but npm is missing. Reinstall Node.js to repair npm."
                    .to_string(),
            ),
            auto_install_supported: false,
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
            auto_install_supported: false,
        },
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
```

Add the install runner in the same file:

```rust
pub async fn install_node_runtime<F>(
    capability: &NodeInstallCapability,
    mut emit_progress: F,
) -> Result<String, String>
where
    F: FnMut(&str),
{
    match std::env::consts::OS {
        "windows" => {
            emit_progress("Checking winget availability...");
            emit_progress("Checking administrator privileges...");

            if !capability.auto_install_supported {
                return Err(
                    capability
                        .message
                        .clone()
                        .unwrap_or_else(|| "Node.js auto-install on Windows is not available.".to_string()),
                );
            }

            emit_progress("Installing Node.js with winget...");
            let (command, args) = build_windows_node_install_command();
            run_install_command(command, &args)
                .await
                .map(|_| "Installed Node.js with winget.".to_string())
        }
        "macos" | "darwin" => {
            emit_progress("Checking Homebrew availability...");

            if capability.brew_binary.is_none() {
                emit_progress("Installing Homebrew from domestic mirror...");
                run_macos_authorized_script(&build_homebrew_bootstrap_script()).await?;
            }

            emit_progress("Installing Node.js with Homebrew...");
            let brew = detect_homebrew_binary().unwrap_or_else(|| "brew".to_string());
            run_install_command(&brew, &["install", "node"])
                .await
                .map(|_| "Installed Node.js with Homebrew.".to_string())
        }
        _ => Err("Node.js auto-install is not supported on this platform.".to_string()),
    }
}
```

Use the existing `osascript` style from `src-tauri/src/commands/misc.rs` for macOS authorization:

```rust
#[cfg(target_os = "macos")]
async fn run_macos_authorized_script(script: &str) -> Result<(), String> {
    let escaped = script.replace('\\', "\\\\").replace('"', "\\\"");
    let applescript = format!(r#"do shell script "{}" with administrator privileges"#, escaped);

    let output = Command::new("osascript")
        .arg("-e")
        .arg(applescript)
        .output()
        .map_err(|error| format!("Failed to execute osascript: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}
```

Update `src-tauri/src/services/installer/install.rs` so the Node branch emits sub-steps and uses the new helper:

```rust
InstallerDependencyName::Node => {
    let capability = super::node_runtime::detect_node_install_capability();

    super::node_runtime::install_node_runtime(&capability, |message| {
        let step = InstallExecutionStep {
            name: InstallerDependencyName::Node,
            stage: InstallProgressStage::Installing,
            message: message.to_string(),
        };
        let _ = app.emit("installer-progress", &step);
        steps.push(step);
    })
    .await
}
```

After install, special-case Node verification before building the final step:

```rust
let status = if dependency == InstallerDependencyName::Node {
    let node_status = super::detect::detect_dependency_status(InstallerDependencyName::Node);
    let npm_status = super::detect::detect_dependency_status(InstallerDependencyName::Npm);

    super::node_runtime::evaluate_node_runtime_verification(
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
```

- [ ] **Step 4: Run the Rust tests to verify they pass**

Run: `cargo test services::installer::node_runtime::tests::`
Expected: PASS

Run: `cargo test verification_result_step_surfaces_broken_runtime_message`
Expected: PASS

- [ ] **Step 5: Commit the execution slice**

```bash
git add src-tauri/src/services/installer/node_runtime.rs src-tauri/src/services/installer/install.rs
git commit -m "feat: add node runtime installer"
```

### Task 3: Make Manual Commands Environment-Aware

**Files:**
- Modify: `src-tauri/src/services/installer/install.rs`
- Modify: `src-tauri/src/commands/installer.rs`
- Test: `src-tauri/src/services/installer/install.rs`

- [ ] **Step 1: Write the failing manual-command tests**

Add these tests to `src-tauri/src/services/installer/install.rs`:

```rust
#[test]
fn windows_node_manual_commands_explain_admin_retry_when_not_elevated() {
    let environment = crate::services::installer::types::InstallerEnvironment {
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

    let commands = get_manual_install_commands_for_environment(&environment);
    let node = commands
        .iter()
        .find(|item| item.name == InstallerDependencyName::Node)
        .expect("node command group");

    assert!(node.commands.iter().any(|command| command.contains("Run as administrator")));
    assert!(node.commands.iter().any(|command| command.contains("https://nodejs.org/en/download")));
}

#[test]
fn macos_node_manual_commands_include_tuna_bootstrap_when_brew_is_missing() {
    let environment = crate::services::installer::types::InstallerEnvironment {
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

    let commands = get_manual_install_commands_for_environment(&environment);
    let node = commands
        .iter()
        .find(|item| item.name == InstallerDependencyName::Node)
        .expect("node command group");

    assert!(node.commands.iter().any(|command| command.contains("mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git")));
    assert!(node.commands.iter().any(|command| command == "brew install node"));
}
```

- [ ] **Step 2: Run the Rust tests to verify they fail**

Run: `cargo test windows_node_manual_commands_explain_admin_retry_when_not_elevated`
Expected: FAIL with `cannot find function 'get_manual_install_commands_for_environment'`

Run: `cargo test macos_node_manual_commands_include_tuna_bootstrap_when_brew_is_missing`
Expected: FAIL for the same missing function

- [ ] **Step 3: Implement environment-aware manual commands and command wiring**

Replace the static platform-only helper in `src-tauri/src/services/installer/install.rs` with:

```rust
use crate::services::installer::types::InstallerEnvironment;

pub fn get_manual_install_commands_for_environment(
    environment: &InstallerEnvironment,
) -> Vec<ManualInstallCommandGroup> {
    let node_status = environment
        .dependencies
        .iter()
        .find(|dependency| dependency.name == InstallerDependencyName::Node);

    let node_commands = match environment.platform.as_str() {
        "windows" => {
            if node_status
                .and_then(|status| status.message.as_deref())
                .is_some_and(|message| message.contains("administrator"))
            {
                vec![
                    "Close DPCC-SWITCH, right-click it, and choose 'Run as administrator' before retrying Node.js install.".to_string(),
                    "If you prefer manual setup, download Node.js LTS from https://nodejs.org/en/download".to_string(),
                ]
            } else if node_status
                .and_then(|status| status.message.as_deref())
                .is_some_and(|message| message.contains("winget"))
            {
                vec![
                    "Install winget from https://aka.ms/getwinget, then reopen DPCC-SWITCH as administrator.".to_string(),
                    "Or install Node.js LTS manually from https://nodejs.org/en/download".to_string(),
                ]
            } else {
                vec!["Install Node.js LTS manually from https://nodejs.org/en/download".to_string()]
            }
        }
        "macos" | "darwin" => {
            if node_status
                .and_then(|status| status.message.as_deref())
                .is_some_and(|message| message.contains("Homebrew is missing"))
            {
                vec![
                    "export HOMEBREW_BREW_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git".to_string(),
                    "export HOMEBREW_CORE_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/homebrew-core.git".to_string(),
                    "export HOMEBREW_API_DOMAIN=https://mirrors.tuna.tsinghua.edu.cn/homebrew-bottles/api".to_string(),
                    "git clone --depth=1 https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git /tmp/cc-switch-homebrew-install".to_string(),
                    "NONINTERACTIVE=1 /bin/bash /tmp/cc-switch-homebrew-install/install.sh".to_string(),
                    "brew install node".to_string(),
                ]
            } else {
                vec!["brew install node".to_string()]
            }
        }
        _ => vec!["Install Node.js with your package manager or nvm.".to_string()],
    };

    let mut groups = Vec::new();
    groups.push(ManualInstallCommandGroup {
        name: InstallerDependencyName::Node,
        title: "Node.js".to_string(),
        commands: node_commands.clone(),
    });
    groups.push(ManualInstallCommandGroup {
        name: InstallerDependencyName::Pnpm,
        title: "pnpm".to_string(),
        commands: [node_commands.clone(), vec!["npm i -g pnpm@latest".to_string()]].concat(),
    });
    groups.push(ManualInstallCommandGroup {
        name: InstallerDependencyName::Codex,
        title: "Codex".to_string(),
        commands: [node_commands.clone(), vec!["npm i -g @openai/codex@latest".to_string()]].concat(),
    });
    groups.push(ManualInstallCommandGroup {
        name: InstallerDependencyName::Gemini,
        title: "Gemini CLI".to_string(),
        commands: [node_commands, vec!["npm i -g @google/gemini-cli@latest".to_string()]].concat(),
    });
    groups.push(ManualInstallCommandGroup {
        name: InstallerDependencyName::Git,
        title: "Git".to_string(),
        commands: match environment.platform.as_str() {
            "windows" => vec!["winget install --id Git.Git -e --source winget".to_string()],
            "macos" | "darwin" => vec!["Install Xcode Command Line Tools or Homebrew Git.".to_string()],
            _ => vec!["Install Git with your distro package manager.".to_string()],
        },
    });
    groups.push(ManualInstallCommandGroup {
        name: InstallerDependencyName::Claude,
        title: "Claude Code".to_string(),
        commands: vec!["curl -fsSL https://claude.ai/install.sh | bash".to_string()],
    });
    groups.push(ManualInstallCommandGroup {
        name: InstallerDependencyName::Opencode,
        title: "OpenCode".to_string(),
        commands: vec!["curl -fsSL https://opencode.ai/install | bash".to_string()],
    });

    groups
}
```

Update `src-tauri/src/commands/installer.rs`:

```rust
#[tauri::command]
pub async fn get_manual_install_commands(
) -> Result<Vec<crate::services::installer::install::ManualInstallCommandGroup>, String> {
    let environment = crate::services::installer::detect::detect_installer_environment();
    Ok(crate::services::installer::install::get_manual_install_commands_for_environment(
        &environment,
    ))
}
```

- [ ] **Step 4: Run the Rust tests to verify they pass**

Run: `cargo test windows_node_manual_commands_explain_admin_retry_when_not_elevated`
Expected: PASS

Run: `cargo test macos_node_manual_commands_include_tuna_bootstrap_when_brew_is_missing`
Expected: PASS

- [ ] **Step 5: Commit the manual-command slice**

```bash
git add src-tauri/src/services/installer/install.rs src-tauri/src/commands/installer.rs
git commit -m "feat: tailor node installer manual commands"
```

### Task 4: Lock the Existing UI Contract with Node Install Scenarios

**Files:**
- Modify: `src/components/settings/AboutSection.test.tsx`
- Modify: `src/components/settings/InstallerCenterDialog.test.tsx`
- Test: `src/components/settings/AboutSection.test.tsx`
- Test: `src/components/settings/InstallerCenterDialog.test.tsx`

- [ ] **Step 1: Write the failing UI contract tests**

Add this test to `src/components/settings/AboutSection.test.tsx`:

```tsx
test("shows the Windows admin retry message for node without exposing a selection checkbox", async () => {
  vi.mocked(installerApi.detectEnvironment).mockResolvedValueOnce({
    ...mockEnvironmentState,
    platform: "windows",
    dependencies: mockEnvironmentState.dependencies.map((dependency) =>
      dependency.name === "node"
        ? {
            ...dependency,
            state: "manual",
            version: null,
            path: null,
            message:
              "Node.js auto-install on Windows requires winget and DPCC-SWITCH to be reopened as administrator.",
            autoInstallSupported: false,
          }
        : dependency,
    ),
  });

  render(<AboutSection isPortable={false} />);

  expect(
    await screen.findByText(
      "Node.js auto-install on Windows requires winget and DPCC-SWITCH to be reopened as administrator.",
    ),
  ).toBeInTheDocument();
  expect(
    screen.queryByRole("checkbox", { name: "Select node" }),
  ).not.toBeInTheDocument();
});

test("renders the mirrored Homebrew bootstrap commands for macOS node setup", async () => {
  vi.mocked(installerApi.getManualCommands).mockResolvedValueOnce([
    {
      name: "node",
      title: "Node.js",
      commands: [
        "export HOMEBREW_BREW_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git",
        "git clone --depth=1 https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git /tmp/cc-switch-homebrew-install",
        "brew install node",
      ],
    },
  ]);

  render(<AboutSection isPortable={false} />);

  await userEvent.setup().click(
    await screen.findByRole("button", { name: "Manual Commands" }),
  );

  expect(
    await screen.findByText(
      "export HOMEBREW_BREW_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git",
    ),
  ).toBeInTheDocument();
  expect(screen.getByText("brew install node")).toBeInTheDocument();
});
```

Add this test to `src/components/settings/InstallerCenterDialog.test.tsx`:

```tsx
test("shows node progress text without changing the existing installer chrome", async () => {
  const { installerApi } = await import("@/lib/api/installer");

  vi.mocked(installerApi.installMissing).mockResolvedValueOnce({
    steps: [
      {
        name: "node",
        stage: "installing",
        message: "Installing Homebrew from domestic mirror...",
      },
      {
        name: "node",
        stage: "verifying",
        message: "Verifying node and npm on PATH...",
      },
    ],
    completedDependencies: [],
    failedDependencies: [],
    manualDependencies: [],
    statusMessage: "done",
  });

  render(<InstallerCenterDialog open onOpenChange={() => {}} />);

  expect(await screen.findByText("环境检测与安装")).toBeInTheDocument();
  expect(screen.getByText("安装进度")).toBeInTheDocument();
});
```

- [ ] **Step 2: Run the UI tests to verify they fail**

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx src/components/settings/InstallerCenterDialog.test.tsx`
Expected: FAIL because the new assertions are not present yet

- [ ] **Step 3: Update the mocked installer payloads to match the new backend contract**

Keep the component tree unchanged; only add the new scenario coverage above and preserve the current UI contract. Update the shared mock payloads so the Node manual-command fixture and installer-progress fixture reflect the backend messages introduced in Tasks 1-3.

Use these exact mock additions:

```tsx
{
  name: "node",
  kind: "core",
  state: "manual",
  version: null,
  path: null,
  message:
    "Node.js auto-install on Windows requires winget and DPCC-SWITCH to be reopened as administrator.",
  autoInstallSupported: false,
}
```

```tsx
{
  name: "node",
  title: "Node.js",
  commands: [
    "export HOMEBREW_BREW_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git",
    "git clone --depth=1 https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/install.git /tmp/cc-switch-homebrew-install",
    "brew install node",
  ],
}
```

- [ ] **Step 4: Run the UI tests to verify they pass**

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx src/components/settings/InstallerCenterDialog.test.tsx`
Expected: PASS

- [ ] **Step 5: Commit the UI contract slice**

```bash
git add src/components/settings/AboutSection.test.tsx src/components/settings/InstallerCenterDialog.test.tsx
git commit -m "test: cover node installer ui states"
```

### Task 5: Run Full Verification and Smoke Checks

**Files:**
- Modify: `src-tauri/src/services/installer/node_runtime.rs`
- Modify: `src-tauri/src/services/installer/detect.rs`
- Modify: `src-tauri/src/services/installer/install.rs`
- Modify: `src-tauri/src/commands/installer.rs`
- Modify: `src/components/settings/AboutSection.test.tsx`
- Modify: `src/components/settings/InstallerCenterDialog.test.tsx`

- [ ] **Step 1: Run the full installer-focused automated verification**

Run: `cargo test services::installer::`
Expected: PASS with the new `node_runtime`, detect, install, and manual-command tests included

Run: `pnpm test:unit src/components/settings/AboutSection.test.tsx src/components/settings/InstallerCenterDialog.test.tsx`
Expected: PASS

Run: `pnpm typecheck`
Expected: PASS

- [ ] **Step 2: Smoke-test the real platform flows**

Run this checklist on Windows:

```text
1. Start DPCC-SWITCH without elevation and confirm Node shows a manual/admin retry message.
2. Reopen DPCC-SWITCH as administrator with winget installed and confirm Node becomes selectable.
3. Trigger Node install and confirm progress includes:
   - Checking winget availability...
   - Checking administrator privileges...
   - Installing Node.js with winget...
   - Verifying node and npm on PATH...
4. Confirm `node --version` and `npm --version` both work in a fresh shell.
```

Run this checklist on macOS:

```text
1. With brew already installed, confirm Node install goes straight to `brew install node`.
2. On a machine without brew, confirm progress includes:
   - Checking Homebrew availability...
   - Installing Homebrew from domestic mirror...
   - Installing Node.js with Homebrew...
   - Verifying node and npm on PATH...
3. Reject the macOS authorization prompt once and confirm the install stops with mirrored manual commands visible.
4. Accept the authorization prompt and confirm `node --version` plus `npm --version` both work in a fresh shell.
```

- [ ] **Step 3: Commit the finished feature after verification**

```bash
git add src-tauri/src/services/installer/node_runtime.rs src-tauri/src/services/installer/mod.rs src-tauri/src/services/installer/detect.rs src-tauri/src/services/installer/install.rs src-tauri/src/commands/installer.rs src/components/settings/AboutSection.test.tsx src/components/settings/InstallerCenterDialog.test.tsx
git commit -m "feat: add node auto install workflow"
```
