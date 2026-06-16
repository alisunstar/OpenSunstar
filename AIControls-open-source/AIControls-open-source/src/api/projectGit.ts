import { invoke } from "@tauri-apps/api/core";

export type ProjectGitInfo = {
  is_repo: boolean;
  branch: string | null;
  branches: string[];
  remote_url: string | null;
  remote_name: string | null;
  last_commit_hash: string | null;
  last_commit_message: string | null;
  last_commit_author: string | null;
  last_commit_date: string | null;
};

export type BranchCommitInfo = {
  hash: string | null;
  message: string | null;
  author: string | null;
  date: string | null;
};

export async function detectProjectGitInfo(
  root: string,
): Promise<ProjectGitInfo | null> {
  try {
    return await invoke<ProjectGitInfo>("detect_project_git_info", { root });
  } catch {
    return null;
  }
}

export async function detectBranchCommitInfo(
  root: string,
  branch: string,
): Promise<BranchCommitInfo | null> {
  try {
    return await invoke<BranchCommitInfo>("detect_branch_commit_info", {
      root,
      branch,
    });
  } catch {
    return null;
  }
}
