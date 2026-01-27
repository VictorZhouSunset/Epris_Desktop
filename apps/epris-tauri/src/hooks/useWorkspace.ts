import { useState, useEffect, useCallback, useRef } from 'react';
import { IpcService } from '../lib/ipc';

export interface WorkspaceConfig {
  path: string;
  exists: boolean;
}

export type WorkspaceStatus = 'idle' | 'loading' | 'ready' | 'error';

export function useWorkspace() {
  const [workspace, setWorkspace] = useState<WorkspaceConfig | null>(null);
  const [status, setStatus] = useState<WorkspaceStatus>('idle');
  const [error, setError] = useState<Error | null>(null);
  
  const requestCountRef = useRef(0);

  const init = useCallback(async (): Promise<void> => {
    const requestId = ++requestCountRef.current;
    
    setStatus('loading');
    setError(null);
    
    try {
      const config = await IpcService.call<WorkspaceConfig>('GET_WORKSPACE_CONFIG');
      
      if (requestId === requestCountRef.current) {
        setWorkspace(config);
        setStatus('ready');
      }
    } catch (err) {
      if (requestId === requestCountRef.current) {
        const normalizedError = err instanceof Error ? err : new Error(String(err));
        setError(normalizedError);
        setStatus('error');
      }
      // No throw here to allow useEffect to stay quiet, but still logs to status/error
    }
  }, []);

  useEffect(() => {
    void init();
    return () => {
      requestCountRef.current++;
    };
  }, [init]);

  const reload = useCallback(() => init(), [init]);

  const effectiveStatus = status === 'ready' && !workspace ? 'loading' : status;

  return { 
    workspace, 
    status: effectiveStatus, 
    error, 
    reload, 
    isLoading: effectiveStatus === 'loading' 
  };
}
