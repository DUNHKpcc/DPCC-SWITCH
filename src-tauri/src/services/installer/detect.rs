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
        detect_binary(
            "node",
            InstallerDependencyName::Node,
            InstallerDependencyKind::Core,
        ),
        detect_binary(
            "npm",
            InstallerDependencyName::Npm,
            InstallerDependencyKind::Core,
        ),
        detect_binary(
            "git",
            InstallerDependencyName::Git,
            InstallerDependencyKind::Core,
        ),
        detect_binary(
            "claude",
            InstallerDependencyName::Claude,
            InstallerDependencyKind::Tool,
        ),
        detect_binary(
            "codex",
            InstallerDependencyName::Codex,
            InstallerDependencyKind::Tool,
        ),
        detect_binary(
            "gemini",
            InstallerDependencyName::Gemini,
            InstallerDependencyKind::Tool,
        ),
        detect_binary(
            "opencode",
            InstallerDependencyName::Opencode,
            InstallerDependencyKind::Tool,
        ),
    ];

    let ready_count = dependencies
        .iter()
        .filter(|dependency| dependency.state == InstallerDependencyState::Installed)
        .count();

    InstallerEnvironment {
        platform: platform.clone(),
        auto_install_supported: matches!(platform.as_str(), "windows" | "macos" | "darwin"),
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
    let auto_install_supported = match (std::env::consts::OS, binary) {
        ("linux", _) => false,
        ("macos", "git") | ("darwin", "git") => false,
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
