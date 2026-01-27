import { useState, useCallback, useEffect } from 'react';
import { IpcService } from '../lib/ipc';
import { GateResult } from '../types/backend';

export function useSecurity(workspacePath: string | undefined) {
  const [gateHistory, setGateHistory] = useState<GateResult[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchHistory = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true);
    try {
      const result = await IpcService.call<GateResult[]>('GET_GATE_HISTORY', {
        workspacePath,
      });
      setGateHistory(result);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, [workspacePath]);

  const runGate = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true);
    try {
      const result = await IpcService.call<GateResult>('RUN_GATE', {
        workspacePath,
      });
      await fetchHistory();
      return result;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    } finally {
      setLoading(false);
    }
  }, [workspacePath, fetchHistory]);

  useEffect(() => {
    if (workspacePath) {
      fetchHistory();
    }
  }, [workspacePath, fetchHistory]);

  return {
    gateHistory,
    loading,
    error,
    fetchHistory,
    runGate
  };
}
