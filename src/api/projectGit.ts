//! 项目 Git 信息 API — 调用 Tauri Rust 后端
//! 从 AIControls v0.2.1 移植

import { invoke } from "@tauri-apps/api/core";

export interface ProjectGitInfo {
  is_repo: boolean;
  branch: string | null;
  branches: string[];
  remote_url: string | null;
  remote_name: string | null;
  last_commit_hash: string | null;
  last_commit_message: string | null;
  last_commit_author: string | null;
  last_commit_date: string | null;
}

/** 检测项目的 Git 仓库信息 */
export async function detectProjectGitInfo(
  root: string,
): Promise<ProjectGitInfo | null> {
  try {
    return await invoke<ProjectGitInfo>("detect_project_git_info", { root });
  } catch {
    return null;
  }
}
