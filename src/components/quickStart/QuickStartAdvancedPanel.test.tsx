import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { defaultAdvancedFields } from "@/lib/quickStart/buildProvider";
import type { QuickStartFormFields } from "@/lib/quickStart/types";
import { QuickStartAdvancedPanel } from "./QuickStartAdvancedPanel";

const baseFields: QuickStartFormFields = {
  apiKey: "",
  customName: "",
  customBaseUrl: "",
  customModel: "deepseek-v4-pro",
};

describe("QuickStartAdvancedPanel", () => {
  it("is collapsed by default and exposes custom Claude protocol, auth, and role models", () => {
    const selection = { mode: "custom", appId: "claude" } as const;
    const fields = {
      ...baseFields,
      ...defaultAdvancedFields("claude", selection),
    };

    render(
      <QuickStartAdvancedPanel
        appId="claude"
        selection={selection}
        fields={fields}
        onChange={vi.fn()}
      />,
    );

    expect(screen.queryByLabelText("API 格式")).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /高级选项/ }));
    expect(screen.getByLabelText("API 格式")).toBeInTheDocument();
    expect(screen.getByLabelText("认证字段")).toBeInTheDocument();
    expect(screen.getByText("Sonnet")).toBeInTheDocument();
    expect(screen.getByText("Opus")).toBeInTheDocument();
    expect(screen.getByText("Fable")).toBeInTheDocument();
    expect(screen.getByText("Haiku")).toBeInTheDocument();
    expect(screen.getByText("Subagent")).toBeInTheDocument();
    expect(screen.getByText("默认兜底模型")).toBeInTheDocument();
  });

  it("exposes Codex upstream format and model after expansion", () => {
    const selection = { mode: "custom", appId: "codex" } as const;
    const fields = {
      ...baseFields,
      ...defaultAdvancedFields("codex", selection),
    };

    render(
      <QuickStartAdvancedPanel
        appId="codex"
        selection={selection}
        fields={fields}
        onChange={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /高级选项/ }));
    expect(screen.getByLabelText("API 格式")).toBeInTheDocument();
    expect(screen.getByDisplayValue("gpt-5.6-sol")).toBeInTheDocument();
  });

  it("exposes Gemini protocol, endpoint, and model after expansion", () => {
    const selection = { mode: "custom", appId: "gemini" } as const;
    const fields = {
      ...baseFields,
      ...defaultAdvancedFields("gemini", selection),
    };

    render(
      <QuickStartAdvancedPanel
        appId="gemini"
        selection={selection}
        fields={fields}
        onChange={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /高级选项/ }));
    expect(screen.getByLabelText("API 格式")).toBeInTheDocument();
    expect(screen.getByText("Gemini Native")).toBeInTheDocument();
    expect(screen.getByDisplayValue("gemini-3.6-flash")).toBeInTheDocument();
  });

  it("locks a preset protocol instead of exposing the four-format selector", () => {
    const selection = {
      mode: "preset",
      appId: "claude-desktop",
      presetName: "Kimi",
      isOfficial: false,
    } as const;
    const fields = {
      ...baseFields,
      ...defaultAdvancedFields("claude-desktop", selection),
    };

    render(
      <QuickStartAdvancedPanel
        appId="claude-desktop"
        selection={selection}
        fields={fields}
        onChange={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: /高级选项/ }));
    expect(
      screen.queryByRole("combobox", { name: "API 格式" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "API 格式" })).toHaveTextContent(
      "Anthropic Messages",
    );
    expect(screen.getByText(/预设已按真实端点锁定协议/)).toBeInTheDocument();
    expect(screen.getByLabelText("Sonnet 实际请求模型")).toHaveValue(
      "kimi-k2.7-code",
    );
  });
});
