import { useState, useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { openPath } from '@tauri-apps/plugin-opener';
import { IpcService } from '../lib/ipc';
import { ExportResult } from '../types/backend';

interface ExportProgress {
  percent: number;
}

export function useExport(workspacePath?: string) {
  const [status, setStatus] = useState<'idle' | 'exporting' | 'done' | 'error'>('idle');
  const [progress, setProgress] = useState<number>(0);
  const [outputPath, setOutputPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    
    const setupListener = async () => {
      const unsub = await listen<ExportProgress>('export-progress', (event) => {
        setProgress(event.payload.percent);
      });
      unlisten = unsub;
    };

    setupListener();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const exportVideo = useCallback(async () => {
    if (!workspacePath) {
      setError('No workspace path');
      setStatus('error');
      return;
    }

    setStatus('exporting');
    setProgress(0);
    setError(null);
    setOutputPath(null);

    try {
      const result = await IpcService.call<ExportResult>('EXPORT_VIDEO', {
        workspacePath,
      });

      if (result.success && result.output_path) {
        setOutputPath(result.output_path);
        setStatus('done');
        setProgress(100);
      } else {
        setError(result.error || 'Export failed');
        setStatus('error');
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setStatus('error');
    }
  }, [workspacePath]);

  const openOutputFolder = useCallback(async () => {
    if (!outputPath) return;
    
    try {
      // Open the folder containing the video using Tauri opener plugin
      // Handle both Windows (\) and Unix (/) path separators
      const lastBackslash = outputPath.lastIndexOf('\\');
      const lastForwardSlash = outputPath.lastIndexOf('/');
      const lastSeparator = Math.max(lastBackslash, lastForwardSlash);
      
      if (lastSeparator === -1) {
        console.error('Invalid path - no separator found');
        return;
      }
      
      const folderPath = outputPath.substring(0, lastSeparator);
      console.log('Opening folder:', folderPath);
      await openPath(folderPath);
      // Reset to idle to allow fresh export
      setStatus('idle');
      setOutputPath(null);
      setProgress(0);
    } catch (err) {
      console.error('Failed to open folder:', err);
      alert('Failed to open folder: ' + err);
    }
  }, [outputPath]);

  return {
    status,
    progress,
    outputPath,
    error,
    exportVideo,
    openOutputFolder,
  };
}
