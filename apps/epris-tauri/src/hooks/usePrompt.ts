import { useState, useCallback, useEffect } from 'react';
import { listen } from '@tauri-apps/api/event';
import { IpcService } from '../lib/ipc';

import { AgentPlanState, PromptResponse } from '../types/backend';

export type PromptStatus = 'idle' | 'sending' | 'success' | 'error';

interface UsePromptOptions {
  onSuccess?: () => void;
  onError?: (error: string) => void;
}

interface PromptProgress {
  percent: number;
  step: string;
}

function normalizePathForCompare(path: string): string {
  return path
    .replace(/\\/g, '/')
    .replace(/\/+$/g, '')
    .toLowerCase();
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
  const [planState, setPlanState] = useState<AgentPlanState | null>(null);

  useEffect(() => {
    let unlistenProgress: (() => void) | undefined;
    let unlistenPlan: (() => void) | undefined;
    
    const setupListener = async () => {
      const unsubProgress = await listen<PromptProgress>('prompt-progress', (event) => {
        setProgress(event.payload.percent);
        setCurrentStep(event.payload.step);
      });
      const unsubPlan = await listen<AgentPlanState>('agent-plan', (event) => {
        const payload = event.payload;
        if (!workspacePath) return;
        const wsNorm = normalizePathForCompare(workspacePath);
        const planPathNorm = normalizePathForCompare(payload?.path || '');
        if (!planPathNorm) return;
        if (!(planPathNorm === wsNorm || planPathNorm.startsWith(`${wsNorm}/`))) return;
        setPlanState(payload);
      });
      unlistenProgress = unsubProgress;
      unlistenPlan = unsubPlan;
    };

    setupListener();
    return () => {
      if (unlistenProgress) unlistenProgress();
      if (unlistenPlan) unlistenPlan();
    };
  }, [workspacePath]);

  useEffect(() => {
    if (!workspacePath) {
      setPlanState(null);
      return;
    }
    IpcService.call<AgentPlanState>('GET_AGENT_PLAN_STATE', { workspacePath })
      .then((state) => setPlanState(state))
      .catch(() => {});
  }, [workspacePath]);

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
    IpcService.call<AgentPlanState>('GET_AGENT_PLAN_STATE', { workspacePath })
      .then((state) => setPlanState(state))
      .catch(() => {});

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

  const revalidateGate = useCallback(async () => {
    if (!workspacePath) {
      const err = 'Workspace path is required';
      setError(err);
      options?.onError?.(err);
      return null;
    }

    setStatus('sending');
    setProgress(0);
    setCurrentStep('Gate: Re-validating...');
    setError(null);

    try {
      const gate = await IpcService.call<any>('RUN_GATE', { workspacePath });

      setLastResponse((prev) => {
        if (prev) {
          return { ...prev, gate_result: gate, success: Boolean(gate?.passed) };
        }
        return {
          success: Boolean(gate?.passed),
          message: gate?.passed ? 'Gate passed' : 'Gate failed',
          gate_result: gate,
        };
      });

      setStatus('success');
      setProgress(100);
      setCurrentStep('Completed');
      if (gate?.passed) options?.onSuccess?.();

      setTimeout(() => {
        setStatus('idle');
        setProgress(0);
        setCurrentStep('');
      }, 1200);

      return gate;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : String(err);
      setError(errorMessage);
      setStatus('error');
      options?.onError?.(errorMessage);
      return null;
    }
  }, [workspacePath, options]);

  return { status, progress, currentStep, error, lastResponse, planState, sendPrompt, clearSession, revalidateGate };
}
