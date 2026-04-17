import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { AboutSection } from "@/components/settings/AboutSection";

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
        "settings.installerCenter.summaryTitle": "Environment Summary",
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
    getToolVersions: vi.fn().mockResolvedValue([]),
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
      ],
      lastCheckedAt: "2026-04-16T00:00:00Z",
      readyCount: 0,
      totalCount: 1,
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

test("hides update actions when app updates are disabled", () => {
  render(<AboutSection isPortable={false} />);

  expect(
    screen.queryByRole("button", { name: /release notes/i }),
  ).not.toBeInTheDocument();
  expect(
    screen.queryByRole("button", { name: /check for updates/i }),
  ).not.toBeInTheDocument();
});
