import { useState } from "react";
import { useTranslation } from "react-i18next";
import { FolderSearch } from "lucide-react";
import { toast } from "sonner";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

interface AddProjectDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onAdd: (name: string, path: string, description?: string) => void;
}

export function AddProjectDialog({
  open,
  onOpenChange,
  onAdd,
}: AddProjectDialogProps) {
  const { t } = useTranslation();
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [desc, setDesc] = useState("");

  const handleSubmit = () => {
    const trimmedName = name.trim();
    const trimmedPath = path.trim();

    if (!trimmedName) {
      toast.error(
        t("projects.validation.nameRequired", {
          defaultValue: "请输入项目名称",
        }),
      );
      return;
    }
    if (!trimmedPath) {
      toast.error(
        t("projects.validation.pathRequired", {
          defaultValue: "请输入项目路径",
        }),
      );
      return;
    }

    onAdd(trimmedName, trimmedPath, desc.trim() || undefined);
    setName("");
    setPath("");
    setDesc("");
    onOpenChange(false);
  };

  const handleBrowse = async () => {
    try {
      // Tauri dialog API — 选择目录
      const { open: openDialog } = await import("@tauri-apps/plugin-dialog");
      const selected = await openDialog({ directory: true, multiple: false });
      if (selected && typeof selected === "string") {
        setPath(selected);
      }
    } catch {
      toast.info(
        t("projects.manualPathHint", {
          defaultValue: "请手动输入项目路径",
        }),
      );
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-md glass border-border p-0 gap-0 overflow-hidden">
        <DialogHeader className="text-left">
          <DialogTitle>
            {t("projects.addTitle", { defaultValue: "添加项目" })}
          </DialogTitle>
          <DialogDescription className="leading-relaxed">
            {t("projects.addDescription", {
              defaultValue: "添加本地项目仓库到侧边栏，方便快速切换管理",
            })}
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-5 px-6 py-5">
          <div className="space-y-2">
            <Label htmlFor="project-name" className="block">
              {t("projects.nameLabel", { defaultValue: "项目名称" })}
            </Label>
            <Input
              id="project-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder={t("projects.namePlaceholder", {
                defaultValue: "例如：我的项目",
              })}
              onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="project-path" className="block">
              {t("projects.pathLabel", { defaultValue: "本地路径" })}
            </Label>
            <div className="flex items-center gap-2">
              <Input
                id="project-path"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                placeholder={t("projects.pathPlaceholder", {
                  defaultValue: "例如：D:\\projects\\my-repo",
                })}
                className="min-w-0 flex-1"
                onKeyDown={(e) => e.key === "Enter" && handleSubmit()}
              />
              <Button
                type="button"
                variant="outline"
                size="icon"
                className="h-9 w-9 shrink-0"
                onClick={handleBrowse}
                title={t("projects.browseFolder", { defaultValue: "浏览文件夹" })}
              >
                <FolderSearch className="h-4 w-4" />
              </Button>
            </div>
          </div>

          <div className="space-y-2">
            <div className="flex items-baseline gap-2">
              <Label htmlFor="project-desc" className="block">
                {t("projects.descLabel", { defaultValue: "项目描述" })}
              </Label>
              <span className="text-xs text-muted-foreground">
                {t("common.optional", { defaultValue: "可选" })}
              </span>
            </div>
            <Textarea
              id="project-desc"
              value={desc}
              onChange={(e) => setDesc(e.target.value)}
              placeholder={t("projects.descPlaceholder", {
                defaultValue: "简要描述项目用途、技术栈或当前目标",
              })}
              rows={3}
              className="min-h-[72px] resize-none"
            />
          </div>
        </div>

        <DialogFooter className="gap-3">
          <Button
            variant="ghost"
            onClick={() => onOpenChange(false)}
            className="hover:bg-muted/50"
          >
            {t("common.cancel", { defaultValue: "取消" })}
          </Button>
          <Button onClick={handleSubmit}>
            {t("common.add", { defaultValue: "添加" })}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
