import { useState, useEffect, useCallback } from "react";
import type { Project } from "@/types/project";
import {
  countProjectCodeLines,
  readPackageVersion,
  gitCommitCountLastNDays,
  gitWeeklyCommitCounts,
  gitContributors,
  type CodeLineResult,
  type Contributor,
} from "@/api/codeMetrics";
import { detectProjectGitInfo, type ProjectGitInfo } from "@/api/projectGit";
import {
  PORTFOLIO_COMMIT_WINDOW_DAYS,
  mapWithConcurrency,
} from "@/lib/portfolioMetrics";

export function useProjectMetricsScan(projects: Project[]) {
  const [codeLinesMap, setCodeLinesMap] = useState<
    Map<string, CodeLineResult>
  >(new Map());
  const [versionMap, setVersionMap] = useState<Map<string, string>>(new Map());
  const [gitInfoMap, setGitInfoMap] = useState<Map<string, ProjectGitInfo>>(
    new Map(),
  );
  const [commits7dMap, setCommits7dMap] = useState<Map<string, number>>(
    new Map(),
  );
  const [commits30dMap, setCommits30dMap] = useState<Map<string, number>>(
    new Map(),
  );
  const [contributorsMap, setContributorsMap] = useState<
    Map<string, Contributor[]>
  >(new Map());
  const [weeklyCommitsMap, setWeeklyCommitsMap] = useState<
    Map<string, number[]>
  >(new Map());
  const [scanning, setScanning] = useState(false);
  const [scanEpoch, setScanEpoch] = useState(0);
  const [scanProgress, setScanProgress] = useState({ done: 0, total: 0 });

  useEffect(() => {
    if (projects.length === 0) {
      setScanning(false);
      return;
    }
    let cancelled = false;
    setScanning(true);
    setScanProgress({ done: 0, total: projects.length });

    const scan = async () => {
      await mapWithConcurrency(projects, 4, async (p) => {
        if (cancelled) return;
        try {
          const [code, version, commits7d, commits30d, contribs, gitInfo, weekly] =
            await Promise.all([
              countProjectCodeLines(p.path),
              readPackageVersion(p.path),
              gitCommitCountLastNDays(p.path, PORTFOLIO_COMMIT_WINDOW_DAYS),
              gitCommitCountLastNDays(p.path, 30),
              gitContributors(p.path),
              detectProjectGitInfo(p.path),
              gitWeeklyCommitCounts(p.path),
            ]);
          if (cancelled) return;
          if (code) setCodeLinesMap((m) => new Map(m).set(p.id, code));
          if (version) setVersionMap((m) => new Map(m).set(p.id, version));
          setCommits7dMap((m) => new Map(m).set(p.id, commits7d));
          setCommits30dMap((m) => new Map(m).set(p.id, commits30d));
          if (contribs.length > 0)
            setContributorsMap((m) => new Map(m).set(p.id, contribs));
          if (gitInfo) setGitInfoMap((m) => new Map(m).set(p.id, gitInfo));
          if (weekly && weekly.length > 0)
            setWeeklyCommitsMap((m) => new Map(m).set(p.id, weekly));
        } catch {
          /* 单个项目失败不影响其他 */
        } finally {
          if (!cancelled) {
            setScanProgress((prev) => ({ ...prev, done: prev.done + 1 }));
          }
        }
      });
      if (!cancelled) {
        setScanning(false);
      }
    };

    void scan();
    return () => {
      cancelled = true;
    };
  }, [projects, scanEpoch]);

  const refreshScan = useCallback(() => {
    setScanEpoch((n) => n + 1);
  }, []);

  return {
    codeLinesMap,
    versionMap,
    gitInfoMap,
    commits7dMap,
    commits30dMap,
    contributorsMap,
    weeklyCommitsMap,
    scanning,
    scanProgress,
    scanEpoch,
    refreshScan,
  };
}
