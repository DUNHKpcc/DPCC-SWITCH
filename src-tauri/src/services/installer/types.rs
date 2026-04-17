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
