import { useState, useCallback, useEffect } from 'react';
import { IpcService } from '../lib/ipc';

async function waitPort(port: number, timeout = 10000) {
  const start = Date.now();
  const url = `http://127.0.0.1:${port}`;
  while (Date.now() - start < timeout) {
    try {
      const res = await fetch(url, { mode: 'no-cors' });
      if (res.status !== 0 || res.type === 'opaque') return true;
    } catch (e) {
      // Continue
    }
    await new Promise(r => setTimeout(r, 500));
  }
  return false;
}

export type PreviewServerStatus = 'stopped' | 'starting' | 'running' | 'error';

export function usePreviewServer(workspacePath: string | undefined) {
  const [status, setStatus] = useState<PreviewServerStatus>('stopped');
  const [port, setPort] = useState<number | null>(null);
  const [error, setError] = useState<Error | null>(null);
  


  const start = useCallback(async () => {
    console.log('[Epris Frontend] Starting preview server for:', workspacePath);
    if (!workspacePath) {
      setError(new Error('Workspace path is required'));
      return;
    }
    
    setStatus('starting');
    setError(null);
    
    try {
      const serverPort = await IpcService.call<number>('START_PREVIEW_SERVER', {
        workspacePath: workspacePath,
      });
      setPort(serverPort);
      
      // WAIT for port to be ready
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
      await IpcService.call<void>('STOP_PREVIEW_SERVER');
      setStatus('stopped');
      setPort(null);
    } catch (err) {
      setError(err instanceof Error ? err : new Error(String(err)));
    }
  }, []);

  // Auto-start when workspace becomes available
  useEffect(() => {
    console.log('[Epris Frontend] usePreviewServer effect', { workspacePath });
    if (workspacePath) {
      // Add delay to allow server to fully initialize
      const timer = setTimeout(() => {
        start().catch(console.error);
      }, 1000);
      return () => clearTimeout(timer);
    }
  }, [workspacePath, start]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      stop().catch(console.error);
    };
  }, [stop]);

  return { status, port, error, start, stop };
}
