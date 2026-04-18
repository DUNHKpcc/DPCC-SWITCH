import { useCallback, useEffect, useState } from "react";
import {
  Info,
  Loader2,
  RefreshCw,
  Shield,
  Terminal,
  CheckCircle2,
  AlertCircle,
  Wrench,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useTranslation } from "react-i18next";
import { getVersion } from "@tauri-apps/api/app";
import { settingsApi } from "@/lib/api";
import { installerApi } from "@/lib/api/installer";
import { useUpdate } from "@/contexts/UpdateContext";
import { Badge } from "@/components/ui/badge";
import { motion } from "framer-motion";
import appIcon from "@/assets/icons/app-icon.png";
import { isWindows } from "@/lib/platform";
import { InstallerCenterDialog } from "@/components/settings/InstallerCenterDialog";
import { LocalEnvironmentCard } from "@/components/settings/LocalEnvironmentCard";
import { APP_UPDATES_ENABLED } from "@/lib/updatePolicy";
import type {
  InstallerDependencyState,
  InstallerDependencyStatus,
} from "@/types/installer";

interface AboutSectionProps {
  isPortable: boolean;
}

interface ToolVersion {
  name: string;
  version: string | null;
  latest_version: string | null;
  error: string | null;
  env_type: "windows" | "wsl" | "macos" | "linux" | "unknown";
  wsl_distro: string | null;
}

const TOOL_NAMES = ["claude", "codex", "gemini", "opencode"] as const;
type ToolName = (typeof TOOL_NAMES)[number];

type WslShellPreference = {
  wslShell?: string | null;
  wslShellFlag?: string | null;
};

const WSL_SHELL_OPTIONS = ["sh", "bash", "zsh", "fish", "dash"] as const;
// UI-friendly order: login shell first.
const WSL_SHELL_FLAG_OPTIONS = ["-lic", "-lc", "-c"] as const;

const ENV_BADGE_CONFIG: Record<
  string,
  { labelKey: string; className: string }
> = {
  wsl: {
    labelKey: "settings.envBadge.wsl",
    className:
      "bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-500/20",
  },
  windows: {
    labelKey: "settings.envBadge.windows",
    className:
      "bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20",
  },
  macos: {
    labelKey: "settings.envBadge.macos",
    className:
      "bg-gray-500/10 text-gray-600 dark:text-gray-400 border-gray-500/20",
  },
  linux: {
    labelKey: "settings.envBadge.linux",
    className:
      "bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20",
  },
};

const DEPENDENCY_STATE_CLASS_NAME: Record<InstallerDependencyState, string> = {
  installed:
    "border-emerald-500/20 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300",
  missing:
    "border-amber-500/20 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  outdated:
    "border-amber-500/20 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  broken:
    "border-red-500/20 bg-red-500/10 text-red-700 dark:text-red-300",
  manual: "border-sky-500/20 bg-sky-500/10 text-sky-700 dark:text-sky-300",
};

const DEPENDENCY_STATE_LABEL_KEY: Record<InstallerDependencyState, string> = {
  installed: "settings.installerDependencyState.installed",
  missing: "settings.installerDependencyState.missing",
  outdated: "settings.installerDependencyState.outdated",
  broken: "settings.installerDependencyState.broken",
  manual: "settings.installerDependencyState.manual",
};

const LOCAL_ENV_GRID_CLASS_NAME = "grid gap-3 sm:grid-cols-2 lg:grid-cols-4";

