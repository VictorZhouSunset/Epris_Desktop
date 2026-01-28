import { useState, useCallback, useEffect } from 'react';
import { IpcService } from '../lib/ipc';

async function waitPort(port: number, timeout = 30000) {
  const start = Date.now();
  // For OpenCode, check global health
  const url = `http://127.0.0.1:${port}/global/health`;
  while (Date.now() - start < timeout) {
    try {
      const res = await fetch(url);
      if (res.ok) return true;
    } catch (e) {
      // Continue
    }
    await new Promise(r => setTimeout(r, 1000));
  }
  return false;
}

export type OpenCodeStatus = 'stopped' | 'starting' | 'running' | 'error';

export function useOpenCode(workspacePath: string | undefined) {
  const [status, setStatus] = useState<OpenCodeStatus>('stopped');
  const [port, setPort] = useState<number | null>(null);
  const [error, setError] = useState<Error | null>(null);
  


  const start = useCallback(async () => {
    console.log('[Epris Frontend] Starting OpenCode for:', workspacePath);
    if (!workspacePath) {
      setError(new Error('Workspace path is required'));
      return;
    }
    
    setStatus('starting');
    setError(null);
    
    try {
      const serverPort = await IpcService.call<number>('START_OPENCODE', {
        workspacePath: workspacePath,
      });
      setPort(serverPort);
      
      // WAIT for OpenCode health check
      await waitPort(serverPort);
      
      setStatus('running');
      return serverPort;
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
      setStatus('error');
      throw err;
    }
  }, [workspacePath]);

  const stop = useCallback(async () => {
    try {
      await IpcService.call<void>('STOP_OPENCODE');
      setStatus('stopped');
      setPort(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    }
  }, []);

  // Auto-start when workspace becomes available
  useEffect(() => {
    console.log('[Epris Frontend] useOpenCode effect', { workspacePath });
    if (workspacePath) {
      // Add delay to allow workspace to be ready
      const timer = setTimeout(() => {
        start().catch(console.error);
      }, 2000);
      return () => {
        clearTimeout(timer);
        stop().catch(console.error);
      };
    }
    stop().catch(console.error);
  }, [workspacePath, start, stop]);

  return { status, port, error, start, stop };
}
