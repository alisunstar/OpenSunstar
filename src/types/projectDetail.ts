export type ProjectDetailTab = "overview" | "aiAssets";

export interface ProjectDetailIntent {
  projectId: string;
  tab: ProjectDetailTab;
  /** Increment to re-open the same project with a new tab. */
  key: number;
}
