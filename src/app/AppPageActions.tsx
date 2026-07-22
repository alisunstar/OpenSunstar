import { useTranslation } from "react-i18next";
import {
  Download,
  FolderArchive,
  History,
  Plus,
  RefreshCw,
  Search,
  Settings,
} from "lucide-react";

import { Button } from "@/components/ui/button";
import type { PageView } from "./navigation";
import type { PageActionRefs } from "./pageActionRefs";

interface AppPageActionsProps {
  view: PageView;
  refs: PageActionRefs;
  onNavigate: (view: PageView) => void;
}

export function hasPageActions(view: PageView): boolean {
  return [
    "mcp",
    "prompts",
    "commands",
    "hooks",
    "ignore",
    "permissions",
    "agents",
    "skills",
    "skillsDiscovery",
  ].includes(view);
}

export function AppPageActions({
  view,
  refs,
  onNavigate,
}: AppPageActionsProps) {
  const { t } = useTranslation();
  const buttonClass = "hover:bg-black/5 dark:hover:bg-white/5";

  switch (view) {
    case "prompts":
      return (
        <Button
          variant="ghost"
          size="sm"
          onClick={() => refs.promptPanel.current?.openAdd()}
          className={buttonClass}
        >
          <Plus className="w-4 h-4 mr-1" />
          {t("prompts.add", { defaultValue: "添加" })}
        </Button>
      );
    case "commands":
      return (
        <AddButton
          label={t("commands.add", { defaultValue: "添加命令" })}
          onClick={() => refs.commandsPanel.current?.openAdd()}
        />
      );
    case "hooks":
      return (
        <AddButton
          label={t("hooks.add", { defaultValue: "添加钩子" })}
          onClick={() => refs.hooksPanel.current?.openAdd()}
        />
      );
    case "ignore":
      return (
        <AddButton
          label={t("ignore.add", { defaultValue: "添加规则" })}
          onClick={() => refs.ignorePanel.current?.openAdd()}
        />
      );
    case "permissions":
      return (
        <AddButton
          label={t("permissions.add", { defaultValue: "添加权限" })}
          onClick={() => refs.permissionsPanel.current?.openAdd()}
        />
      );
    case "agents":
      return (
        <AddButton
          label={t("agents.add", { defaultValue: "添加 Subagent" })}
          onClick={() => refs.agentsPanel.current?.openAdd()}
        />
      );
    case "mcp":
      return (
        <>
          <ActionButton
            icon={<Download className="w-4 h-4 mr-1" />}
            label={t("mcp.importExisting", { defaultValue: "导入" })}
            onClick={() => refs.mcpPanel.current?.openImport()}
          />
          <AddButton
            label={t("mcp.addMcp", { defaultValue: "添加" })}
            onClick={() => refs.mcpPanel.current?.openAdd()}
          />
          <ActionButton
            icon={<Search className="w-4 h-4 mr-1" />}
            label={t("mcp.discover", { defaultValue: "发现MCP" })}
            onClick={() => onNavigate("mcpDiscovery")}
          />
        </>
      );
    case "skills":
      return (
        <>
          <ActionButton
            icon={<History className="w-4 h-4 mr-1" />}
            label={t("skills.restoreFromBackup.button", {
              defaultValue: "恢复",
            })}
            onClick={() =>
              refs.unifiedSkillsPanel.current?.openRestoreFromBackup()
            }
          />
          <ActionButton
            icon={<FolderArchive className="w-4 h-4 mr-1" />}
            label={t("skills.installFromZip.button", { defaultValue: "安装" })}
            onClick={() =>
              refs.unifiedSkillsPanel.current?.openInstallFromZip()
            }
          />
          <ActionButton
            icon={<Download className="w-4 h-4 mr-1" />}
            label={t("skills.import", { defaultValue: "导入" })}
            onClick={() => refs.unifiedSkillsPanel.current?.openImport()}
          />
          <ActionButton
            icon={<Search className="w-4 h-4 mr-1" />}
            label={t("skills.discover", { defaultValue: "发现" })}
            onClick={() => onNavigate("skillsDiscovery")}
          />
        </>
      );
    case "skillsDiscovery":
      return (
        <>
          <ActionButton
            icon={<RefreshCw className="w-4 h-4 mr-1" />}
            label={t("skills.refresh", { defaultValue: "刷新" })}
            onClick={() => refs.skillsPage.current?.refresh()}
          />
          <ActionButton
            icon={<Settings className="w-4 h-4 mr-1" />}
            label={t("skills.repoManager", { defaultValue: "仓库" })}
            onClick={() => refs.skillsPage.current?.openRepoManager()}
          />
        </>
      );
    default:
      return null;
  }
}

function AddButton({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <ActionButton
      icon={<Plus className="w-4 h-4 mr-1" />}
      label={label}
      onClick={onClick}
    />
  );
}

function ActionButton({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <Button
      variant="ghost"
      size="sm"
      onClick={onClick}
      className="hover:bg-black/5 dark:hover:bg-white/5"
    >
      {icon}
      {label}
    </Button>
  );
}
