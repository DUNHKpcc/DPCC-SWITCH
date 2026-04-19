import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { AboutSection } from "@/components/settings/AboutSection";
import { installerApi } from "@/lib/api/installer";
import type { InstallerEnvironment } from "@/types/installer";

const { mockEnvironmentState } = vi.hoisted(() => ({
  mockEnvironmentState: {
    platform: "macos",
    autoInstallSupported: true,
    dependencies: [
      {
        name: "node",
        kind: "core",
        state: "installed",
        version: "v22.1.0",
        path: "/usr/local/bin/node",
        message: null,
        autoInstallSupported: true,
      },
      {
        name: "npm",
        kind: "core",
        state: "installed",
        version: "10.9.0",
        path: "/usr/local/bin/npm",
        message: null,
        autoInstallSupported: true,
      },
      {
        name: "pnpm",
        kind: "core",
        state: "installed",
        version: "9.0.0",
        path: "/usr/local/bin/pnpm",
        message: null,
        autoInstallSupported: true,
      },
      {
        name: "git",
        kind: "core",
        state: "installed",
        version: "2.49.0",
        path: "/usr/bin/git",
        message: null,
        autoInstallSupported: false,
      },
      {
        name: "claude",
        kind: "tool",
        state: "installed",
        version: "1.0.0",
        path: "/usr/local/bin/claude",
        message: null,
        autoInstallSupported: true,
      },
      {
        name: "codex",
        kind: "tool",
        state: "missing",
        version: null,
        path: null,
        message: "codex was not found on PATH.",
        autoInstallSupported: true,
      },
      {
        name: "gemini",
        kind: "tool",
        state: "missing",
        version: null,
        path: null,
        message: "gemini was not found on PATH.",
        autoInstallSupported: true,
      },
      {
        name: "opencode",
        kind: "tool",
        state: "manual",
        version: null,
        path: null,
        message: "OpenCode requires manual install on this platform.",
        autoInstallSupported: false,
      },
    ],
    lastCheckedAt: "2026-04-18T00:00:00Z",
    readyCount: 5,
    totalCount: 8,
  } as InstallerEnvironment,
}));

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      const translations: Record<string, string> = {
        "common.about": "About",
        "common.auto": "Auto",
        "common.loading": "Loading",
        "common.notInstalled": "Not installed",
        "common.refresh": "Refresh Detection",
        "common.refreshing": "Refreshing...",
        "common.unknown": "Unknown",
        "common.version": "Version",
        "settings.aboutHint": "View version information and update status.",
        "settings.checkForUpdates": "Check for Updates",
        "settings.installAllMissingDependencies":
          "Install All Missing Dependencies",
        "settings.installSelectedDependencies":
          "Install Selected Dependencies",
        "settings.inlineManualCommands": "Manual Commands",
        "settings.inlineManualCommandsHidden": "Hide Manual Commands",
        "settings.installerCenterCoreDependencies": "Core Dependencies",
        "settings.installerCenterToolDependencies": "CLI Tools",
        "settings.installerDependencyKind.core": "core",
        "settings.installerDependencyKind.tool": "tool",
        "settings.installerDependencyState.broken": "broken",
        "settings.installerDependencyState.installed": "installed",
        "settings.installerDependencyState.manual": "manual",
        "settings.installerDependencyState.missing": "missing",
        "settings.installerDependencyState.outdated": "outdated",
        "settings.installerProgressEmpty": "No install activity yet.",
        "settings.installerProgressTitle": "Install Progress",
        "settings.installerVersionUnavailable": "Version unavailable",
        "settings.localEnvCheck": "Environment Check",
        "settings.manualInstall": "Manual Install",
        "settings.openInstallerCenter": "Environment Check & Install",
      };

      if (key === "settings.updateTo") {
        return `Update to v${options?.version ?? ""}`;
      }

      if (key === "settings.updateAvailable") {
        return `New version available: ${options?.version ?? ""}`;
      }

      if (key === "settings.selectDependency") {
        return `Select ${options?.name ?? ""}`;
      }

      if (key === "settings.installDependency") {
        return `Install ${options?.name ?? ""}`;
      }

      if (key === "settings.manualInstallDependency") {
        return `Manual install ${options?.name ?? ""}`;
      }

      if (key === "settings.installSelectedDependenciesCount") {
        return `Install Selected Dependencies (${options?.count ?? 0})`;
      }

      if (key === "settings.installNodeIncludesNpm") {
        return "Install Node.js (includes npm)";
      }

      return translations[key] ?? key;
    },
  }),
}));

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: vi.fn().mockResolvedValue("3.13.0"),
}));

