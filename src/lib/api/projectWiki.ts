/**
 * Project Wiki Baseline 前端 API
 *
 * 封装与后端 Tauri 命令的交互。
 */

import { invoke } from "@tauri-apps/api/core";

import type {
  WikiChangedFilesResult,
  WikiCandidate,
  WikiCandidateImportResult,
  WikiComparisonReport,
  WikiGeneratorRunResult,
  WikiInitPlan,
  WikiInitResult,
  WikiInventory,
  WikiDocument,
  WikiLintResult,
  WikiLifecycle,
  WikiScanResult,
} from "@/types/projectWiki";

export const projectWikiApi = {
  /** 扫描项目 Wiki 状态 */
  scan: (projectId: string): Promise<WikiScanResult> =>
    invoke<WikiScanResult>("scan_project_wiki_cmd", { projectId }),

  /** 构建 Wiki Inventory */
  inventory: (projectId: string): Promise<WikiInventory> =>
    invoke<WikiInventory>("inventory_project_wiki_cmd", { projectId }),

  /** 读取正式 Wiki；传入 candidateId 时读取隔离候选。 */
  readDocument: (
    projectId: string,
    candidateId?: string,
  ): Promise<WikiDocument> =>
    invoke<WikiDocument>("read_project_wiki_document_cmd", {
      projectId,
      candidateId: candidateId ?? null,
    }),

  /** 在系统文件管理器中打开正式 Wiki 目录。 */
  openFolder: (projectId: string): Promise<void> =>
    invoke<void>("open_project_wiki_folder_cmd", { projectId }),

  /** 运行 Wiki Lint 校验 */
  lint: (projectId: string, qualityMode = false): Promise<WikiLintResult> =>
    invoke<WikiLintResult>("run_project_wiki_lint_cmd", {
      projectId,
      qualityMode,
    }),

  /** 预览 Wiki 初始化 */
  previewInit: (projectId: string): Promise<WikiInitPlan> =>
    invoke<WikiInitPlan>("preview_project_wiki_init_cmd", { projectId }),

  /** 初始化 Wiki */
  init: (projectId: string): Promise<WikiInitResult> =>
    invoke<WikiInitResult>("init_project_wiki_cmd", { projectId }),

  /** 映射变更文件到 Wiki 页面 */
  changedFiles: (
    projectId: string,
    changedFiles?: string[],
  ): Promise<WikiChangedFilesResult> =>
    invoke<WikiChangedFilesResult>("map_project_wiki_changed_files_cmd", {
      projectId,
      changedFiles,
    }),

  /** 验收候选 Wiki，并把当前 Git commit 记录为同步基线 */
  accept: (projectId: string): Promise<WikiLifecycle> =>
    invoke<WikiLifecycle>("accept_project_wiki_cmd", { projectId }),

  /** 从 Git 基线和当前文件状态重新计算 Wiki 生命周期 */
  refreshLifecycle: (projectId: string): Promise<WikiLifecycle> =>
    invoke<WikiLifecycle>("refresh_project_wiki_lifecycle_cmd", { projectId }),

  /** 列出生成器写入隔离目录的 Wiki 候选。 */
  listCandidates: (projectId: string): Promise<WikiCandidate[]> =>
    invoke<WikiCandidate[]>("list_project_wiki_candidates_cmd", { projectId }),

  /** 备份现有 Wiki 后，将候选导入正式目录并进入待验收状态。 */
  importCandidate: (
    projectId: string,
    candidateId: string,
  ): Promise<WikiCandidateImportResult> =>
    invoke<WikiCandidateImportResult>("import_project_wiki_candidate_cmd", {
      projectId,
      candidateId,
    }),

  /** 固定 Commit、固定模型对照多个候选的可验证质量指标。 */
  compareCandidates: (
    projectId: string,
    candidateIds: string[],
  ): Promise<WikiComparisonReport> =>
    invoke<WikiComparisonReport>("compare_project_wiki_candidates_cmd", {
      projectId,
      candidateIds,
    }),

  /** 在隔离快照中运行生成器；内置引擎会校验并自动导入为待验收版本。 */
  runGenerator: (
    projectId: string,
    engine: string,
    model?: string,
  ): Promise<WikiGeneratorRunResult> =>
    invoke<WikiGeneratorRunResult>("run_project_wiki_generator_cmd", {
      projectId,
      engine,
      model,
    }),
};
