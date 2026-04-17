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

#[cfg(test)]
mod tests {
    use super::{build_install_plan, get_manual_install_commands};
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
    }
}
