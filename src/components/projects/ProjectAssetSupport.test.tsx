import { describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { ProjectAssetEnableSwitch } from "@/components/projects/ProjectAssetSupport";
import { renderWithProviders } from "../../../tests/renderWithProviders";

describe("ProjectAssetEnableSwitch", () => {
  it("明确告诉用户开关控制的是本项目关联", async () => {
    const user = userEvent.setup();
    const onCheckedChange = vi.fn();

    renderWithProviders(
      <ProjectAssetEnableSwitch
        assetType="mcp"
        checked={false}
        onCheckedChange={onCheckedChange}
      />,
    );

    expect(screen.getByText("本项目关联")).toBeInTheDocument();
    const projectSwitch = screen.getByRole("switch", {
      name: "本项目关联",
    });
    await user.click(projectSwitch);
    expect(onCheckedChange).toHaveBeenCalledWith(true);
  });
});
