import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { defaultAdvancedFields } from "@/lib/quickStart/buildProvider";
import type { QuickStartFormFields } from "@/lib/quickStart/types";
import { QuickStartCustomFields } from "./QuickStartCustomFields";

describe("QuickStartCustomFields", () => {
  it("keeps the basic model and Claude role mapping synchronized", () => {
    const onChange = vi.fn();
    const selection = { mode: "custom", appId: "claude" } as const;
    const fields: QuickStartFormFields = {
      apiKey: "",
      customName: "",
      customBaseUrl: "",
      customModel: "deepseek-v4-pro",
      ...defaultAdvancedFields("claude", selection),
    };

    render(
      <QuickStartCustomFields
        appId="claude"
        fields={fields}
        onChange={onChange}
      />,
    );
    fireEvent.change(screen.getByDisplayValue("deepseek-v4-pro"), {
      target: { value: "custom-main-model" },
    });

    expect(onChange).toHaveBeenCalledWith({
      customModel: "custom-main-model",
      advancedClaude: expect.objectContaining({
        haikuModel: "custom-main-model",
        sonnetModel: "custom-main-model",
        opusModel: "custom-main-model",
      }),
    });
  });

  it("preserves role models that the user has already customized", () => {
    const onChange = vi.fn();
    const selection = { mode: "custom", appId: "claude" } as const;
    const initialized = defaultAdvancedFields("claude", selection);
    const fields: QuickStartFormFields = {
      ...initialized,
      apiKey: "",
      customName: "",
      customBaseUrl: "",
      customModel: "base-model",
      advancedClaude: {
        ...initialized.advancedClaude!,
        haikuModel: "base-model",
        haikuModelName: "base-model",
        sonnetModel: "base-model",
        sonnetModelName: "base-model",
        opusModel: "manually-selected-strong-model",
        opusModelName: "Strong",
        fableModel: "base-model",
        fableModelName: "base-model",
        fallbackModel: "base-model",
      },
    };

    render(
      <QuickStartCustomFields
        appId="claude"
        fields={fields}
        onChange={onChange}
      />,
    );
    fireEvent.change(screen.getByDisplayValue("base-model"), {
      target: { value: "new-base-model" },
    });

    expect(onChange).toHaveBeenCalledWith({
      customModel: "new-base-model",
      advancedClaude: expect.objectContaining({
        sonnetModel: "new-base-model",
        opusModel: "manually-selected-strong-model",
        opusModelName: "Strong",
      }),
    });
  });
});
