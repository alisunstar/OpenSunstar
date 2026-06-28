import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { ExternalLink, Download, Trash2, Loader2, Star } from "lucide-react";
import { settingsApi } from "@/lib/api";
import { cn } from "@/lib/utils";
import type { DiscoverableSkill } from "@/lib/api/skills";

type SkillCardSkill = DiscoverableSkill & { installed: boolean };

export type SkillSource = "repos" | "skillssh" | "clawhub" | "modelscope";

interface SkillCardProps {
  skill: SkillCardSkill;
  onInstall: (key: string) => Promise<void>;
  onUninstall: (key: string) => Promise<void>;
  installs?: number;
  source?: SkillSource;
  stars?: number;
  /** skills.sh 官方榜排名（1–50） */
  rank?: number;
}

const SOURCE_BADGE_STYLES: Record<SkillSource, string> = {
  repos: "bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20",
  skillssh:
    "bg-emerald-500/10 text-emerald-600 dark:text-emerald-400 border-emerald-500/20",
  clawhub:
    "bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-500/20",
  modelscope:
    "bg-violet-500/10 text-violet-600 dark:text-violet-400 border-violet-500/20",
};

const SOURCE_LABELS: Record<SkillSource, string> = {
  repos: "GitHub",
  skillssh: "skills.sh",
  clawhub: "ClawHub",
  modelscope: "ModelScope",
};

export function SkillCard({
  skill,
  onInstall,
  onUninstall,
  installs,
  source,
  stars,
  rank,
}: SkillCardProps) {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(false);

  const handleInstall = async () => {
    setLoading(true);
    try {
      await onInstall(skill.key);
    } finally {
      setLoading(false);
    }
  };

  const handleUninstall = async () => {
    setLoading(true);
    try {
      await onUninstall(skill.key);
    } finally {
      setLoading(false);
    }
  };

  const handleOpenLink = async () => {
    if (skill.readmeUrl) {
      try {
        await settingsApi.openExternal(skill.readmeUrl);
      } catch (error) {
        console.error("Failed to open URL:", error);
      }
    }
  };

  const showDirectory =
    Boolean(skill.directory) &&
    skill.directory.trim().toLowerCase() !== skill.name.trim().toLowerCase();

  return (
    <Card className="glass-card flex flex-col h-full transition-all duration-300 hover:shadow-lg group relative overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-br from-primary/5 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-500 pointer-events-none" />
      <CardHeader className="pb-3">
        <div className="flex items-start justify-between gap-2">
          {typeof rank === "number" && (
            <div
              className={cn(
                "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg font-mono text-sm font-bold",
                rank <= 3
                  ? "bg-amber-500/15 text-amber-600 dark:text-amber-400"
                  : "bg-muted text-muted-foreground",
              )}
              aria-label={`#${rank}`}
            >
              #{rank}
            </div>
          )}
          <div className="flex-1 min-w-0">
            <CardTitle className="text-base font-semibold truncate">
              {skill.name}
            </CardTitle>
            <div className="flex items-center gap-1.5 mt-1.5 flex-wrap">
              {showDirectory && (
                <CardDescription className="text-xs truncate">
                  {skill.directory}
                </CardDescription>
              )}
              {skill.repoOwner && skill.repoName && (
                <Badge
                  variant="outline"
                  className="shrink-0 text-[10px] px-1.5 py-0 h-4 border-border-default"
                >
                  {skill.repoOwner}/{skill.repoName}
                </Badge>
              )}
              {typeof stars === "number" && stars > 0 && (
                <Badge
                  variant="secondary"
                  className="shrink-0 text-[10px] px-1.5 py-0 h-4"
                >
                  <Star className="h-2.5 w-2.5 mr-0.5 text-amber-500" />
                  {stars.toLocaleString()}
                </Badge>
              )}
              {typeof installs === "number" && (
                <Badge
                  variant="secondary"
                  className="shrink-0 text-[10px] px-1.5 py-0 h-4"
                >
                  <Download className="h-2.5 w-2.5 mr-0.5" />
                  {installs.toLocaleString()}
                </Badge>
              )}
            </div>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            {source && (
              <Badge
                variant="outline"
                className={`text-[10px] px-1.5 py-0 h-4 ${SOURCE_BADGE_STYLES[source]}`}
              >
                {SOURCE_LABELS[source]}
              </Badge>
            )}
            {skill.installed && (
              <Badge
                variant="default"
                className="bg-green-600/90 hover:bg-green-600 dark:bg-green-700/90 dark:hover:bg-green-700 text-white border-0"
              >
                {t("skills.installed")}
              </Badge>
            )}
          </div>
        </div>
      </CardHeader>
      {skill.description ? (
        <CardContent className="flex-1 pt-0">
          <p className="text-sm text-muted-foreground/90 line-clamp-4 leading-relaxed">
            {skill.description}
          </p>
        </CardContent>
      ) : (
        <div className="flex-1" />
      )}
      <CardFooter className="flex gap-2 pt-3 border-t border-border/50 relative z-10">
        {skill.readmeUrl && (
          <Button
            variant="ghost"
            size="sm"
            onClick={handleOpenLink}
            disabled={loading}
            className="flex-1"
          >
            <ExternalLink className="h-3.5 w-3.5 mr-1.5" />
            {t("skills.view")}
          </Button>
        )}
        {skill.installed ? (
          <Button
            variant="outline"
            size="sm"
            onClick={handleUninstall}
            disabled={loading}
            className="flex-1 border-red-200 text-red-600 hover:bg-red-50 hover:text-red-700 dark:border-red-900/50 dark:text-red-400 dark:hover:bg-red-950/50 dark:hover:text-red-300"
          >
            {loading ? (
              <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
            ) : (
              <Trash2 className="h-3.5 w-3.5 mr-1.5" />
            )}
            {loading ? t("skills.uninstalling") : t("skills.uninstall")}
          </Button>
        ) : (
          <Button
            variant="mcp"
            size="sm"
            onClick={handleInstall}
            disabled={loading}
            className="flex-1"
          >
            {loading ? (
              <Loader2 className="h-3.5 w-3.5 mr-1.5 animate-spin" />
            ) : (
              <Download className="h-3.5 w-3.5 mr-1.5" />
            )}
            {loading ? t("skills.installing") : t("skills.install")}
          </Button>
        )}
      </CardFooter>
    </Card>
  );
}
