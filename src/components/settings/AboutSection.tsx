import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertCircle,
  CheckCircle2,
  Info,
  Loader2,
  RefreshCw,
  Terminal,
} from "lucide-react";
import { motion } from "framer-motion";
import { getVersion } from "@tauri-apps/api/app";
import { useTranslation } from "react-i18next";

import appIcon from "@/assets/icons/app-icon.png";
import gitIcon from "@/icons/extracted/git.png";
import nodeIcon from "@/icons/extracted/node.png";
import npmIcon from "@/icons/extracted/npm.png";
import pnpmIcon from "@/icons/extracted/pnpm.png";
import { LocalEnvironmentCard } from "@/components/settings/LocalEnvironmentCard";
import { InstallerProgressPanel } from "@/components/settings/InstallerProgressPanel";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useUpdate } from "@/contexts/UpdateContext";
import { settingsApi } from "@/lib/api";
import { installerApi } from "@/lib/api/installer";
import { isWindows } from "@/lib/platform";
import { APP_UPDATES_ENABLED } from "@/lib/updatePolicy";
import type {
  InstallExecutionStep,
  InstallerDependencyName,
  InstallerDependencyState,
  InstallerDependencyStatus,
  ManualInstallCommandGroup,
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

const CORE_DEPENDENCY_NAMES = [
  "node",
  "npm",
  "pnpm",
  "git",
] as const satisfies readonly InstallerDependencyName[];
const TOOL_NAMES = [
  "claude",
  "codex",
  "gemini",
  "opencode",
] as const satisfies readonly InstallerDependencyName[];
type ToolName = (typeof TOOL_NAMES)[number];

type WslShellPreference = {
  wslShell?: string | null;
  wslShellFlag?: string | null;
};

const WSL_SHELL_OPTIONS = ["sh", "bash", "zsh", "fish", "dash"] as const;
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
      "bg-black/5 text-foreground border-border-default dark:bg-white/10 dark:text-foreground",
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
  manual: "border-border-default bg-muted text-foreground",
};

const DEPENDENCY_STATE_LABEL_KEY: Record<InstallerDependencyState, string> = {
  installed: "settings.installerDependencyState.installed",
  missing: "settings.installerDependencyState.missing",
  outdated: "settings.installerDependencyState.outdated",
  broken: "settings.installerDependencyState.broken",
  manual: "settings.installerDependencyState.manual",
};

const LOCAL_ENV_GRID_CLASS_NAME = "grid gap-3 px-2 sm:grid-cols-2 lg:grid-cols-4";
const FEATURE_CARD_GRID_CLASS_NAME = "grid gap-3 px-3 lg:grid-cols-4";
const FEATURE_CARD_SPAN_CLASS_NAME = "min-w-0 lg:col-span-2";
const CORE_DEPENDENCY_ICON_SRC = {
  node: nodeIcon,
  npm: npmIcon,
  pnpm: pnpmIcon,
  git: gitIcon,
} as const;

function isPendingDependency(dependency?: InstallerDependencyStatus) {
  return dependency?.state === "missing";
}

function getToolDisplayName(toolName: ToolName) {
  return toolName === "opencode"
    ? "OpenCode"
    : toolName.charAt(0).toUpperCase() + toolName.slice(1);
}

function formatDependencyPath(path: string) {
  const separator = path.includes("\\") ? "\\" : "/";
  const segments = path.split(/[/\\]+/).filter(Boolean);

  if (segments.length <= 3) {
    return path;
  }

  return `...${separator}${segments.slice(-2).join(separator)}`;
}

