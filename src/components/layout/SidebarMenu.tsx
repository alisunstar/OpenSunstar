import { useState } from "react";
import { ChevronDown } from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@/components/ui/collapsible";

interface SidebarMenuProps {
  icon: React.ReactNode;
  label: string;
  defaultOpen?: boolean;
  active?: boolean;
  children: React.ReactNode;
  /** CollapsibleContent 最大高度（超出后内部滚动） */
  maxHeight?: string;
}

export function SidebarMenu({
  icon,
  label,
  defaultOpen = false,
  active,
  children,
  maxHeight,
}: SidebarMenuProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <Collapsible open={open} onOpenChange={setOpen}>
      <CollapsibleTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          className={cn(
            "relative w-full justify-start gap-3 h-9 rounded-lg text-sm font-normal",
            "transition-all duration-150 ease-out group",
            "pl-3 pr-2",
            active
              ? "bg-primary/10 text-primary hover:bg-primary/15"
              : "text-muted-foreground hover:bg-muted/50 hover:text-foreground",
          )}
        >
          {/* 图标 */}
          <span
            className={cn(
              "shrink-0 flex items-center justify-center w-4 h-4 transition-colors duration-150",
              active ? "text-primary" : "text-muted-foreground",
            )}
          >
            {icon}
          </span>

          {/* 标签 */}
          <span className="flex-1 text-left font-medium">{label}</span>

          {/* 折叠箭头 */}
          <ChevronDown
            className={cn(
              "h-3.5 w-3.5 shrink-0 transition-all duration-200 ease-out",
              open && "rotate-180",
              active ? "text-primary/70" : "text-muted-foreground/40",
              "group-hover:text-muted-foreground/80",
            )}
          />
        </Button>
      </CollapsibleTrigger>

      <CollapsibleContent
        className={cn(
          "space-y-0.5 pt-0.5 pb-1",
          "data-[state=open]:animate-collapsible-down data-[state=closed]:animate-collapsible-up",
          maxHeight && "overflow-y-auto",
        )}
        style={maxHeight ? { maxHeight } : undefined}
      >
        {children}
      </CollapsibleContent>
    </Collapsible>
  );
}