export function AboutSection({ isPortable }: AboutSectionProps) {
  // ... (use hooks as before) ...
  const { t } = useTranslation();
  const [version, setVersion] = useState<string | null>(null);
  const [isLoadingVersion, setIsLoadingVersion] = useState(true);
  const [toolVersions, setToolVersions] = useState<ToolVersion[]>([]);
  const [isLoadingTools, setIsLoadingTools] = useState(true);
  const [installerOpen, setInstallerOpen] = useState(false);
  const [coreDependencies, setCoreDependencies] = useState<
    InstallerDependencyStatus[]
  >([]);

  const {
    hasUpdate,
    updateInfo,
    checkUpdate,
    isChecking,
  } = useUpdate();

  const [wslShellByTool, setWslShellByTool] = useState<
    Record<string, WslShellPreference>
  >({});
  const [loadingTools, setLoadingTools] = useState<Record<string, boolean>>({});

  const refreshToolVersions = useCallback(
    async (
      toolNames: ToolName[],
      wslOverrides?: Record<string, WslShellPreference>,
    ) => {
      if (toolNames.length === 0) return;

      // 单工具刷新使用统一后端入口（get_tool_versions）并带工具过滤。
      setLoadingTools((prev) => {
        const next = { ...prev };
        for (const name of toolNames) next[name] = true;
        return next;
      });

      try {
        const updated = await settingsApi.getToolVersions(
          toolNames,
          wslOverrides,
        );

        setToolVersions((prev) => {
          if (prev.length === 0) return updated;
          const byName = new Map(updated.map((t) => [t.name, t]));
          const merged = prev.map((t) => byName.get(t.name) ?? t);
          const existing = new Set(prev.map((t) => t.name));
          for (const u of updated) {
            if (!existing.has(u.name)) merged.push(u);
          }
          return merged;
        });
      } catch (error) {
        console.error("[AboutSection] Failed to refresh tools", error);
      } finally {
        setLoadingTools((prev) => {
          const next = { ...prev };
          for (const name of toolNames) next[name] = false;
          return next;
        });
      }
    },
    [],
  );

  const loadAllToolVersions = useCallback(async () => {
    setIsLoadingTools(true);
    try {
      const [versionsResult, environmentResult] = await Promise.allSettled([
        settingsApi.getToolVersions([...TOOL_NAMES], wslShellByTool),
        installerApi.detectEnvironment(),
      ]);

      if (versionsResult.status === "fulfilled") {
        setToolVersions(versionsResult.value);
      } else {
        console.error(
          "[AboutSection] Failed to load tool versions",
          versionsResult.reason,
        );
      }

      if (environmentResult.status === "fulfilled") {
        setCoreDependencies(
          environmentResult.value.dependencies.filter(
            (dependency) => dependency.kind === "core",
          ),
        );
      } else {
        console.error(
          "[AboutSection] Failed to load core dependencies",
          environmentResult.reason,
        );
      }
    } catch (error) {
      console.error("[AboutSection] Failed to load tool versions", error);
    } finally {
      setIsLoadingTools(false);
    }
  }, [wslShellByTool]);

  const handleToolShellChange = async (toolName: ToolName, value: string) => {
    const wslShell = value === "auto" ? null : value;
    const nextPref: WslShellPreference = {
      ...(wslShellByTool[toolName] ?? {}),
      wslShell,
    };
    setWslShellByTool((prev) => ({ ...prev, [toolName]: nextPref }));
    await refreshToolVersions([toolName], { [toolName]: nextPref });
  };

  const handleToolShellFlagChange = async (
    toolName: ToolName,
    value: string,
  ) => {
    const wslShellFlag = value === "auto" ? null : value;
    const nextPref: WslShellPreference = {
      ...(wslShellByTool[toolName] ?? {}),
      wslShellFlag,
    };
    setWslShellByTool((prev) => ({ ...prev, [toolName]: nextPref }));
    await refreshToolVersions([toolName], { [toolName]: nextPref });
  };

  useEffect(() => {
    let active = true;
    const load = async () => {
      try {
        const [appVersion] = await Promise.all([
          getVersion(),
          ...(isWindows() ? [] : [loadAllToolVersions()]),
        ]);

        if (active) {
          setVersion(appVersion);
        }
      } catch (error) {
        console.error("[AboutSection] Failed to load info", error);
        if (active) {
          setVersion(null);
        }
      } finally {
        if (active) {
          setIsLoadingVersion(false);
        }
      }
    };

    void load();
    return () => {
      active = false;
    };
    // Mount-only: loadAllToolVersions is intentionally excluded to avoid
    // re-fetching all tools whenever wslShellByTool changes. Single-tool
    // refreshes are handled by refreshToolVersions in the shell/flag handlers.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const displayVersion = version ?? t("common.unknown");

  return (
    <motion.section
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
      className="space-y-6"
    >
      <header className="space-y-1">
        <h3 className="text-sm font-medium">{t("common.about")}</h3>
        <p className="text-xs text-muted-foreground">
          {t("settings.aboutHint")}
        </p>
      </header>

      <motion.div
        initial={{ opacity: 0, scale: 0.98 }}
        animate={{ opacity: 1, scale: 1 }}
        transition={{ duration: 0.3, delay: 0.1 }}
        className="rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-6 space-y-5 shadow-sm"
      >
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-2">
            <div className="flex items-center gap-2">
              <img src={appIcon} alt="DPCC-SWITCH" className="h-5 w-5" />
              <h4 className="text-lg font-semibold text-foreground">
                DPCC-SWITCH
              </h4>
            </div>
            <div className="flex items-center gap-2">
              <Badge variant="outline" className="gap-1.5 bg-background/80">
                <span className="text-muted-foreground">
                  {t("common.version")}
                </span>
                {isLoadingVersion ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <span className="font-medium">{`v${displayVersion}`}</span>
                )}
              </Badge>
              {isPortable && APP_UPDATES_ENABLED && (
                <Badge variant="secondary" className="gap-1.5">
                  <Info className="h-3 w-3" />
                  {t("settings.portableMode")}
                </Badge>
              )}
            </div>
          </div>

          {APP_UPDATES_ENABLED && (
            <div className="flex items-center gap-2">
              <Button
                type="button"
                size="sm"
                disabled={isChecking}
                className="h-8 gap-1.5 text-xs"
                onClick={() => {
                  void checkUpdate();
                }}
              >
                <RefreshCw
                  className={isChecking ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"}
                />
                {t("settings.checkForUpdates")}
              </Button>
            </div>
          )}
        </div>

        {APP_UPDATES_ENABLED && hasUpdate && updateInfo && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            className="rounded-lg bg-primary/10 border border-primary/20 px-4 py-3 text-sm"
          >
            <p className="font-medium text-primary mb-1">
              {t("settings.updateAvailable", {
                version: updateInfo.availableVersion,
              })}
            </p>
            {updateInfo.notes && (
              <p className="text-muted-foreground line-clamp-3 leading-relaxed">
                {updateInfo.notes}
              </p>
            )}
          </motion.div>
        )}
      </motion.div>

      {!isWindows() && (
        <div className="space-y-3">
          <div className="flex items-center justify-between px-1">
            <h3 className="text-sm font-medium">
              {t("settings.localEnvCheck")}
            </h3>
            <Button
              size="sm"
              variant="outline"
              className="h-7 gap-1.5 text-xs"
              onClick={() => loadAllToolVersions()}
              disabled={isLoadingTools}
            >
              <RefreshCw
                className={
                  isLoadingTools ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"
                }
              />
              {isLoadingTools ? t("common.refreshing") : t("common.refresh")}
            </Button>
          </div>

          {coreDependencies.length > 0 && (
            <div className="space-y-3 px-1">
              <h4 className="text-xs font-semibold uppercase tracking-[0.12em] text-muted-foreground">
                {t("settings.installerCenterCoreDependencies", {
                  defaultValue: "Core Dependencies",
                })}
              </h4>
              <div
                data-testid="local-env-core-grid"
                className={LOCAL_ENV_GRID_CLASS_NAME}
              >
                {coreDependencies.map((dependency, index) => (
                  <LocalEnvironmentCard
                    key={dependency.name}
                    testId={`local-env-card-${dependency.name}`}
                    delay={0.1 + index * 0.05}
                    header={
                      <div className="flex items-center justify-between gap-3">
                        <div className="flex min-w-0 items-center gap-2">
                          <Terminal className="h-4 w-4 text-muted-foreground" />
                          <span className="text-sm font-medium lowercase">
                            {dependency.name}
                          </span>
                        </div>
                        {dependency.state === "installed" ? (
                          <CheckCircle2
                            data-testid={`local-env-status-icon-${dependency.name}`}
                            className="h-4 w-4 text-green-500"
                          />
                        ) : (
                          <Badge
                            className={`capitalize ${DEPENDENCY_STATE_CLASS_NAME[dependency.state]}`}
                          >
                            {t(DEPENDENCY_STATE_LABEL_KEY[dependency.state], {
                              defaultValue: dependency.state,
                            })}
                          </Badge>
                        )}
                      </div>
                    }
                    footer={
                      <div className="flex items-center justify-between gap-3">
                        <div
                          className="min-w-0 flex-1 truncate text-xs font-mono text-muted-foreground"
                          title={
                            dependency.version ??
                            dependency.message ??
                            t("settings.installerVersionUnavailable", {
                              defaultValue: "Version unavailable",
                            })
                          }
                        >
                          {dependency.version ??
                            t("settings.installerVersionUnavailable", {
                              defaultValue: "Version unavailable",
                            })}
                        </div>
                        {dependency.path ? (
                          <div
                            data-testid={`local-env-path-${dependency.name}`}
                            className="min-w-0 max-w-[58%] truncate text-right text-[11px] text-muted-foreground/90"
                            title={dependency.path}
                          >
                            {dependency.path}
                          </div>
                        ) : null}
                      </div>
                    }
                  />
                ))}
              </div>
            </div>
          )}

          <div
            data-testid="local-env-tool-grid"
            className={`${LOCAL_ENV_GRID_CLASS_NAME} px-1`}
          >
            {TOOL_NAMES.map((toolName, index) => {
              const tool = toolVersions.find((item) => item.name === toolName);
              // Special case for OpenCode (capital C), others use capitalize
              const displayName =
                toolName === "opencode"
                  ? "OpenCode"
                  : toolName.charAt(0).toUpperCase() + toolName.slice(1);
              const title = tool?.version || tool?.error || t("common.unknown");

              return (
                <LocalEnvironmentCard
                  key={toolName}
                  testId={`local-env-card-${toolName}`}
                  delay={0.15 + index * 0.05}
                  header={
                    <div className="flex items-center justify-between gap-3">
                      <div className="flex min-w-0 flex-wrap items-center gap-2">
                      <Terminal className="h-4 w-4 text-muted-foreground" />
                      <span className="text-sm font-medium">{displayName}</span>
                      {tool?.env_type && ENV_BADGE_CONFIG[tool.env_type] && (
                        <span
                          className={`text-[9px] px-1.5 py-0.5 rounded-full border ${ENV_BADGE_CONFIG[tool.env_type].className}`}
                        >
                          {t(ENV_BADGE_CONFIG[tool.env_type].labelKey)}
                        </span>
                      )}
                      {tool?.env_type === "wsl" && (
                        <Select
                          value={wslShellByTool[toolName]?.wslShell || "auto"}
                          onValueChange={(v) =>
                            handleToolShellChange(toolName, v)
                          }
                          disabled={isLoadingTools || loadingTools[toolName]}
                        >
                          <SelectTrigger className="h-6 w-[70px] text-xs">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="auto">
                              {t("common.auto")}
                            </SelectItem>
                            {WSL_SHELL_OPTIONS.map((shell) => (
                              <SelectItem key={shell} value={shell}>
                                {shell}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      )}
                      {tool?.env_type === "wsl" && (
                        <Select
                          value={
                            wslShellByTool[toolName]?.wslShellFlag || "auto"
                          }
                          onValueChange={(v) =>
                            handleToolShellFlagChange(toolName, v)
                          }
                          disabled={isLoadingTools || loadingTools[toolName]}
                        >
                          <SelectTrigger className="h-6 w-[70px] text-xs">
                            <SelectValue />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="auto">
                              {t("common.auto")}
                            </SelectItem>
                            {WSL_SHELL_FLAG_OPTIONS.map((flag) => (
                              <SelectItem key={flag} value={flag}>
                                {flag}
                              </SelectItem>
                            ))}
                          </SelectContent>
                        </Select>
                      )}
                      </div>
                      {isLoadingTools || loadingTools[toolName] ? (
                        <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
                      ) : tool?.version ? (
                        tool.latest_version &&
                        tool.version !== tool.latest_version ? (
                          <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-yellow-500/10 text-yellow-600 dark:text-yellow-400 border border-yellow-500/20">
                            {tool.latest_version}
                          </span>
                        ) : (
                          <CheckCircle2 className="h-4 w-4 text-green-500" />
                        )
                      ) : (
                        <AlertCircle className="h-4 w-4 text-yellow-500" />
                      )}
                    </div>
                  }
                  footer={
                    <div
                      className="text-xs font-mono text-muted-foreground truncate"
                      title={title}
                    >
                      {isLoadingTools
                        ? t("common.loading")
                        : tool?.version
                          ? tool.version
                          : tool?.error || t("common.notInstalled")}
                    </div>
                  }
                />
              );
            })}
          </div>
        </div>
      )}

      <motion.div
        initial={{ opacity: 0, y: 10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.3, delay: 0.3 }}
        className="space-y-3"
      >
        <h3 className="px-1 text-sm font-medium">
          {t("settings.installerCenter", {
            defaultValue: "Environment Check & Install",
          })}
        </h3>
        <div className="rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-4 shadow-sm">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div className="space-y-1">
              <div className="flex items-center gap-2 text-sm font-medium">
                <Shield className="h-4 w-4 text-primary" />
                {t("settings.installerCenter", {
                  defaultValue: "Environment Check & Install",
                })}
              </div>
              <p className="text-xs text-muted-foreground">
                {t("settings.installerCenterHint", {
                  defaultValue:
                    "Detect local dependencies and install supported CLI tools from one place.",
                })}
              </p>
            </div>
            <Button
              size="sm"
              className="h-8 gap-1.5 text-xs"
              onClick={() => setInstallerOpen(true)}
            >
              <Wrench className="h-3.5 w-3.5" />
              {t("settings.openInstallerCenter", {
                defaultValue: "Environment Check & Install",
              })}
            </Button>
          </div>
        </div>
        <InstallerCenterDialog
          open={installerOpen}
          onOpenChange={setInstallerOpen}
        />
      </motion.div>
    </motion.section>
  );
}
