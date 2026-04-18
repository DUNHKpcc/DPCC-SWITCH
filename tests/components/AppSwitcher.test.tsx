import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { AppSwitcher } from "@/components/AppSwitcher";

describe("AppSwitcher", () => {
  it("shows only icons even when fewer than five apps are visible", () => {
    const onSwitch = vi.fn();

    render(
      <AppSwitcher
        activeApp="claude"
        onSwitch={onSwitch}
        visibleApps={{
          claude: true,
          codex: true,
          gemini: false,
          opencode: true,
          openclaw: false,
        }}
        compact={false}
      />,
    );

    const claudeButton = screen.getByRole("button", { name: "Claude" });
    const codexButton = screen.getByRole("button", { name: "Codex" });
    const openCodeButton = screen.getByRole("button", { name: "OpenCode" });

    expect(claudeButton.children).toHaveLength(1);
    expect(codexButton.children).toHaveLength(1);
    expect(openCodeButton.children).toHaveLength(1);
  });
});
