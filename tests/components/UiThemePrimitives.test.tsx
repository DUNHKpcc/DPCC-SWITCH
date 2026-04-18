import { render, screen } from "@testing-library/react";
import { describe, expect, test } from "vitest";

import { buttonVariants } from "@/components/ui/button";
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs";

describe("monochrome theme primitives", () => {
  test("default button variant does not use blue accent classes", () => {
    const className = buttonVariants({ variant: "default" });

    expect(className).not.toContain("blue-");
  });

  test("active tabs do not use blue accent classes", () => {
    render(
      <Tabs defaultValue="general">
        <TabsList>
          <TabsTrigger value="general">General</TabsTrigger>
          <TabsTrigger value="about">About</TabsTrigger>
        </TabsList>
      </Tabs>,
    );

    const activeTab = screen.getByRole("tab", { name: "General" });

    expect(activeTab.className).not.toContain("blue-");
  });
});
