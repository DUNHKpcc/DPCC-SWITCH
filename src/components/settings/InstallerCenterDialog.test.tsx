import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { vi } from "vitest";

import { InstallerCenterDialog } from "@/components/settings/InstallerCenterDialog";

vi.mock("react-i18next", () => ({
  useTranslation: () => ({
    t: (key: string, options?: Record<string, unknown>) => {
      const translations: Record<string, string> = {
        "common.close": "关闭",
        "common.refresh": "刷新",
        "common.refreshing": "刷新中...",
        "common.unknown": "未知",
        "settings.installerCenterTitle": "环境检测与安装",
        "settings.installerCenterDescription":
          "检测 CLI 依赖、安装缺失工具，并查看手动安装命令。",
        "settings.installerCenterSummaryTitle": "环境摘要",
        "settings.installerCenterLoading": "正在检测安装环境...",
        "settings.installerCenterCoreDependencies": "核心依赖",
        "settings.installerCenterToolDependencies": "CLI 工具",
        "settings.installerCenterManualCommands": "手动命令",
        "settings.installerCenterInstallMissing": "安装缺失项",
        "settings.installerCenterShowManualCommands": "手动命令",
        "settings.installerCenterHideManualCommands": "隐藏手动命令",
        "settings.installerCenterCloseAriaLabel": "关闭环境检测与安装",
        "settings.installerDependencyKind.core": "核心",
        "settings.installerDependencyKind.tool": "工具",
        "settings.installerDependencyState.installed": "已安装",
        "settings.installerDependencyState.missing": "缺失",
        "settings.installerDependencyState.outdated": "过旧",
        "settings.installerDependencyState.broken": "异常",
        "settings.installerDependencyState.manual": "手动处理",
        "settings.installerVersionUnavailable": "版本未知",
        "settings.installerProgressTitle": "安装进度",
        "settings.installerProgressEmpty": "暂无安装活动。",
        "settings.envBadge.linux": "Linux",
      };

      if (key === "settings.installerCenterReadyCount") {
        return `${options?.readyCount ?? 0}/${options?.totalCount ?? 0} 项依赖已就绪`;
      }

      return translations[key] ?? key;
    },
  }),
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

test("loads environment status when opened", async () => {
  render(<InstallerCenterDialog open onOpenChange={() => {}} />);

  await waitFor(() => {
    expect(screen.getByText("环境摘要")).toBeInTheDocument();
    expect(screen.getByText("CLI 工具")).toBeInTheDocument();
  });
});

test("shows manual commands on linux", async () => {
  const user = userEvent.setup();

  render(<InstallerCenterDialog open onOpenChange={() => {}} />);

  await user.click(await screen.findByRole("button", { name: "手动命令" }));

  expect(
    await screen.findByText(/package manager or nvm/i),
  ).toBeInTheDocument();
});

test("adds extra bottom padding to the scroll area so the last card is fully reachable", async () => {
  render(<InstallerCenterDialog open onOpenChange={() => {}} />);

  const scrollArea = await screen.findByTestId("installer-center-scroll");

  expect(scrollArea.className).toContain("pb-10");
});

test("provides an explicit close action so users can return from the installer center", async () => {
  const user = userEvent.setup();
  const onOpenChange = vi.fn();

  render(<InstallerCenterDialog open onOpenChange={onOpenChange} />);

  await user.click(
    await screen.findByRole("button", { name: "关闭环境检测与安装" }),
  );

  expect(onOpenChange).toHaveBeenCalledWith(false);
});

test("renders above the settings header and stays within the viewport on smaller windows", async () => {
  render(<InstallerCenterDialog open onOpenChange={() => {}} />);

  const dialog = await screen.findByRole("dialog");

  expect(dialog.className).toContain("z-50");
  expect(dialog.className).toContain("w-[min(96vw,80rem)]");
});

test("renders installer center chrome in chinese", async () => {
  render(<InstallerCenterDialog open onOpenChange={() => {}} />);

  expect(await screen.findByText("环境检测与安装")).toBeInTheDocument();
  expect(screen.getByText("环境摘要")).toBeInTheDocument();
  expect(screen.getByRole("button", { name: "刷新" })).toBeInTheDocument();
  expect(
    screen.getByRole("button", { name: "安装缺失项" }),
  ).toBeInTheDocument();
  expect(screen.getByText("0/1 项依赖已就绪")).toBeInTheDocument();
  expect(screen.getByText("CLI 工具")).toBeInTheDocument();
  expect(screen.getByText("安装进度")).toBeInTheDocument();
});

test("does not render core dependency cards in installer center", async () => {
  render(<InstallerCenterDialog open onOpenChange={() => {}} />);

  expect(screen.queryByText("核心依赖")).not.toBeInTheDocument();
  expect(screen.queryByText("node")).not.toBeInTheDocument();
});
