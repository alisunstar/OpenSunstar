import * as React from "react";
import { cn } from "@/lib/utils";

/**
 * 排版基元：统一标题/辅助文字层级，替代散落的 text-[Npx] 任意值。
 * 字号阶梯：Caption 12px → 正文 14px → SectionTitle 16px → PageTitle 20px。
 */

function PageTitle({
  className,
  ...props
}: React.HTMLAttributes<HTMLHeadingElement>) {
  return (
    <h1
      className={cn(
        "text-xl font-semibold tracking-tight text-foreground",
        className,
      )}
      {...props}
    />
  );
}

function SectionTitle({
  className,
  ...props
}: React.HTMLAttributes<HTMLHeadingElement>) {
  return (
    <h2
      className={cn("text-base font-semibold text-foreground", className)}
      {...props}
    />
  );
}

function Caption({
  className,
  ...props
}: React.HTMLAttributes<HTMLParagraphElement>) {
  return (
    <p
      className={cn("text-xs text-muted-foreground", className)}
      {...props}
    />
  );
}

export { PageTitle, SectionTitle, Caption };
