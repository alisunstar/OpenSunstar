import { useState, useCallback, useEffect } from "react";
import type { Project } from "@/types/project";
import {
  loadProjects,
  persistProjectsLocal,
  createLocalProject,
  removeProjectLocal,
  PROJECTS_STORAGE_KEY,
} from "@/types/project";
import {
  projectsApi,
  type Project as DbProject,
} from "@/lib/api/projects";
import { clearProjectBoardMetadata } from "@/lib/projectBoardMetadata";

const DB_MIGRATED_KEY = "OpenSunstar-projects-db-sync-v1";

function toDb(project: Project): DbProject {
  const created = Date.parse(project.addedAt) || Date.now();
  return {
    id: project.id,
    name: project.name,
    path: project.path,
    git_remote_url: null,
    created_at: created,
    updated_at: Date.now(),
  };
}

function fromDb(row: DbProject): Project {
  return {
    id: row.id,
    name: row.name,
    path: row.path,
    addedAt: new Date(row.created_at).toISOString(),
  };
}

async function migrateLocalToDb(): Promise<void> {
  if (localStorage.getItem(DB_MIGRATED_KEY)) return;
  for (const project of loadProjects()) {
    try {
      await projectsApi.upsert(toDb(project));
    } catch (e) {
      console.warn("[useProjects] migrate upsert failed", project.id, e);
    }
  }
  localStorage.setItem(DB_MIGRATED_KEY, "1");
}

async function loadFromDbOrLocal(): Promise<Project[]> {
  await migrateLocalToDb();
  const dbRows = await projectsApi.getAll();
  if (dbRows.length > 0) {
    const mapped = dbRows.map(fromDb);
    persistProjectsLocal(mapped);
    return mapped;
  }
  const local = loadProjects();
  for (const project of local) {
    try {
      await projectsApi.upsert(toDb(project));
    } catch {
      /* keep local */
    }
  }
  return local;
}

export function useProjects() {
  const [projects, setProjects] = useState<Project[]>(() => loadProjects());
  const [ready, setReady] = useState(false);

  const reload = useCallback(async () => {
    try {
      const list = await loadFromDbOrLocal();
      setProjects(list);
    } catch (e) {
      console.warn("[useProjects] DB load failed, using localStorage", e);
      setProjects(loadProjects());
    } finally {
      setReady(true);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  useEffect(() => {
    const handleStorage = (e: StorageEvent) => {
      if (e.key === PROJECTS_STORAGE_KEY) setProjects(loadProjects());
    };
    const handleCustom = () => setProjects(loadProjects());
    window.addEventListener("storage", handleStorage);
    window.addEventListener("projects-changed", handleCustom);
    return () => {
      window.removeEventListener("storage", handleStorage);
      window.removeEventListener("projects-changed", handleCustom);
    };
  }, []);

  const add = useCallback(
    async (name: string, path: string, description?: string): Promise<Project> => {
      const project = createLocalProject(name, path, description);
      setProjects(loadProjects());
      try {
        await projectsApi.upsert(toDb(project));
      } catch (e) {
        console.warn("[useProjects] upsert failed", e);
      }
      window.dispatchEvent(new Event("projects-changed"));
      return project;
    },
    [],
  );

  const remove = useCallback(async (id: string) => {
    removeProjectLocal(id);
    clearProjectBoardMetadata(id);
    setProjects(loadProjects());
    try {
      await projectsApi.delete(id);
    } catch (e) {
      console.warn("[useProjects] delete failed", e);
    }
    window.dispatchEvent(new Event("projects-changed"));
  }, []);

  return { projects, add, remove, ready };
}
