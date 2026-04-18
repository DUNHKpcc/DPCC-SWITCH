import { render, screen, fireEvent } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useForm } from "react-hook-form";

import { BasicFormFields } from "@/components/providers/forms/BasicFormFields";
import { Form } from "@/components/ui/form";
import type { ProviderFormData } from "@/lib/schemas/provider";

vi.mock("@/components/IconPicker", () => ({
  IconPicker: () => <div data-testid="icon-picker">icon-picker</div>,
}));

vi.mock("@/components/common/FullScreenPanel", () => ({
  FullScreenPanel: ({
    isOpen,
    title,
    children,
    footer,
  }: {
    isOpen: boolean;
    title: string;
    children: React.ReactNode;
    footer?: React.ReactNode;
  }) =>
    isOpen ? (
      <div data-testid="fullscreen-panel">
        <div>{title}</div>
        {children}
        {footer}
      </div>
    ) : null,
}));

function TestBasicFormFields() {
  const form = useForm<ProviderFormData>({
    defaultValues: {
      name: "Provider",
      notes: "",
      websiteUrl: "",
      settingsConfig: "{}",
      icon: "",
      iconColor: "",
    },
  });

  return (
    <Form {...form}>
      <BasicFormFields form={form} />
    </Form>
  );
}

describe("BasicFormFields", () => {
  it("uses FullScreenPanel for the icon picker", () => {
    render(<TestBasicFormFields />);

    fireEvent.click(screen.getAllByRole("button")[0]);

    expect(screen.getByTestId("fullscreen-panel")).toBeInTheDocument();
    expect(screen.getByTestId("icon-picker")).toBeInTheDocument();
  });
});
