/**
 * Project Wiki Baseline 前端 API
 *
 * 封装与后端 Tauri 命令的交互。
 */

import { invoke } from "@tauri-apps/api/core";

import type {
  WikiChangedFilesResult,
  WikiInitPlan,
  WikiInitResult,
  WikiInventory,
  WikiLintResult,
  WikiScanResult,
} from "@/types/projectWiki";

export const projectWikiApi = {
  /** 扫描项目 Wiki 状态 */
  scan: (projectId: string): Promise<WikiScanResult> =>
    invoke<WikiScanResult>("scan_project_wiki_cmd", { projectId }),

  /** 构建 Wiki Inventory */
  inventory: (projectId: string): Promise<WikiInventory> =>
    invoke<WikiInventory>("inventory_project_wiki_cmd", { projectId }),

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
};
