import { invoke } from "@tauri-apps/api/core";

export interface SddDescriptorSummary {
  id: string;
  name: string;
  version: string;
  phaseModel: string;
  installType: string;
  descriptionZh?: string;
  descriptionEn?: string;
  repoUrl?: string;
  starCount?: number;
}

export interface SignalMatch {
  signal: string;
  matchedPath: string;
  confidence: string;
}

export interface SddDetectionResult {
  descriptorId: string;
  detected: boolean;
  confidence: string;
  signalMatches: SignalMatch[];
}

export const sddApi = {
  async listDescriptors(): Promise<SddDescriptorSummary[]> {
    return await invoke<SddDescriptorSummary[]>("sdd_list_descriptors_cmd");
  },

  async detectProject(projectId: string): Promise<SddDetectionResult[]> {
    return await invoke<SddDetectionResult[]>("sdd_detect_project_cmd", {
      projectId,
    });
  },

  async detectAllProjects(): Promise<Record<string, SddDetectionResult[]>> {
    return await invoke<Record<string, SddDetectionResult[]>>(
      "sdd_detect_all_projects_cmd",
    );
  },

  async getDetectionResults(
    projectId: string,
  ): Promise<SddDetectionResult[]> {
    return await invoke<SddDetectionResult[]>(
      "sdd_get_detection_results_cmd",
      { projectId },
    );
  },
};
