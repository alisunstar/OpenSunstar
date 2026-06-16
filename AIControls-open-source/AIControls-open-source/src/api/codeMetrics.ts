import { invoke } from "@tauri-apps/api/core";

export interface LanguageStat {
  language: string;
  code_lines: number;
  comment_lines: number;
  blank_lines: number;
  files: number;
}

export interface CodeLineResult {
  total_lines: number;
  code_lines: number;
  comment_lines: number;
  blank_lines: number;
  files: number;
  languages: LanguageStat[];
}

function formatInvokeError(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return String(e);
}

/** 使用 tokei 统计项目目录的代码行数；失败时返回 null。 */
export async function countProjectCodeLines(root: string): Promise<CodeLineResult | null> {
  try {
    return await invoke<CodeLineResult>("count_project_code_lines", { root });
  } catch (e) {
    console.warn("[countProjectCodeLines] failed:", formatInvokeError(e));
    return null;
  }
}

/** 读取项目 package.json 中的 version；无文件或失败时返回 null。 */
export async function readPackageVersion(root: string): Promise<string | null> {
  try {
    return await invoke<string | null>("read_package_version", { root });
  } catch (e) {
    console.warn("[readPackageVersion] failed:", formatInvokeError(e));
    return null;
  }
}

export interface ProjectProgressResult {
  progress: number;
  summary: string;
}

/** AI 评估 MVP 项目的完成进度；失败时返回 null。 */
export async function estimateProjectProgress(root: string): Promise<ProjectProgressResult | null> {
  try {
    return await invoke<ProjectProgressResult>("estimate_project_progress", { root });
  } catch (e) {
    console.warn("[estimateProjectProgress] failed:", formatInvokeError(e));
    return null;
  }
}

/** 统计近 N 天内的 Git 提交数量；失败时返回 0。 */
export async function gitCommitCountLastNDays(root: string, days: number): Promise<number> {
  try {
    return await invoke<number>("git_commit_count_last_n_days", { root, days });
  } catch (e) {
    console.warn("[gitCommitCountLastNDays] failed:", formatInvokeError(e));
    return 0;
  }
}

/** 返回最近 12 周每周的提交数量（从最旧到最新）；失败时返回全零数组。 */
export async function gitWeeklyCommitCounts(root: string): Promise<number[]> {
  try {
    return await invoke<number[]>("git_weekly_commit_counts", { root });
  } catch (e) {
    console.warn("[gitWeeklyCommitCounts] failed:", formatInvokeError(e));
    return Array(12).fill(0);
  }
}

/** 返回 Git 仓库的贡献者列表（按提交数降序）。 */
export interface Contributor {
  name: string;
  email: string;
  commits: number;
}

export async function gitContributors(root: string): Promise<Contributor[]> {
  try {
    return await invoke<Contributor[]>("git_contributors", { root });
  } catch (e) {
    console.warn("[gitContributors] failed:", formatInvokeError(e));
    return [];
  }
}

export interface LocalChangeStatus {
  has_changes: boolean;
  details: string;
}

export async function gitCheckLocalChanges(root: string): Promise<LocalChangeStatus | null> {
  try {
    return await invoke<LocalChangeStatus>("git_check_local_changes", { root });
  } catch (e) {
    console.warn("[gitCheckLocalChanges] failed:", formatInvokeError(e));
    return null;
  }
}

export async function gitPull(root: string): Promise<string> {
  return await invoke<string>("git_pull", { root });
}