vi.mock("@/contexts/UpdateContext", () => ({
  useUpdate: () => ({
    hasUpdate: false,
    updateInfo: null,
    updateHandle: null,
    checkUpdate: vi.fn().mockResolvedValue(false),
    resetDismiss: vi.fn(),
    isChecking: false,
  }),
}));

vi.mock("@/lib/updater", () => ({
  relaunchApp: vi.fn(),
}));

vi.mock("@/lib/api", () => ({
  settingsApi: {
    getToolVersions: vi.fn().mockResolvedValue([
      {
        name: "claude",
        version: "1.0.0",
        latest_version: "1.0.0",
        error: null,
        env_type: "linux",
        wsl_distro: null,
      },
      {
        name: "codex",
        version: null,
        latest_version: null,
        error: "codex was not found on PATH.",
        env_type: "linux",
        wsl_distro: null,
      },
      {
        name: "gemini",
        version: null,
        latest_version: null,
        error: "gemini was not found on PATH.",
        env_type: "linux",
        wsl_distro: null,
      },
      {
        name: "opencode",
        version: null,
        latest_version: null,
        error: "opencode was not found on PATH.",
        env_type: "linux",
        wsl_distro: null,
      },
    ]),
    openExternal: vi.fn().mockResolvedValue(undefined),
    checkUpdates: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock("@/lib/api/installer", () => ({
  installerApi: {
    detectEnvironment: vi.fn().mockResolvedValue(mockEnvironmentState),
    installMissing: vi.fn().mockResolvedValue({
      steps: [],
      completedDependencies: [],
      failedDependencies: [],
      manualDependencies: [],
      statusMessage: "done",
    }),
    installSelected: vi.fn().mockResolvedValue({
      steps: [],
      completedDependencies: ["codex"],
      failedDependencies: [],
      manualDependencies: [],
      statusMessage: "done",
    }),
    getManualCommands: vi.fn().mockResolvedValue([
      {
        name: "node",
        title: "Node.js",
        commands: ["Install Node.js with your package manager or nvm."],
      },
      {
        name: "opencode",
        title: "OpenCode",
        commands: ["curl -fsSL https://opencode.ai/install | bash"],
      },
    ]),
    subscribeProgress: vi.fn().mockResolvedValue(() => {}),
  },
}));

test("shows dependency cards immediately while environment detection is still loading", () => {
  vi.mocked(installerApi.detectEnvironment).mockImplementationOnce(
    () => new Promise(() => {}),
  );

  render(<AboutSection isPortable={false} />);

  expect(screen.getByTestId("local-env-card-node")).toBeInTheDocument();
  expect(screen.getByTestId("local-env-card-codex")).toBeInTheDocument();
  expect(screen.getAllByText("Loading").length).toBeGreaterThan(0);
});

test("renders dedicated icons for core dependency cards", async () => {
  render(<AboutSection isPortable={false} />);

  expect(await screen.findByRole("img", { name: "node icon" })).toBeInTheDocument();
  expect(screen.getByRole("img", { name: "npm icon" })).toBeInTheDocument();
  expect(screen.getByRole("img", { name: "pnpm icon" })).toBeInTheDocument();
  expect(screen.getByRole("img", { name: "git icon" })).toBeInTheDocument();
});

test("displays distinguishable tool path suffixes for npm-based CLIs", async () => {
  vi.mocked(installerApi.detectEnvironment).mockResolvedValueOnce({
    ...mockEnvironmentState,
    dependencies: mockEnvironmentState.dependencies.map((dependency) =>
      dependency.name === "codex"
        ? {
            ...dependency,
            state: "installed",
            version: "0.42.0",
            path: "/Users/dpccskisw/.nvm/versions/node/v25.7.0/bin/codex",
            message: null,
          }
        : dependency,
    ),
  });

  render(<AboutSection isPortable={false} />);

  const codexPath = await screen.findByTestId("local-env-path-codex");

  expect(codexPath).toHaveTextContent(".../bin/codex");
  expect(codexPath).toHaveAttribute(
    "title",
    "/Users/dpccskisw/.nvm/versions/node/v25.7.0/bin/codex",
  );
});

test("keeps dependency cards at the initial loading height after detection completes", async () => {
  let resolveEnvironment: ((value: typeof mockEnvironmentState) => void) | undefined;

  vi.mocked(installerApi.detectEnvironment).mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        resolveEnvironment = resolve;
      }),
  );

  render(<AboutSection isPortable={false} />);

  const nodeCard = screen.getByTestId("local-env-card-node");
  const initialClassName = nodeCard.className;

  expect(initialClassName).toContain("h-[8rem]");

  resolveEnvironment?.(mockEnvironmentState);

  expect(
    await screen.findByTestId("local-env-status-icon-node"),
  ).toBeInTheDocument();
  expect(screen.getByTestId("local-env-card-node").className).toBe(initialClassName);
});

test("adds horizontal gutter to dependency grids so hovered edge cards do not clip", async () => {
  render(<AboutSection isPortable={false} />);

  await screen.findByTestId("local-env-card-node");

  const coreGrid = screen.getByTestId("local-env-core-grid");
  const toolGrid = screen.getByTestId("local-env-tool-grid");

  expect(coreGrid.className).toContain("px-2");
  expect(toolGrid.className).toContain("px-2");
});

test("renders inline installer actions and removes the installer launcher button", async () => {
  render(<AboutSection isPortable={false} />);

  expect(screen.getByText("© SunJiaHao")).toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: "Check for Updates" }),
  ).not.toBeInTheDocument();
  expect(
    await screen.findByRole("button", {
      name: "Install All Missing Dependencies",
    }),
  ).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "Install Selected Dependencies" }),
  ).toBeDisabled();
  expect(
    screen.queryByRole("button", { name: "Environment Check & Install" }),
  ).not.toBeInTheDocument();
});

