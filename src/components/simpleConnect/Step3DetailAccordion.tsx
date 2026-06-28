import { useTranslation } from "react-i18next";
import { Activity } from "lucide-react";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { PoolHealthPanel } from "./PoolHealthPanel";
import { SC_INNER } from "./ui";

interface Step3DetailAccordionProps {
  poolEnabled: boolean;
  defaultOpen?: string[];
}

export function Step3DetailAccordion({
  poolEnabled,
  defaultOpen = ["runtime"],
}: Step3DetailAccordionProps) {
  const { t } = useTranslation();

  // 密钥池未启用时无详情可展示
  if (!poolEnabled) return null;

  return (
    <Accordion
      type="multiple"
      defaultValue={defaultOpen}
      className="w-full space-y-2"
    >
      <AccordionItem value="runtime" className={`${SC_INNER} border px-0`}>
        <AccordionTrigger className="px-4 py-3 hover:no-underline">
          <span className="flex items-center gap-2 text-sm font-medium">
            <Activity className="h-4 w-4 text-primary" />
            {t("simpleConnect.step3Sections.runtimeSection", {
              defaultValue: "密钥池运行态",
            })}
          </span>
        </AccordionTrigger>
        <AccordionContent className="px-4 pb-4 pt-0">
          <PoolHealthPanel enabled pollMs={2500} embedded />
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  );
}
