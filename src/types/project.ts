/** 本地项目仓库（看板 UI 模型；持久化以 SQLite 为主，localStorage 为缓存） */
export interface Project {
  id: string;
  name: string;
  /** 本地仓库绝对路径 */
  path: string;
  /** 简短描述（可选） */
  description?: string;
  addedAt: string; // ISO timestamp
}

export const PROJECTS_STORAGE_KEY = "OpenSunstar-projects";

function generateId(): string {
  return `proj_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

export function loadProjects(): Project[] {
  try {
    const raw = localStorage.getItem(PROJECTS_STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed;
  } catch {
    return [];
  }
}

export function persistProjectsLocal(projects: Project[]): void {
  localStorage.setItem(PROJECTS_STORAGE_KEY, JSON.stringify(projects));
}

export function createLocalProject(
  name: string,
  path: string,
  description?: string,
): Project {
  const projects = loadProjects();
  const project: Project = {
    id: generateId(),
    name: name.trim(),
    path: path.trim(),
    description: description?.trim() || undefined,
    addedAt: new Date().toISOString(),
  };
  projects.push(project);
  persistProjectsLocal(projects);
  return project;
}

export function removeProjectLocal(id: string): void {
  persistProjectsLocal(loadProjects().filter((p) => p.id !== id));
}

export function updateProject(
  id: string,
  updates: Partial<Pick<Project, "name" | "path">>,
): Project | null {
  const projects = loadProjects();
  const index = projects.findIndex((p) => p.id === id);
  if (index === -1) return null;
  projects[index] = { ...projects[index], ...updates };
  persistProjectsLocal(projects);
  return projects[index];
}

/** @deprecated 使用 createLocalProject */
export function addProject(
  name: string,
  path: string,
  description?: string,
): Project {
  return createLocalProject(name, path, description);
}

/** @deprecated 使用 removeProjectLocal */
export function removeProject(id: string): void {
  removeProjectLocal(id);
}
