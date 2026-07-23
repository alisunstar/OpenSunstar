/**
 * Project Wiki Baseline React Hooks
 */

import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";

import { projectWikiApi } from "@/lib/api/projectWiki";
import type {
  WikiChangedFilesResult,
  WikiInitPlan,
  WikiInitResult,
  WikiLintResult,
  WikiScanResult,
} from "@/types/projectWiki";

/** 扫描 Wiki 状态 */
export function useProjectWikiScan(projectId: string | undefined) {
  const [data, setData] = useState<WikiScanResult | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!projectId) return;
    setLoading(true);
    try {
      const result = await projectWikiApi.scan(projectId);
      setData(result);
    } catch (e) {
      // 静默失败，wiki 区块非关键路径
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { data, loading, refresh };
}

/** 运行 Wiki Lint */
export function useProjectWikiLint(projectId: string | undefined) {
  const [result, setResult] = useState<WikiLintResult | null>(null);
  const [loading, setLoading] = useState(false);
  const { t } = useTranslation();

  const lint = useCallback(
    async (qualityMode = false) => {
      if (!projectId) return;
      setLoading(true);
      try {
        const res = await projectWikiApi.lint(projectId, qualityMode);
        setResult(res);
        if (res.summary.passed) {
          toast.success(
            t("projectWiki.lint.passed", { defaultValue: "Lint 通过" }),
          );
        } else {
          toast.error(
            t("projectWiki.lint.failed", {
              defaultValue: "Lint 失败：{{errors}} 个错误",
              errors: res.summary.errorCount,
            }),
          );
        }
        return res;
      } catch (e) {
        toast.error(String(e));
      } finally {
        setLoading(false);
      }
    },
    [projectId, t],
  );

  return { result, loading, lint };
}

/** 初始化 Wiki */
export function useProjectWikiInit(projectId: string | undefined) {
  const [plan, setPlan] = useState<WikiInitPlan | null>(null);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState(false);
  const { t } = useTranslation();

  const preview = useCallback(async () => {
    if (!projectId) return;
    setLoading(true);
    try {
      const p = await projectWikiApi.previewInit(projectId);
      setPlan(p);
      return p;
    } catch (e) {
      toast.error(String(e));
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  const confirm = useCallback(async () => {
    if (!projectId) return null;
    setInstalling(true);
    try {
      const result: WikiInitResult = await projectWikiApi.init(projectId);
      toast.success(
        t("projectWiki.init.success", {
          defaultValue: "Wiki 初始化完成，已创建 {{count}} 个文件",
          count: result.filesCreated.length,
        }),
      );
      setPlan(null);
      return result;
    } catch (e) {
      toast.error(String(e));
      return null;
    } finally {
      setInstalling(false);
    }
  }, [projectId, t]);

  return { plan, loading, installing, preview, confirm };
}

/** 变更文件映射 */
export function useProjectWikiChangedFiles(projectId: string | undefined) {
  const [data, setData] = useState<WikiChangedFilesResult | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!projectId) return;
    setLoading(true);
    try {
      const result = await projectWikiApi.changedFiles(projectId);
      setData(result);
    } catch {
      // 静默
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  return { data, loading, refresh };
}
