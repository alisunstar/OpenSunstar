import { useTranslation } from "react-i18next";
import { Activity, BarChart3, ListChecks } from "lucide-react";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { PoolHealthPanel } from "./PoolHealthPanel";
import { UsageSummaryPanel } from "./UsageSummaryPanel";
import { ToolStatusPanel } from "./ToolStatusPanel";
import { SC_INNER } from "./ui";

interface Step3DetailAccordionProps {
  poolEnabled: boolean;
  statusRefresh: number;
  selectedTool: string;
  onSelectTool: (tool: string) => void;
  defaultOpen?: string[];
}

export function Step3DetailAccordion({
  poolEnabled,
  statusRefresh,
  selectedTool,
  onSelectTool,
  defaultOpen = ["status"],
}: Step3DetailAccordionProps) {
  const { t } = useTranslation();

  return (
    <Accordion
      type="multiple"
      defaultValue={defaultOpen}
      className="w-full space-y-2"
    >
      <AccordionItem value="status" className={`${SC_INNER} border px-0`}>
        <AccordionTrigger className="px-4 py-3 hover:no-underline">
          <span className="flex items-center gap-2 text-sm font-medium">
            <ListChecks className="h-4 w-4 text-primary" />
            {t("simpleConnect.step3.statusSection", {
              defaultValue: "CLI 配置状态",
            })}
          </span>
        </AccordionTrigger>
        <AccordionContent className="px-4 pb-4 pt-0">
          <ToolStatusPanel
            refreshToken={statusRefresh}
            selectedTool={selectedTool}
            onSelectTool={onSelectTool}
            embedded
          />
        </AccordionContent>
      </AccordionItem>

      {poolEnabled && (
        <AccordionItem value="runtime" className={`${SC_INNER} border px-0`}>
          <AccordionTrigger className="px-4 py-3 hover:no-underline">
            <span className="flex items-center gap-2 text-sm font-medium">
              <Activity className="h-4 w-4 text-primary" />
              {t("simpleConnect.step3.runtimeSection", {
                defaultValue: "密钥池运行态",
              })}
            </span>
          </AccordionTrigger>
          <AccordionContent className="px-4 pb-4 pt-0">
            <PoolHealthPanel enabled pollMs={2500} embedded />
          </AccordionContent>
        </AccordionItem>
      )}

      <AccordionItem value="usage" className={`${SC_INNER} border px-0`}>
        <AccordionTrigger className="px-4 py-3 hover:no-underline">
          <span className="flex items-center gap-2 text-sm font-medium">
            <BarChart3 className="h-4 w-4 text-primary" />
            {t("simpleConnect.step3.usageSection", {
              defaultValue: "用量概览（只读）",
            })}
          </span>
        </AccordionTrigger>
        <AccordionContent className="px-4 pb-4 pt-0">
          <UsageSummaryPanel embedded />
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  );
}
