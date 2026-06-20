import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Eye } from "lucide-react";
import { useTranslation } from "react-i18next";

interface DiffConfirmDialogProps {
  isOpen: boolean;
  title: string;
  description?: string;
  filePath?: string;
  leftLabel: string;
  rightLabel: string;
  currentContent: string;
  newContent: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export function DiffConfirmDialog({
  isOpen,
  title,
  description,
  filePath,
  leftLabel,
  rightLabel,
  currentContent,
  newContent,
  onConfirm,
  onCancel,
}: DiffConfirmDialogProps) {
  const { t } = useTranslation();

  return (
    <Dialog
      open={isOpen}
      onOpenChange={(open) => {
        if (!open) onCancel();
      }}
    >
      <DialogContent className="max-w-4xl max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Eye className="h-5 w-5 text-blue-500" />
            {title}
          </DialogTitle>
          {description && (
            <DialogDescription>{description}</DialogDescription>
          )}
          {filePath && (
            <p className="text-xs font-mono text-muted-foreground truncate">
              {filePath}
            </p>
          )}
        </DialogHeader>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3 flex-1 min-h-0 overflow-hidden">
          <div className="flex flex-col min-h-0">
            <div className="text-xs font-medium text-muted-foreground mb-1.5">
              {leftLabel}
            </div>
            <pre className="flex-1 overflow-auto rounded-lg border border-border/60 bg-muted/30 p-3 text-xs leading-relaxed whitespace-pre-wrap font-mono min-h-[200px] max-h-[50vh]">
              {currentContent || "—"}
            </pre>
          </div>
          <div className="flex flex-col min-h-0">
            <div className="text-xs font-medium text-muted-foreground mb-1.5">
              {rightLabel}
            </div>
            <pre className="flex-1 overflow-auto rounded-lg border border-blue-500/30 bg-blue-500/5 p-3 text-xs leading-relaxed whitespace-pre-wrap font-mono min-h-[200px] max-h-[50vh]">
              {newContent || "—"}
            </pre>
          </div>
        </div>

        <DialogFooter className="gap-2 sm:justify-end">
          <Button variant="outline" onClick={onCancel}>
            {t("common.cancel")}
          </Button>
          <Button onClick={onConfirm}>
            {t("dryRun.confirmWrite", { defaultValue: "确认写入" })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
