import { useState, useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { IpcService } from '../lib/ipc';

import { PromptResponse } from '../types/backend';

export type PromptStatus = 'idle' | 'sending' | 'success' | 'error';

interface UsePromptOptions {
  onSuccess?: () => void;
  onError?: (error: string) => void;
}

interface PromptProgress {
  percent: number;
  step: string;
}

export function usePrompt(
  workspacePath: string | undefined, 
  options?: UsePromptOptions,
  provider: string = 'opencode', 
  model: string = 'opencode/big-pickle'
) {
  const [status, setStatus] = useState<PromptStatus>('idle');
  const [progress, setProgress] = useState<number>(0);
  const [currentStep, setCurrentStep] = useState<string>('');
  const [error, setError] = useState<string | null>(null);
  const [lastResponse, setLastResponse] = useState<PromptResponse | null>(null);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    
    const setupListener = async () => {
      const unsub = await listen<PromptProgress>('prompt-progress', (event) => {
        setProgress(event.payload.percent);
        setCurrentStep(event.payload.step);
      });
      unlisten = unsub;
    };

    setupListener();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  const sendPrompt = useCallback(async (prompt: string, providerOverride?: string, modelOverride?: string) => {
    if (!workspacePath) {
      const err = 'Workspace path is required';
      setError(err);
      options?.onError?.(err);
      return;
    }

    setStatus('sending');
    setProgress(0);
    setCurrentStep('Initializing AI...');
    setError(null);

    try {
      const response = await IpcService.call<PromptResponse>('SEND_PROMPT', {
        workspacePath: workspacePath,
        prompt,
        provider: providerOverride || provider,
        model: modelOverride || model
      });
      
      if (response.canceled) {
        setLastResponse(null);
        setStatus('idle');
        setProgress(0);
        setCurrentStep('');
        setError(null);
        return;
      }

      setLastResponse(response);
      setStatus('success');
      setProgress(100);
      setCurrentStep('Completed');
      options?.onSuccess?.();
      
      setTimeout(() => {
        setStatus('idle');
        setProgress(0);
        setCurrentStep('');
      }, 2000);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      setStatus('error');
      options?.onError?.(errorMessage);
    }
  }, [workspacePath, options, provider, model]);

  const clearSession = useCallback(async () => {
    try {
      await IpcService.call<void>('CLEAR_SESSION');
      setLastResponse(null);
      console.log('[Epris] Session cleared');
    } catch (err) {
      console.error('[Epris] Failed to clear session:', err);
    }
  }, []);

  return { status, progress, currentStep, error, lastResponse, sendPrompt, clearSession };
}
