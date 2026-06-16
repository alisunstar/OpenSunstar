//! 项目代码指标 API — 调用 Tauri Rust 后端 (tokei + git)
//! 从 AIControls v0.2.1 移植

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

export interface ProjectProgressResult {
  progress: number;
  summary: string;
}

export interface Contributor {
  name: string;
  email: string;
  commits: number;
}

function warn(msg: string, e: unknown): void {
  console.warn(`[codeMetrics] ${msg}:`, e instanceof Error ? e.message : String(e));
}

/** 使用 tokei 统计项目目录的代码行数 */
export async function countProjectCodeLines(
  root: string,
): Promise<CodeLineResult | null> {
  try {
    return await invoke<CodeLineResult>("count_project_code_lines", { root });
  } catch (e) {
    warn("countProjectCodeLines failed", e);
    return null;
  }
}

/** 读取项目 package.json 中的 version */
export async function readPackageVersion(
  root: string,
): Promise<string | null> {
  try {
    return await invoke<string | null>("read_package_version", { root });
  } catch (e) {
    warn("readPackageVersion failed", e);
    return null;
  }
}

/** AI 评估 MVP 项目的完成进度（待 ai_client 移植后可用） */
export async function estimateProjectProgress(
  _root: string,
): Promise<ProjectProgressResult | null> {
  try {
    return await invoke<ProjectProgressResult>("estimate_project_progress", {
      root: _root,
    });
  } catch {
    // 命令尚未移植，静默返回 null
    return null;
  }
}

/** 统计近 N 天内的 Git 提交数量 */
export async function gitCommitCountLastNDays(
  root: string,
  days: number,
): Promise<number> {
  try {
    return await invoke<number>("git_commit_count_last_n_days", { root, days });
  } catch (e) {
    warn("gitCommitCountLastNDays failed", e);
    return 0;
  }
}

/** 返回最近 12 周每周的提交数量 */
export async function gitWeeklyCommitCounts(
  root: string,
): Promise<number[]> {
  try {
    return await invoke<number[]>("git_weekly_commit_counts", { root });
  } catch (e) {
    warn("gitWeeklyCommitCounts failed", e);
    return Array(12).fill(0);
  }
}

/** 返回 Git 仓库的贡献者列表 */
export async function gitContributors(
  root: string,
): Promise<Contributor[]> {
  try {
    return await invoke<Contributor[]>("git_contributors", { root });
  } catch (e) {
    warn("gitContributors failed", e);
    return [];
  }
}
