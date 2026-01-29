import { useState, useCallback, useEffect } from 'react';
import { IpcService } from '../lib/ipc';
import { SnapshotDAG } from '../types/backend';

export function useSnapshot(workspacePath: string | undefined, onCheckoutCallback?: () => void) {
  const [dag, setDag] = useState<SnapshotDAG | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchDAG = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true);
    try {
      const result = await IpcService.call<SnapshotDAG>('GET_SNAPSHOT_DAG', {
        workspacePath,
      });
      setDag(result);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [workspacePath]);

  const checkoutSnapshot = useCallback(async (snapshotId: string) => {
    if (!workspacePath) return;
    setLoading(true);
    try {
      try {
        await IpcService.call<void>('AUTO_SAVE_CHECKPOINT', {
          workspacePath,
          reason: 'Before restore snapshot',
        });
      } catch {}
      try {
        await IpcService.call<void>('CANCEL_CURRENT_RUN');
      } catch {}
      try {
        await IpcService.call<void>('STOP_GATE_VALIDATION');
      } catch {}
      await IpcService.call<void>('CHECKOUT_SNAPSHOT', {
        workspacePath,
        snapshotId,
      });
      await fetchDAG();
      setError(null);
      if (onCheckoutCallback) onCheckoutCallback();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspacePath, fetchDAG]);

  const manualSaveSnapshot = useCallback(async (name: string, description: string, parentId: string) => {
    if (!workspacePath) return;
    setLoading(true);
    try {
      await IpcService.call<string>('MANUAL_SAVE_SNAPSHOT', {
        workspacePath,
        name,
        description,
        parentId,
      });
      await fetchDAG();
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspacePath, fetchDAG]);

  const updateMetadata = useCallback(async (snapshotId: string, name: string, description: string) => {
    if (!workspacePath) return;
    try {
      await IpcService.call<void>('UPDATE_SNAPSHOT_METADATA', {
        workspacePath,
        snapshotId,
        name,
        description,
      });
      await fetchDAG();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    }
  }, [workspacePath, fetchDAG]);

  const getHead = useCallback(async () => {
    if (!workspacePath) return 'root';
    try {
      return await IpcService.call<string>('GET_DAG_HEAD', { workspacePath });
    } catch (err) {
      console.error('[Epris] Failed to get HEAD:', err);
      return 'root';
    }
  }, [workspacePath]);

  const deleteSnapshotTree = useCallback(async (snapshotId: string) => {
    if (!workspacePath) return;
    setLoading(true);
    try {
      await IpcService.call<string[]>('DELETE_SNAPSHOT_TREE', {
        workspacePath,
        snapshotId,
      });
      await fetchDAG();
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspacePath, fetchDAG]);

  const checkUnsavedChanges = useCallback(async () => {
    if (!workspacePath) return false;
    try {
      return await IpcService.call<boolean>('CHECK_UNSAVED_CHANGES', { workspacePath });
    } catch (err) {
      console.error('[Epris] Failed to check unsaved changes:', err);
      return false;
    }
  }, [workspacePath]);
  
  const clearHistory = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true);
    try {
      await IpcService.call<void>('CLEAR_SNAPSHOT_HISTORY', { workspacePath });
      await fetchDAG();
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspacePath, fetchDAG]);

  useEffect(() => {
    if (workspacePath) {
      fetchDAG();
    }
  }, [workspacePath, fetchDAG]);

  const saveSnapshotLayout = useCallback(async (snapshotId: string, x: number, y: number) => {
    if (!workspacePath) return;

    // Optimistic update
    setDag(prev => {
      if (!prev) return prev;
      return {
        ...prev,
        layout: {
          ...prev.layout,
          [snapshotId]: { x, y }
        }
      };
    });

    try {
      await IpcService.call<void>('SAVE_SNAPSHOT_LAYOUT', {
        workspacePath,
        snapshotId,
        x,
        y,
      });
    } catch (err) {
      console.error('[Epris] Failed to save layout:', err);
      // Revert on error by fetching the current state
      fetchDAG();
    }
  }, [workspacePath, fetchDAG]);

  return {
    dag,
    loading,
    error,
    fetchDAG,
    checkoutSnapshot,
    manualSaveSnapshot,
    updateMetadata,
    getHead,
    deleteSnapshotTree,
    checkUnsavedChanges,
    clearHistory,
    saveSnapshotLayout
  };
}
