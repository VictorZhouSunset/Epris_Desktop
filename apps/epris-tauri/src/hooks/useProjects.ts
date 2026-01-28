import { useCallback, useEffect, useMemo, useState } from 'react';
import { IpcService } from '../lib/ipc';
import { ProjectInfo, ProjectsOverview } from '../types/backend';

export function useProjects() {
  const [overview, setOverview] = useState<ProjectsOverview | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const o = await IpcService.call<ProjectsOverview>('GET_PROJECTS_OVERVIEW');
      setOverview(o);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const activeProject: ProjectInfo | null = useMemo(() => {
    if (!overview?.active_project_id) return null;
    return overview.projects.find((p) => p.id === overview.active_project_id) ?? null;
  }, [overview]);

  const createProject = useCallback(
    async (projectsRoot: string | undefined, name: string | undefined) => {
      const p = await IpcService.call<ProjectInfo>('CREATE_PROJECT', {
        projectsRoot,
        name,
      });
      await refresh();
      return p;
    },
    [refresh],
  );

  const setActiveProject = useCallback(
    async (projectId: string) => {
      await IpcService.call<void>('SET_ACTIVE_PROJECT', { projectId });
      await refresh();
    },
    [refresh],
  );

  const deleteProject = useCallback(
    async (projectId: string) => {
      await IpcService.call<void>('DELETE_PROJECT', { projectId });
      await refresh();
    },
    [refresh],
  );

  const renameProject = useCallback(
    async (projectId: string, name: string) => {
      await IpcService.call<void>('RENAME_PROJECT', { projectId, name });
      await refresh();
    },
    [refresh],
  );

  const getDefaultProjectsRoot = useCallback(async () => {
    return IpcService.call<string>('GET_DEFAULT_PROJECTS_ROOT');
  }, []);

  const pickProjectsRoot = useCallback(async (initial?: string) => {
    return IpcService.call<string | null>('PICK_PROJECTS_ROOT', { initial });
  }, []);

  const setProjectsRoot = useCallback(
    async (newRoot: string, moveExisting: boolean) => {
      await IpcService.call<ProjectsOverview>('SET_PROJECTS_ROOT', {
        newRoot,
        moveExisting,
      });
      await refresh();
    },
    [refresh],
  );

  return {
    overview,
    loading,
    error,
    refresh,
    activeProject,
    createProject,
    setActiveProject,
    deleteProject,
    renameProject,
    getDefaultProjectsRoot,
    pickProjectsRoot,
    setProjectsRoot,
  };
}
