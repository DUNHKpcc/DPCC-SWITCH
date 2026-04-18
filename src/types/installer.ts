export type InstallerDependencyKind = "core" | "tool";

export type InstallerDependencyName =
  | "node"
  | "npm"
  | "pnpm"
  | "git"
  | "claude"
  | "codex"
  | "gemini"
  | "opencode";

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
  name: InstallerDependencyName;
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
  name: InstallerDependencyName;
  title: string;
  commands: string[];
}

export interface InstallExecutionStep {
  name: InstallerDependencyName;
  stage: InstallProgressStage;
  message: string;
}

export interface InstallerRunResult {
  steps: InstallExecutionStep[];
  completedDependencies: InstallerDependencyName[];
  failedDependencies: InstallerDependencyName[];
  manualDependencies: InstallerDependencyName[];
  statusMessage: string;
}
