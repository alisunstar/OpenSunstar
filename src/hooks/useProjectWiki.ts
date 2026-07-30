/**
 * Project Wiki Baseline React Hooks
 */

import { useCallback, useEffect, useState } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";

import { projectWikiApi } from "@/lib/api/projectWiki";
import type {
  WikiChangedFilesResult,
  WikiCandidate,
  WikiCandidateImportResult,
  WikiComparisonReport,
  WikiInitPlan,
  WikiInitResult,
  WikiLintResult,
  WikiLifecycle,
  WikiDocument,
  WikiScanResult,
} from "@/types/projectWiki";

/** 按需读取正式 Wiki 或隔离候选，供导入、验收前阅读。 */
export function useProjectWikiDocument(projectId: string | undefined) {
  const [data, setData] = useState<WikiDocument | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const open = useCallback(
    async (candidateId?: string): Promise<WikiDocument | null> => {
      if (!projectId) return null;
      setLoading(true);
      setData(null);
      setError(null);
      try {
        const document = await projectWikiApi.readDocument(
          projectId,
          candidateId,
        );
        setData(document);
        return document;
      } catch (cause) {
        const message = String(cause);
        setError(message);
        toast.error(message);
        return null;
      } finally {
        setLoading(false);
      }
    },
    [projectId],
  );

  const close = useCallback(() => {
    setData(null);
    setError(null);
  }, []);

  return { data, loading, error, open, close };
}

/** 扫描 Wiki 状态 */
export function useProjectWikiScan(projectId: string | undefined) {
  const [data, setData] = useState<WikiScanResult | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!projectId) return;
    setLoading(true);
    try {
      // 生命周期由控制面基于 Git 基线重新计算，scan 只读取其最终快照。
      await projectWikiApi.refreshLifecycle(projectId);
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
        if (res.summary.passed && res.summary.warningCount === 0) {
          toast.success(
            t("projectWiki.lint.passed", { defaultValue: "Lint 通过" }),
          );
        } else if (res.summary.passed) {
          toast.warning(
            t("projectWiki.lint.warnings", {
              defaultValue: "Lint 完成：{{warnings}} 个质量警告待处理",
              warnings: res.summary.warningCount,
            }),
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

/** 验收 Wiki 并建立 Git 同步基线。 */
export function useProjectWikiAcceptance(projectId: string | undefined) {
  const [loading, setLoading] = useState(false);
  const { t } = useTranslation();

  const accept = useCallback(async (): Promise<WikiLifecycle | null> => {
    if (!projectId) return null;
    setLoading(true);
    try {
      const lifecycle = await projectWikiApi.accept(projectId);
      toast.success(
        t("projectWiki.accept.success", {
          defaultValue: "Wiki 已验收，并已建立 Commit 同步基线",
        }),
      );
      return lifecycle;
    } catch (e) {
      toast.error(String(e));
      return null;
    } finally {
      setLoading(false);
    }
  }, [projectId, t]);

  return { loading, accept };
}

/** 发现并安全导入生成器候选产物。 */
export function useProjectWikiCandidates(projectId: string | undefined) {
  const [data, setData] = useState<WikiCandidate[]>([]);
  const [loading, setLoading] = useState(false);
  const [importingId, setImportingId] = useState<string | null>(null);
  const { t } = useTranslation();

  const refresh = useCallback(async () => {
    if (!projectId) {
      setData([]);
      return;
    }
    setLoading(true);
    try {
      setData(await projectWikiApi.listCandidates(projectId));
    } catch {
      setData([]);
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const importCandidate = useCallback(
    async (candidateId: string): Promise<WikiCandidateImportResult | null> => {
      if (!projectId) return null;
      setImportingId(candidateId);
      try {
        const result = await projectWikiApi.importCandidate(
          projectId,
          candidateId,
        );
        toast.success(
          t("projectWiki.candidates.importSuccess", {
            defaultValue: "已导入 {{count}} 个文件，正式 Wiki 已进入待验收状态",
            count: result.filesWritten,
          }),
        );
        await refresh();
        return result;
      } catch (e) {
        toast.error(String(e));
        return null;
      } finally {
        setImportingId(null);
      }
    },
    [projectId, refresh, t],
  );

  return { data, loading, importingId, refresh, importCandidate };
}

/** 运行固定 Commit、固定模型的生成器候选质量对照。 */
export function useProjectWikiComparison(projectId: string | undefined) {
  const [data, setData] = useState<WikiComparisonReport | null>(null);
  const [loading, setLoading] = useState(false);

  const compare = useCallback(
    async (candidateIds: string[]) => {
      if (!projectId) return null;
      setLoading(true);
      try {
        const result = await projectWikiApi.compareCandidates(
          projectId,
          candidateIds,
        );
        setData(result);
        return result;
      } catch (e) {
        toast.error(String(e));
        return null;
      } finally {
        setLoading(false);
      }
    },
    [projectId],
  );

  return { data, loading, compare };
}

/** 使用内置 Provider 或可选适配器在隔离源码快照中生成 Wiki。 */
export function useProjectWikiGenerator(projectId: string | undefined) {
  const [loading, setLoading] = useState(false);
  const { t } = useTranslation();

  const generate = useCallback(
    async (engine = "builtin", model?: string) => {
      if (!projectId) return null;
      setLoading(true);
      try {
        const result = await projectWikiApi.runGenerator(
          projectId,
          engine,
          model,
        );
        toast.success(
          t("projectWiki.generator.success", {
            defaultValue:
              "项目 Wiki 已生成并导入，共 {{count}} 个页面，请继续验收",
            count: result.candidate.pageCount,
          }),
        );
        return result;
      } catch (error) {
        toast.error(String(error));
        return null;
      } finally {
        setLoading(false);
      }
    },
    [projectId, t],
  );

  return { loading, generate };
}
