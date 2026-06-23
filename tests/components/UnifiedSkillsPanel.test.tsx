import { createRef } from "react";
import { render, screen, waitFor, act } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";

import UnifiedSkillsPanel, {
  type UnifiedSkillsPanelHandle,
} from "@/components/skills/UnifiedSkillsPanel";

const skillsMocks = vi.hoisted(() => ({
  scanUnmanagedMock: vi.fn(),
  toggleSkillAppMock: vi.fn(),
  uninstallSkillMock: vi.fn(),
  importSkillsMock: vi.fn(),
  installFromZipMock: vi.fn(),
  deleteSkillBackupMock: vi.fn(),
  restoreSkillBackupMock: vi.fn(),
  batchToggleSkillAppMock: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    info: vi.fn(),
  },
}));

vi.mock("@/hooks/useSkills", async () => {
  const { createSkillsHooksMock } = await import("../mocks/useSkillsMocks");
  return createSkillsHooksMock({
    useDeleteSkillBackup: () => ({
      mutateAsync: skillsMocks.deleteSkillBackupMock,
      isPending: false,
    }),
    useToggleSkillApp: () => ({
      mutateAsync: skillsMocks.toggleSkillAppMock,
    }),
    useBatchToggleSkillApp: () => ({
      mutateAsync: skillsMocks.batchToggleSkillAppMock,
      isPending: false,
    }),
    useRestoreSkillBackup: () => ({
      mutateAsync: skillsMocks.restoreSkillBackupMock,
      isPending: false,
    }),
    useUninstallSkill: () => ({
      mutateAsync: skillsMocks.uninstallSkillMock,
    }),
    useScanUnmanagedSkills: () => ({
      data: [
        {
          directory: "shared-skill",
          name: "Shared Skill",
          description: "Imported from Claude",
          foundIn: ["claude"],
          path: "/tmp/shared-skill",
        },
      ],
      refetch: skillsMocks.scanUnmanagedMock,
    }),
    useImportSkillsFromApps: () => ({
      mutateAsync: skillsMocks.importSkillsMock,
    }),
    useInstallSkillsFromZip: () => ({
      mutateAsync: skillsMocks.installFromZipMock,
    }),
  });
});

describe("UnifiedSkillsPanel", () => {
  beforeEach(() => {
    skillsMocks.scanUnmanagedMock.mockResolvedValue({
      data: [
        {
          directory: "shared-skill",
          name: "Shared Skill",
          description: "Imported from Claude",
          foundIn: ["claude"],
          path: "/tmp/shared-skill",
        },
      ],
    });
    skillsMocks.toggleSkillAppMock.mockReset();
    skillsMocks.batchToggleSkillAppMock.mockReset();
    skillsMocks.uninstallSkillMock.mockReset();
    skillsMocks.importSkillsMock.mockReset();
    skillsMocks.installFromZipMock.mockReset();
    skillsMocks.deleteSkillBackupMock.mockReset();
    skillsMocks.restoreSkillBackupMock.mockReset();
  });

  it("opens the import dialog without crashing when app toggles render", async () => {
    const ref = createRef<UnifiedSkillsPanelHandle>();

    render(
      <UnifiedSkillsPanel
        ref={ref}
        onOpenDiscovery={() => {}}
        currentApp="claude"
      />,
    );

    await act(async () => {
      await ref.current?.openImport();
    });

    await waitFor(() => {
      expect(screen.getByText("skills.import")).toBeInTheDocument();
      expect(screen.getByText("Shared Skill")).toBeInTheDocument();
      expect(screen.getByText("/tmp/shared-skill")).toBeInTheDocument();
    });
  });
});
