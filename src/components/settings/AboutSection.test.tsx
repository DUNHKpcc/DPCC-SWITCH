import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { AboutSection } from "@/components/settings/AboutSection";
import { installerApi } from "@/lib/api/installer";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      const translations: Record<string, string> = {
        "common.about": "About",
        "common.version": "Version",
        "common.unknown": "Unknown",
        "settings.aboutHint": "View version information and update status.",
        "settings.releaseNotes": "Release Notes",
        "settings.checkForUpdates": "Check for Updates",
        "settings.localEnvCheck": "Environment Check",
        "settings.installerCenter": "Environment Check & Install",
        "settings.openInstallerCenter": "Environment Check & Install",
        "settings.installerCenterTitle": "Environment Check & Install",
        "settings.installerCenterSummaryTitle": "Environment Summary",
        "settings.installerCenterCoreDependencies": "Core Dependencies",
        "settings.installerDependencyKind.core": "core",
        "settings.installerDependencyState.installed": "installed",
        "settings.installerDependencyState.missing": "missing",
        "common.refresh": "Refresh",
        "common.refreshing": "Refreshing...",
      };

      if (key === "settings.updateTo") {
        return `Update to v${options?.version ?? ""}`;
      }

      if (key === "settings.updateAvailable") {
        return `New version available: ${options?.version ?? ""}`;
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
    ]),
    openExternal: vi.fn().mockResolvedValue(undefined),
    checkUpdates: vi.fn().mockResolvedValue(undefined),
  },
}));

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
        {
          name: "npm",
          kind: "core",
          state: "installed",
          version: "11.0.0",
          path: "/usr/local/bin/npm",
          message: null,
          autoInstallSupported: false,
        },
        {
          name: "pnpm",
          kind: "core",
          state: "installed",
          version: "10.1.0",
          path: "/usr/local/bin/pnpm",
          message: null,
          autoInstallSupported: false,
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
      ],
      lastCheckedAt: "2026-04-16T00:00:00Z",
      readyCount: 3,
      totalCount: 4,
    }),
    installMissing: vi.fn(),
    getManualCommands: vi.fn().mockResolvedValue([
      {
        name: "node",
        title: "Node.js",
        commands: ["Install Node.js with your package manager or nvm."],
      },
    ]),
    subscribeProgress: vi.fn().mockResolvedValue(() => {}),
  },
}));

test("opens installer center from about section", async () => {
  const user = userEvent.setup();

  render(<AboutSection isPortable={false} />);

  await user.click(
    await screen.findByRole("button", { name: /environment check & install/i }),
  );

  expect(await screen.findByText(/environment summary/i)).toBeInTheDocument();
});

test("shows core dependency cards inside local environment check", async () => {
  render(<AboutSection isPortable={false} />);

  expect(await screen.findByText("Core Dependencies")).toBeInTheDocument();
  expect(await screen.findByText("node")).toBeInTheDocument();
  expect(screen.getByText("npm")).toBeInTheDocument();
  expect(screen.getByText("pnpm")).toBeInTheDocument();
  expect(screen.getByText("git")).toBeInTheDocument();
  expect(screen.getByText("missing")).toBeInTheDocument();
});

test("reuses the same local environment card shell for core dependencies and tools", async () => {
  render(<AboutSection isPortable={false} />);

  const nodeCard = await screen.findByTestId("local-env-card-node");
  const claudeCard = await screen.findByTestId("local-env-card-claude");
  const coreGrid = screen.getByTestId("local-env-core-grid");
  const toolGrid = screen.getByTestId("local-env-tool-grid");

  for (const card of [nodeCard, claudeCard]) {
    expect(card.className).toContain("rounded-xl");
    expect(card.className).toContain("from-card/80");
    expect(card.className).toContain("to-card/40");
    expect(card.className).toContain("hover:border-primary/30");
  }

  expect(coreGrid.className).toContain("sm:grid-cols-2");
  expect(coreGrid.className).toContain("lg:grid-cols-4");
  expect(toolGrid.className).toContain("sm:grid-cols-2");
  expect(toolGrid.className).toContain("lg:grid-cols-4");
});

test("renders core cards with the same height shell as tool cards and right-aligned path metadata", async () => {
  render(<AboutSection isPortable={false} />);

  const npmCard = await screen.findByTestId("local-env-card-npm");
  const claudeCard = await screen.findByTestId("local-env-card-claude");
  const pnpmPath = screen.getByTestId("local-env-path-pnpm");

  expect(npmCard.className).toContain("p-4");
  expect(npmCard.className).toContain("gap-2");
  expect(claudeCard.className).toContain("p-4");
  expect(claudeCard.className).toContain("gap-2");
  expect(pnpmPath.className).toContain("text-right");
  expect(pnpmPath.textContent).toContain("/usr/local/bin/pnpm");
});

test("shows a check icon for installed core dependencies", async () => {
  vi.mocked(installerApi.detectEnvironment).mockResolvedValueOnce({
    platform: "linux",
    autoInstallSupported: false,
    dependencies: [
      {
        name: "node",
        kind: "core",
        state: "installed",
        version: "22.1.0",
        path: "/usr/local/bin/node",
        message: null,
        autoInstallSupported: false,
      },
    ],
    lastCheckedAt: "2026-04-16T00:00:00Z",
    readyCount: 1,
    totalCount: 1,
  });

  render(<AboutSection isPortable={false} />);

  expect(await screen.findByTestId("local-env-status-icon-node")).toBeInTheDocument();
  expect(screen.queryByText("installed")).not.toBeInTheDocument();
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
