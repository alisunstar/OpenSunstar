import { useState, useCallback, useEffect } from "react";
import type { Project } from "@/types/project";
import { loadProjects, addProject, removeProject } from "@/types/project";

const STORAGE_KEY = "OpenSunstar-projects";

export function useProjects() {
  const [projects, setProjects] = useState<Project[]>(() => loadProjects());

  // 跨组件同步：监听 storage 事件 + 自定义事件
  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key === STORAGE_KEY) {
        setProjects(loadProjects());
      }
    };
    const handleCustom = () => setProjects(loadProjects());
    window.addEventListener("storage", handleStorage);
    window.addEventListener("projects-changed", handleCustom);
    return () => {
      window.removeEventListener("storage", handleStorage);
      window.removeEventListener("projects-changed", handleCustom);
    };
  }, []);

  const add = useCallback((name: string, path: string, description?: string): Project => {
    const project = addProject(name, path, description);
    setProjects(loadProjects());
    window.dispatchEvent(new Event("projects-changed"));
    return project;
  }, []);

  const remove = useCallback((id: string) => {
    removeProject(id);
    setProjects(loadProjects());
    window.dispatchEvent(new Event("projects-changed"));
  }, []);

  return { projects, add, remove };
}
