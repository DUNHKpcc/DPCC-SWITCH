import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { UpdateProvider, useUpdate } from "@/contexts/UpdateContext";

const checkForUpdateMock = vi.fn();

vi.mock("@/lib/updater", () => ({
  checkForUpdate: (...args: unknown[]) => checkForUpdateMock(...args),
}));

function UpdateProbe() {
  const { hasUpdate } = useUpdate();
  return <div data-testid="has-update">{String(hasUpdate)}</div>;
}

describe("UpdateProvider", () => {
  beforeEach(() => {
    checkForUpdateMock.mockReset();
    checkForUpdateMock.mockResolvedValue({ status: "up-to-date" });
    localStorage.clear();
    vi.useFakeTimers();
  });

  it("does not check updates on mount when app updates are disabled", async () => {
    render(
      <UpdateProvider>
        <UpdateProbe />
      </UpdateProvider>,
    );

    await act(async () => {
      vi.advanceTimersByTime(1500);
    });

    expect(screen.getByTestId("has-update")).toHaveTextContent("false");
    expect(checkForUpdateMock).not.toHaveBeenCalled();
  });
});
