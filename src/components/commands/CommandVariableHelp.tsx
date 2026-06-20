import { useTranslation } from "react-i18next";
import { Info } from "lucide-react";

export function CommandVariableHelp() {
  const { t } = useTranslation();

  return (
    <div className="rounded-lg border border-border/50 bg-muted/30 p-3 text-xs text-muted-foreground space-y-1">
      <div className="flex items-center gap-2 font-medium text-foreground">
        <Info className="h-3.5 w-3.5" />
        {t("commands.variableHelp.title", { defaultValue: "模板变量" })}
      </div>
      <p>
        <code className="px-1 py-0.5 rounded bg-muted">$ARGUMENTS</code> —{" "}
        {t("commands.variableHelp.arguments", {
          defaultValue: "Claude Code / Gemini 用户输入参数占位符",
        })}
      </p>
      <p>
        {t("commands.variableHelp.codexNote", {
          defaultValue: "Codex 不支持独立命令文件，勾选后不会写入磁盘。",
        })}
      </p>
    </div>
  );
}
