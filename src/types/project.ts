/** 本地项目仓库 */
export interface Project {
  id: string;
  name: string;
  /** 本地仓库绝对路径 */
  path: string;
  /** 简短描述（可选） */
  description?: string;
  addedAt: string; // ISO timestamp
}

const STORAGE_KEY = "OpenSunstar-projects";

function generateId(): string {
  return `proj_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

export function loadProjects(): Project[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed;
  } catch {
    return [];
  }
}

function saveProjects(projects: Project[]): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(projects));
}

export function addProject(name: string, path: string, description?: string): Project {
  const projects = loadProjects();
  const project: Project = {
    id: generateId(),
    name: name.trim(),
    path: path.trim(),
    description: description?.trim() || undefined,
    addedAt: new Date().toISOString(),
  };
  projects.push(project);
  saveProjects(projects);
  return project;
}

export function removeProject(id: string): void {
  const projects = loadProjects().filter((p) => p.id !== id);
  saveProjects(projects);
}

export function updateProject(id: string, updates: Partial<Pick<Project, "name" | "path">>): Project | null {
  const projects = loadProjects();
  const index = projects.findIndex((p) => p.id === id);
  if (index === -1) return null;
  projects[index] = { ...projects[index], ...updates };
  saveProjects(projects);
  return projects[index];
}
