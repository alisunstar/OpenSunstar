import {
  Bot,
  Code2,
  Sparkles,
  Terminal,
  Workflow,
  Zap,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "@/lib/utils";
import { TOOL_HINTS, TOOL_LABELS } from "./constants";

const TOOL_ICONS: Record<string, typeof Terminal> = {
  "claude-code": Sparkles,
  codex: Terminal,
  "gemini-cli": Bot,
  opencode: Code2,
  openclaw: Workflow,
  hermes: Zap,
};

interface CliToolGridProps {
  tools: string[];
  selectedTool: string;
  configuredTools?: Set<string>;
  onSelect: (tool: string) => void;
}

export function CliToolGrid({
  tools,
  selectedTool,
  configuredTools,
  onSelect,
}: CliToolGridProps) {
  const { t } = useTranslation();

  return (
    <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
      {tools.map((tool) => {
        const Icon = TOOL_ICONS[tool] ?? Terminal;
        const selected = selectedTool === tool;
        const configured = configuredTools?.has(tool);

        return (
          <button
            key={tool}
            type="button"
            onClick={() => onSelect(tool)}
            className={cn(
              "flex items-start gap-3 rounded-xl border p-3 text-left transition-all",
              "hover:bg-muted/40 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
              selected
                ? "border-primary bg-primary/5 ring-1 ring-primary/30"
                : "border-border/50 bg-card/50",
            )}
          >
            <div
              className={cn(
                "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg",
                selected ? "bg-primary/15 text-primary" : "bg-muted text-muted-foreground",
              )}
            >
              <Icon className="h-4 w-4" />
            </div>
            <div className="min-w-0 flex-1">
              <div className="flex items-center gap-2">
                <span className="font-medium text-sm truncate">
                  {TOOL_LABELS[tool] ?? tool}
                </span>
                {configured && (
                  <span className="h-1.5 w-1.5 shrink-0 rounded-full bg-emerald-500" />
                )}
              </div>
              <p className="text-xs text-muted-foreground mt-0.5 truncate">
                {TOOL_HINTS[tool] ??
                  t("simpleConnect.cliGeneric", { defaultValue: "CLI 配置" })}
              </p>
            </div>
          </button>
        );
      })}
    </div>
  );
}
