import { useTranslation } from "react-i18next";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface ShortcutsHelpProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

interface ShortcutGroup {
  title: string;
  keys: { key: string; description: string }[];
}

export function ShortcutsHelp({ open, onOpenChange }: ShortcutsHelpProps) {
  const { t } = useTranslation();

  const groups: ShortcutGroup[] = [
    {
      title: t("shortcuts.navigation", { defaultValue: "导航" }),
      keys: [
        {
          key: "Alt+1",
          description: "MCP",
        },
        {
          key: "Alt+2",
          description: "Prompts",
        },
        {
          key: "Alt+3",
          description: "Skills",
        },
        {
          key: "Alt+4",
          description: t("shortcuts.sessions", { defaultValue: "Context" }),
        },
        {
          key: "Alt+5",
          description: t("shortcuts.tokenStats", { defaultValue: "AI Tokens" }),
        },
        {
          key: "Alt+6",
          description: t("shortcuts.kanban", { defaultValue: "看板总览" }),
        },
      ],
    },
    {
      title: t("shortcuts.layout", { defaultValue: "布局" }),
      keys: [
        {
          key: "Ctrl+B",
          description: t("shortcuts.toggleSidebar", {
            defaultValue: "折叠/展开侧边栏",
          }),
        },
        {
          key: "Esc",
          description: t("shortcuts.goBack", {
            defaultValue: "返回上一级 / 关闭弹窗",
          }),
        },
      ],
    },
    {
      title: t("shortcuts.general", { defaultValue: "通用" }),
      keys: [
        {
          key: "? 或 Ctrl+/",
          description: t("shortcuts.showHelp", {
            defaultValue: "显示/隐藏此面板",
          }),
        },
      ],
    },
  ];

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg glass border-border">
        <DialogHeader>
          <DialogTitle>
            {t("shortcuts.title", { defaultValue: "键盘快捷键" })}
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-5">
          {groups.map((group) => (
            <div key={group.title}>
              <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2">
                {group.title}
              </h3>
              <div className="space-y-1.5">
                {group.keys.map((item) => (
                  <div
                    key={item.key}
                    className="flex items-center justify-between py-1.5 px-3 rounded-lg bg-muted/30"
                  >
                    <span className="text-sm text-foreground">
                      {item.description}
                    </span>
                    <kbd className="inline-flex items-center px-2 py-0.5 rounded-md bg-background border border-border text-xs font-mono text-muted-foreground shadow-sm">
                      {item.key}
                    </kbd>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>

        <p className="text-[11px] text-muted-foreground text-center pt-2 border-t border-border/40">
          {t("shortcuts.footer", {
            defaultValue: "按 ? 或 Ctrl+/ 随时呼出此面板",
          })}
        </p>
      </DialogContent>
    </Dialog>
  );
}
