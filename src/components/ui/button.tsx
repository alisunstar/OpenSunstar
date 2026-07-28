import * as React from "react";
import { Slot } from "@radix-ui/react-slot";
import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

const buttonVariants = cva(
  "inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-lg text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1 ring-offset-background disabled:pointer-events-none disabled:opacity-50",
  {
    variants: {
      variant: {
        // 主按钮：语义 token，--primary 在浅色/暗色下均保证白字对比度 ≥4.5:1
        default: "bg-primary text-primary-foreground hover:bg-primary-hover",
        // 危险按钮
        destructive:
          "bg-destructive text-destructive-foreground hover:bg-destructive-hover",
        // 轮廓按钮
        outline:
          "border border-input bg-background text-foreground hover:bg-accent hover:text-accent-foreground",
        // 次按钮
        secondary:
          "bg-secondary text-secondary-foreground hover:bg-secondary/80",
        // 幽灵按钮
        ghost:
          "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
        // 【例外变体】MCP/安装动作按钮：emerald 品牌色。
        // 注意：这是基座中唯一带业务语义的变体——MCP 管理是本产品核心域，
        // emerald 已是事实品牌色。此为例外而非常态，新业务色不得再进入基座。
        mcp: "bg-mcp text-mcp-foreground hover:bg-mcp-hover",
        // 链接按钮
        link: "text-primary underline-offset-4 hover:underline",
      },
      size: {
        default: "h-9 px-4 py-2",
        sm: "h-8 rounded-md px-3 text-xs",
        lg: "h-10 rounded-md px-8",
        icon: "h-9 w-9 p-1.5",
      },
    },
    defaultVariants: {
      variant: "default",
      size: "default",
    },
  },
);

export interface ButtonProps
  extends
    React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof buttonVariants> {
  asChild?: boolean;
}

const Button = React.forwardRef<HTMLButtonElement, ButtonProps>(
  ({ className, variant, size, asChild = false, ...props }, ref) => {
    const Comp = asChild ? Slot : "button";
    return (
      <Comp
        className={cn(buttonVariants({ variant, size, className }))}
        ref={ref}
        {...props}
      />
    );
  },
);
Button.displayName = "Button";

export { Button, buttonVariants };