test("installs a selected missing dependency inline", async () => {
  const user = userEvent.setup();

  render(<AboutSection isPortable={false} />);

  await user.click(await screen.findByRole("checkbox", { name: "Select codex" }));
  await user.click(
    screen.getByRole("button", { name: "Install Selected Dependencies (1)" }),
  );

  expect(installerApi.installSelected).toHaveBeenCalledWith(["codex"]);
});

test("renders missing dependency detail only once and keeps only the checkbox in the right-aligned action group", async () => {
  render(<AboutSection isPortable={false} />);

  const codexMessage = await screen.findAllByText("codex was not found on PATH.");
  const actionRow = screen.getByTestId("local-env-actions-codex");
  const actionGroup = screen.getByTestId("local-env-action-group-codex");

  expect(codexMessage).toHaveLength(1);
  expect(actionRow.className).toContain("min-h-7");
  expect(actionGroup.className).toContain("ml-auto");
  expect(actionGroup.className).toContain("gap-2");
  expect(screen.queryByRole("button", { name: "Install codex" })).not.toBeInTheDocument();
  expect(screen.getByRole("checkbox", { name: "Select codex" })).toBeInTheDocument();
});

test("does not render a per-card install button for selectable missing dependencies", async () => {
  render(<AboutSection isPortable={false} />);

  expect(
    screen.queryByRole("button", { name: "Install codex" }),
  ).not.toBeInTheDocument();
});

test("does not offer outdated dependencies as selectable auto-installs", async () => {
  vi.mocked(installerApi.detectEnvironment).mockResolvedValueOnce({
    ...mockEnvironmentState,
    dependencies: mockEnvironmentState.dependencies.map((dependency) =>
      dependency.name === "codex"
        ? {
            ...dependency,
            state: "outdated",
            message: "codex is outdated.",
          }
        : dependency,
    ),
  });

  render(<AboutSection isPortable={false} />);

  expect(
    screen.queryByRole("checkbox", { name: "Select codex" }),
  ).not.toBeInTheDocument();
});

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

test("reveals manual install commands from the top action bar instead of a dependency card button", async () => {
  const user = userEvent.setup();

  render(<AboutSection isPortable={false} />);

  expect(
    screen.queryByRole("button", { name: "Manual install opencode" }),
  ).not.toBeInTheDocument();
  await user.click(
    await screen.findByRole("button", { name: "Manual Commands" }),
  );

  expect(
    await screen.findByText("curl -fsSL https://opencode.ai/install | bash"),
  ).toBeInTheDocument();
});

test("renders the mirrored Homebrew bootstrap commands for macOS node setup", async () => {
  const user = userEvent.setup();

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

  await user.click(
    await screen.findByRole("button", { name: "Manual Commands" }),
  );

  expect(
    await screen.findByText(
      "export HOMEBREW_BREW_GIT_REMOTE=https://mirrors.tuna.tsinghua.edu.cn/git/homebrew/brew.git",
    ),
  ).toBeInTheDocument();
  expect(screen.getByText("brew install node")).toBeInTheDocument();
});

test("installs all missing dependencies from the inline action bar", async () => {
  const user = userEvent.setup();

  render(<AboutSection isPortable={false} />);

  await user.click(
    await screen.findByRole("button", {
      name: "Install All Missing Dependencies",
    }),
  );

  expect(installerApi.installMissing).toHaveBeenCalled();
});

test("hides update actions when app updates are disabled", () => {
  render(<AboutSection isPortable={false} />);

  expect(
    screen.queryByRole("button", { name: /release notes/i }),
  ).not.toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: /check for updates/i }),
  ).not.toBeInTheDocument();
});