export function AboutSection({ isPortable }: AboutSectionProps) {
  const { t } = useTranslation();
  const [version, setVersion] = useState<string | null>(null);
  const [isLoadingVersion, setIsLoadingVersion] = useState(true);
  const [toolVersions, setToolVersions] = useState<ToolVersion[]>([]);
  const [dependencies, setDependencies] = useState<InstallerDependencyStatus[]>([]);
  const [manualCommands, setManualCommands] = useState<ManualInstallCommandGroup[]>(
    [],
  );
  const [progress, setProgress] = useState<InstallExecutionStep[]>([]);
  const [isLoadingTools, setIsLoadingTools] = useState(true);
  const [installing, setInstalling] = useState(false);
  const [showManualCommands, setShowManualCommands] = useState(false);
  const [selectedDependencies, setSelectedDependencies] = useState<
    InstallerDependencyName[]
  >([]);

  const {
    hasUpdate,
    updateInfo,
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
          const byName = new Map(updated.map((tool) => [tool.name, tool]));
          const merged = prev.map((tool) => byName.get(tool.name) ?? tool);
          const existing = new Set(prev.map((tool) => tool.name));
          for (const tool of updated) {
            if (!existing.has(tool.name)) merged.push(tool);
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

  const loadInstallerState = useCallback(async () => {
    setIsLoadingTools(true);
    try {
      const [versionsResult, environmentResult, manualCommandsResult] =
        await Promise.allSettled([
          settingsApi.getToolVersions([...TOOL_NAMES], wslShellByTool),
          installerApi.detectEnvironment(),
          installerApi.getManualCommands(),
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
        setDependencies(environmentResult.value.dependencies);
      } else {
        console.error(
          "[AboutSection] Failed to load dependencies",
          environmentResult.reason,
        );
      }

      if (manualCommandsResult.status === "fulfilled") {
        setManualCommands(manualCommandsResult.value);
      } else {
        console.error(
          "[AboutSection] Failed to load manual commands",
          manualCommandsResult.reason,
        );
      }
    } catch (error) {
      console.error("[AboutSection] Failed to load installer state", error);
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
          ...(isWindows() ? [] : [loadInstallerState()]),
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    if (isWindows()) return;

    let active = true;
    let unlisten: (() => void) | undefined;

    void (async () => {
      try {
        unlisten = await installerApi.subscribeProgress((event) => {
          if (!active) return;
          setProgress((current) => [...current, event].slice(-30));
        });
      } catch (error) {
        console.error("[AboutSection] Failed to subscribe installer progress", error);
      }
    })();

    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  const dependenciesByName = useMemo(
    () => new Map(dependencies.map((dependency) => [dependency.name, dependency])),
    [dependencies],
  );
  const toolVersionsByName = useMemo(
    () => new Map(toolVersions.map((tool) => [tool.name, tool])),
    [toolVersions],
  );

  const installableDependencies = useMemo(
    () =>
      [...CORE_DEPENDENCY_NAMES, ...TOOL_NAMES].filter((name) => {
        const dependency = dependenciesByName.get(name);
        return Boolean(
          dependency &&
            isPendingDependency(dependency) &&
            dependency.autoInstallSupported,
        );
      }),
    [dependenciesByName],
  );

  useEffect(() => {
    setSelectedDependencies((current) =>
      current.filter((name) => installableDependencies.includes(name)),
    );
  }, [installableDependencies]);

  const handleToggleSelected = (name: InstallerDependencyName, checked: boolean) => {
    setSelectedDependencies((current) => {
      if (checked) {
        return current.includes(name) ? current : [...current, name];
      }
      return current.filter((item) => item !== name);
    });
  };

  const runInstall = useCallback(
    async (task: () => Promise<{ steps: InstallExecutionStep[] }>) => {
      setInstalling(true);
      setProgress([]);
      try {
        const result = await task();
        if (result.steps.length > 0) {
          setProgress(result.steps);
        }
        await loadInstallerState();
      } catch (error) {
        console.error("[AboutSection] Failed to install dependencies", error);
      } finally {
        setInstalling(false);
      }
    },
    [loadInstallerState],
  );

  const handleInstallAll = async () => {
    await runInstall(() => installerApi.installMissing());
  };

  const handleInstallSelected = async (names = selectedDependencies) => {
    if (names.length === 0) return;
    await runInstall(() => installerApi.installSelected(names));
  };

  const displayVersion = version ?? t("common.unknown");
  const installSelectedLabel =
    selectedDependencies.length > 0
      ? t("settings.installSelectedDependenciesCount", {
          defaultValue: `Install Selected Dependencies (${selectedDependencies.length})`,
          count: selectedDependencies.length,
        })
      : t("settings.installSelectedDependencies", {
          defaultValue: "Install Selected Dependencies",
        });

  const renderDependencyCard = (
    dependencyName: InstallerDependencyName,
    index: number,
  ) => {
    const dependency = dependenciesByName.get(dependencyName);
    const tool =
      TOOL_NAMES.includes(dependencyName as ToolName)
        ? toolVersionsByName.get(dependencyName)
        : undefined;
    const displayName = TOOL_NAMES.includes(dependencyName as ToolName)
      ? getToolDisplayName(dependencyName as ToolName)
      : dependencyName;
    const versionUnavailable = t("settings.installerVersionUnavailable", {
      defaultValue: "Version unavailable",
    });
    const fallbackDetail = dependency
      ? isPendingDependency(dependency) || dependency.state === "manual" || dependency.state === "broken"
        ? t("common.notInstalled")
        : versionUnavailable
      : t("common.unknown");
    const detailTitle = isLoadingTools
      ? t("common.loading")
      : dependency?.version ?? tool?.version ?? fallbackDetail;
    const detailValue = isLoadingTools
      ? t("common.loading")
      : dependency?.version ?? tool?.version ?? fallbackDetail;
    const message = !isLoadingTools
      ? dependency?.message ?? tool?.error ?? null
      : null;
    const isSelectable = Boolean(
      dependency &&
        isPendingDependency(dependency) &&
        dependency.autoInstallSupported &&
        !installing,
    );
    const isCardInstalling =
      installing ||
      (TOOL_NAMES.includes(dependencyName as ToolName) &&
        Boolean(loadingTools[dependencyName]));
    const coreDependencyIcon = CORE_DEPENDENCY_ICON_SRC[
      dependencyName as keyof typeof CORE_DEPENDENCY_ICON_SRC
    ];
    const displayPath = dependency?.path
      ? formatDependencyPath(dependency.path)
      : null;

    return (
      <LocalEnvironmentCard
        key={dependencyName}
        testId={`local-env-card-${dependencyName}`}
        delay={0.1 + index * 0.05}
        header={
          <div className="flex items-start justify-between gap-3">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              {coreDependencyIcon ? (
                <img
                  src={coreDependencyIcon}
                  alt={`${dependencyName} icon`}
                  className="mt-0.5 h-4 w-4 shrink-0 object-contain"
                />
              ) : (
                <Terminal className="mt-0.5 h-4 w-4 text-muted-foreground" />
              )}
              <span className="text-sm font-medium lowercase">{displayName}</span>
              {tool?.env_type && ENV_BADGE_CONFIG[tool.env_type] && (
                <span
                  className={`text-[9px] px-1.5 py-0.5 rounded-full border ${ENV_BADGE_CONFIG[tool.env_type].className}`}
                >
                  {t(ENV_BADGE_CONFIG[tool.env_type].labelKey)}
                </span>
              )}
              {tool?.env_type === "wsl" && (
                <Select
                  value={wslShellByTool[dependencyName]?.wslShell || "auto"}
                  onValueChange={(value) =>
                    handleToolShellChange(dependencyName as ToolName, value)
                  }
                  disabled={isLoadingTools || installing || loadingTools[dependencyName]}
                >
                  <SelectTrigger className="h-6 w-[70px] text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="auto">{t("common.auto")}</SelectItem>
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
                  value={wslShellByTool[dependencyName]?.wslShellFlag || "auto"}
                  onValueChange={(value) =>
                    handleToolShellFlagChange(dependencyName as ToolName, value)
                  }
                  disabled={isLoadingTools || installing || loadingTools[dependencyName]}
                >
                  <SelectTrigger className="h-6 w-[70px] text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="auto">{t("common.auto")}</SelectItem>
                    {WSL_SHELL_FLAG_OPTIONS.map((flag) => (
                      <SelectItem key={flag} value={flag}>
                        {flag}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}
            </div>
            {isCardInstalling || isLoadingTools ? (
              <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />
            ) : dependency?.state === "installed" ? (
              <CheckCircle2
                data-testid={`local-env-status-icon-${dependencyName}`}
                className="h-4 w-4 text-green-500"
              />
            ) : dependency?.state ? (
              <Badge
                className={`capitalize ${DEPENDENCY_STATE_CLASS_NAME[dependency.state]}`}
              >
                {t(DEPENDENCY_STATE_LABEL_KEY[dependency.state], {
                  defaultValue: dependency.state,
                })}
              </Badge>
            ) : (
              <AlertCircle className="h-4 w-4 text-yellow-500" />
            )}
          </div>
        }
        footer={
          <div className="flex h-full min-h-0 flex-col gap-2 overflow-hidden">
            <div className="flex min-h-0 items-start justify-between gap-2 overflow-hidden">
              <div className="min-w-0 flex-1 overflow-hidden">
                <div
                  className="truncate text-xs font-mono text-muted-foreground"
                  title={detailTitle}
                >
                  {detailValue}
                </div>
                <p
                  title={message ?? undefined}
                  className={`mt-0.5 min-h-[1rem] truncate text-[11px] leading-4 text-muted-foreground ${
                    message ? "" : "invisible"
                  }`}
                >
                  {message ?? "placeholder"}
                </p>
              </div>
              {!isLoadingTools && dependency?.path ? (
                <div
                  data-testid={`local-env-path-${dependencyName}`}
                  className="min-w-0 max-w-[58%] truncate text-right text-[11px] text-muted-foreground/90"
                  title={dependency.path}
                >
                  {displayPath}
                </div>
              ) : null}
            </div>
            <div
              data-testid={`local-env-actions-${dependencyName}`}
              className="flex min-h-7 items-center"
            >
              <div
                data-testid={`local-env-action-group-${dependencyName}`}
                className="ml-auto flex min-w-0 items-center gap-2"
              >
                {!isLoadingTools && isSelectable ? (
                  <Checkbox
                    aria-label={t("settings.selectDependency", {
                      defaultValue: `Select ${dependencyName}`,
                      name: dependencyName,
                    })}
                    checked={selectedDependencies.includes(dependencyName)}
                    onCheckedChange={(checked) =>
                      handleToggleSelected(dependencyName, checked === true)
                    }
                    disabled={installing}
                  />
                ) : null}
              </div>
            </div>
          </div>
        }
      />
    );
  };

  return (
    <motion.section
      initial={{ opacity: 0, y: 10 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.3 }}
      className="space-y-6"
    >
      <header className="space-y-1">
        <h3 className="text-sm font-medium">{t("common.about")}</h3>
        <p className="text-xs text-muted-foreground">{t("settings.aboutHint")}</p>
      </header>

      <div className={FEATURE_CARD_GRID_CLASS_NAME}>
        <motion.div
          initial={{ opacity: 0, scale: 0.98 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ duration: 0.3, delay: 0.1 }}
          className={`${FEATURE_CARD_SPAN_CLASS_NAME} rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-6 space-y-5 shadow-sm`}
        >
          <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                <img src={appIcon} alt="DPCC-SWITCH" className="h-5 w-5" />
                <h4 className="text-lg font-semibold text-foreground">DPCC-SWITCH</h4>
              </div>
              <div className="flex items-center gap-2">
                <Badge variant="outline" className="gap-1.5 bg-background/80">
                  <span className="text-muted-foreground">{t("common.version")}</span>
                  {isLoadingVersion ? (
                    <Loader2 className="h-3 w-3 animate-spin" />
                  ) : (
                    <span className="font-medium">{`v${displayVersion}`}</span>
                  )}
                </Badge>
                {isPortable && APP_UPDATES_ENABLED ? (
                  <Badge variant="secondary" className="gap-1.5">
                    <Info className="h-3 w-3" />
                    {t("settings.portableMode")}
                  </Badge>
                ) : null}
              </div>
            </div>

          <div className="space-y-1 text-xs text-muted-foreground sm:text-right">
            <div>© SunJiaHao</div>
            <div>如有问题发送至sjh2329952249@163.com</div>
          </div>
        </div>

          {APP_UPDATES_ENABLED && hasUpdate && updateInfo ? (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              className="rounded-lg border border-primary/20 bg-primary/10 px-4 py-3 text-sm"
            >
              <p className="mb-1 font-medium text-primary">
                {t("settings.updateAvailable", {
                  version: updateInfo.availableVersion,
                })}
              </p>
              {updateInfo.notes ? (
                <p className="line-clamp-3 leading-relaxed text-muted-foreground">
                  {updateInfo.notes}
                </p>
              ) : null}
            </motion.div>
          ) : null}
        </motion.div>
      </div>

      {!isWindows() ? (
        <div className="space-y-4">
          <div className="flex flex-col gap-3 px-1 sm:flex-row sm:items-center sm:justify-between">
            <h3 className="text-sm font-medium">{t("settings.localEnvCheck")}</h3>
            <div className="flex flex-wrap gap-2">
              <Button
                size="sm"
                variant="outline"
                className="h-7 gap-1.5 text-xs"
                onClick={() => void loadInstallerState()}
                disabled={isLoadingTools || installing}
              >
                <RefreshCw
                  className={
                    isLoadingTools ? "h-3.5 w-3.5 animate-spin" : "h-3.5 w-3.5"
                  }
                />
                {isLoadingTools ? t("common.refreshing") : t("common.refresh")}
              </Button>
              <Button
                size="sm"
                className="h-7 gap-1.5 text-xs"
                onClick={() => void handleInstallAll()}
                disabled={installableDependencies.length === 0 || installing}
              >
                {installing ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : null}
                {t("settings.installAllMissingDependencies", {
                  defaultValue: "Install All Missing Dependencies",
                })}
              </Button>
              <Button
                size="sm"
                variant="secondary"
                className="h-7 gap-1.5 text-xs"
                onClick={() => void handleInstallSelected()}
                disabled={selectedDependencies.length === 0 || installing}
              >
                {installing ? (
                  <Loader2 className="h-3.5 w-3.5 animate-spin" />
                ) : null}
                {installSelectedLabel}
              </Button>
              <Button
                size="sm"
                variant="ghost"
                className="h-7 text-xs"
                onClick={() => setShowManualCommands((current) => !current)}
              >
                {showManualCommands
                  ? t("settings.inlineManualCommandsHidden", {
                      defaultValue: "Hide Manual Commands",
                    })
                  : t("settings.inlineManualCommands", {
                      defaultValue: "Manual Commands",
                    })}
              </Button>
            </div>
          </div>

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
              {CORE_DEPENDENCY_NAMES.map((name, index) =>
                renderDependencyCard(name, index),
              )}
            </div>
          </div>

          <div className="space-y-3 px-1">
            <h4 className="text-xs font-semibold uppercase tracking-[0.12em] text-muted-foreground">
              {t("settings.installerCenterToolDependencies", {
                defaultValue: "CLI Tools",
              })}
            </h4>
            <div
              data-testid="local-env-tool-grid"
              className={LOCAL_ENV_GRID_CLASS_NAME}
            >
              {TOOL_NAMES.map((name, index) =>
                renderDependencyCard(name, CORE_DEPENDENCY_NAMES.length + index),
              )}
            </div>
          </div>

          {showManualCommands ? (
            <div className="space-y-3 px-1">
              <h4 className="text-xs font-semibold uppercase tracking-[0.12em] text-muted-foreground">
                {t("settings.inlineManualCommands", {
                  defaultValue: "Manual Commands",
                })}
              </h4>
              <div className="grid gap-4 md:grid-cols-2">
                {manualCommands.map((group) => (
                  <div
                    key={group.name}
                    className="rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-4 shadow-sm"
                  >
                    <p className="mb-3 text-sm font-semibold">{group.title}</p>
                    <div className="space-y-2">
                      {group.commands.map((command) => (
                        <pre
                          key={command}
                          className="overflow-x-auto rounded-md bg-muted/70 p-3 text-xs text-foreground"
                        >
                          <code>{command}</code>
                        </pre>
                      ))}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          ) : null}

          <div className={FEATURE_CARD_GRID_CLASS_NAME}>
            <div className={FEATURE_CARD_SPAN_CLASS_NAME}>
              <InstallerProgressPanel steps={progress} />
            </div>
          </div>
        </div>
      ) : null}
    </motion.section>
  );
}
