import { cn } from "@/lib/utils";
import type { HookEventType } from "@/lib/api/hooks";

const STYLES: Record<HookEventType, string> = {
  PreToolUse: "bg-blue-100 text-blue-800 dark:bg-blue-950 dark:text-blue-300",
  PostToolUse: "bg-green-100 text-green-800 dark:bg-green-950 dark:text-green-300",
  Notification:
    "bg-amber-100 text-amber-800 dark:bg-amber-950 dark:text-amber-300",
  Stop: "bg-red-100 text-red-800 dark:bg-red-950 dark:text-red-300",
};

interface HookEventTypeBadgeProps {
  eventType: HookEventType;
  className?: string;
}

export function HookEventTypeBadge({
  eventType,
  className,
}: HookEventTypeBadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-md px-2 py-0.5 text-xs font-medium",
        STYLES[eventType],
        className,
      )}
    >
      {eventType}
    </span>
  );
}
