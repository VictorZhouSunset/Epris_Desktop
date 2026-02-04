import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { RefreshCw, Check, AlertTriangle, Loader2, Download, X } from 'lucide-react';
import type { SttStatus } from '../types/backend';

interface EnvironmentStatus {
  node_valid: boolean;
  pnpm_valid: boolean;
  provider_cli_valid: boolean;
  workspace_deps_valid: boolean;
  skills_valid: boolean;
  missing: string[];
  details: {
    node?: { version: string; source: string };
    pnpm?: { version: string; source: string };
    provider_cli?: { version: string; source: string };
  };
}

interface FirstRunWizardProps {
  workspacePath?: string;
  provider: string; // 'opencode' | 'gemini'
  onComplete: () => void;
  onClose?: () => void;
  requestedPackages?: string[];
}

export function FirstRunWizard({ workspacePath, provider, onComplete, onClose, requestedPackages }: FirstRunWizardProps) {
  const [status, setStatus] = useState<EnvironmentStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [liveInstallLine, setLiveInstallLine] = useState<string>('');
  const [progress, setProgress] = useState<{ step: string; percent: number } | null>(null);
  const [stt, setStt] = useState<SttStatus | null>(null);
  const [sttInstalling, setSttInstalling] = useState(false);
  const [sttLogs, setSttLogs] = useState<string[]>([]);
  const [sttProgress, setSttProgress] = useState<string>('');
  const [sttProgressPercent, setSttProgressPercent] = useState<number | null>(null);
  const [sttModel, setSttModel] = useState<'tiny' | 'tiny.en' | 'small'>('tiny.en');

  const requested = (requestedPackages || []).map((s) => String(s || '').trim()).filter(Boolean);

  const onCompleteRef = useRef(onComplete);
  useEffect(() => {
    onCompleteRef.current = onComplete;
  }, [onComplete]);

  const checkEnv = useCallback(async () => {
    if (!workspacePath) return;
    setLoading(true);
    try {
      const env = await invoke<EnvironmentStatus>('get_environment_status', { 
        provider,
        workspacePath 
      });
      setStatus(env);
      
      // Auto-complete if everything is ready
      if (
        env.node_valid && 
        env.pnpm_valid && 
        env.provider_cli_valid && 
        env.workspace_deps_valid && 
        env.skills_valid
      ) {
        onCompleteRef.current();
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }, [workspacePath, provider]);

  const checkStt = useCallback(async () => {
    try {
      const s = await invoke<SttStatus>('get_stt_status');
      setStt(s);
      const model = String(s.model || '').toLowerCase();
      if (model.includes('ggml-small')) setSttModel('small');
      else if (model.includes('ggml-tiny.') && !model.includes('tiny.en')) setSttModel('tiny');
      else if (model.includes('ggml-tiny.en')) setSttModel('tiny.en');
    } catch {
      setStt({ installed: false, binary_ok: false, model_ok: false });
    }
  }, []);

  useEffect(() => {
    // Avoid re-checking while an install is running (prevents flicker/unmount issues).
    if (installing) return;
    checkEnv();
    void checkStt();
  }, [checkEnv, checkStt, installing]);

  useEffect(() => {
    // Listen for install progress
    const unlistenProgress = listen<any>('env_install_progress', (e) => {
      setProgress(e.payload);
    });
    const unlistenLogs = listen<string>('env_install_log', (e) => {
      const line = String(e.payload || '');
      // Avoid spamming the log window with heartbeat lines (keep it as a “live” status line).
      if (line.startsWith('pnpm install still running (') || line.startsWith('pnpm add still running (')) {
        setLiveInstallLine(line);
        return;
      }
      setLiveInstallLine('');
      setLogs(prev => [...prev.slice(-99), line]);
    });
    const unlistenSttLogs = listen<string>('stt_install_log', (e) => {
      setSttLogs(prev => [...prev.slice(-99), e.payload]);
    });
    const unlistenSttProgress = listen<any>('stt_install_progress', (e) => {
      const p = e.payload || {};
      const label = String(p.label || 'Downloading');
      const downloaded = typeof p.downloaded_bytes === 'number' ? p.downloaded_bytes : 0;
      const total = typeof p.total_bytes === 'number' ? p.total_bytes : null;
      const percent = typeof p.percent === 'number' ? p.percent : null;
      const mb = (n: number) => (n / (1024 * 1024)).toFixed(1);

      if (total && percent !== null && total > 0) {
        setSttProgress(`${label}: ${mb(downloaded)} / ${mb(total)} MB (${Math.round(percent)}%)`);
        setSttProgressPercent(percent);
      } else {
        setSttProgress(`${label}: ${mb(downloaded)} MB`);
        setSttProgressPercent(null);
      }
    });

    return () => {
      unlistenProgress.then(f => f());
      unlistenLogs.then(f => f());
      unlistenSttLogs.then(f => f());
      unlistenSttProgress.then(f => f());
    };
  }, []);

  const handleInstall = useCallback(async () => {
    if (!workspacePath) return;
    setInstalling(true);
    setLogs([]);
    setLiveInstallLine('');
    setError(null);
    try {
      await invoke('install_missing_dependencies', { 
        workspacePath, 
        provider 
      });
      await checkEnv();
    } catch (err) {
      setError(String(err));
    } finally {
      setInstalling(false);
      setProgress(null);
    }
  }, [workspacePath, provider, checkEnv]);

  const handleInstallRequestedPackages = useCallback(async () => {
    if (!workspacePath) return;
    if (requested.length === 0) return;
    setInstalling(true);
    setLogs([]);
    setLiveInstallLine('');
    setError(null);
    try {
      await invoke('install_js_packages', { workspacePath, packages: requested });
      await checkEnv();
    } catch (err) {
      setError(String(err));
    } finally {
      setInstalling(false);
      setProgress(null);
    }
  }, [workspacePath, requested, checkEnv]);

  const handleCancelInstall = useCallback(async () => {
    try {
      await invoke('cancel_env_install');
    } catch {}
    setInstalling(false);
    setProgress(null);
    setLiveInstallLine('');
    setError('Canceled.');
  }, []);

  const handleInstallStt = async () => {
    setSttInstalling(true);
    setSttLogs([]);
    setSttProgress('');
    setSttProgressPercent(null);
    try {
      await invoke('install_whispercpp', { model: sttModel });
      await checkStt();
    } catch (err) {
      setSttLogs(prev => [...prev, `ERROR: ${String(err)}`]);
    } finally {
      setSttInstalling(false);
    }
  };

  const isComplete = Boolean(
    status &&
      status.node_valid &&
      status.pnpm_valid &&
      status.provider_cli_valid &&
      status.workspace_deps_valid &&
      status.skills_valid,
  );

  if (isComplete) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/90 backdrop-blur-sm p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-2xl shadow-2xl w-full max-w-2xl overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="p-6 border-b border-slate-800 bg-slate-900 flex items-start justify-between gap-4">
          <div>
          <h2 className="text-xl font-bold text-white flex items-center gap-2">
            <Download className="text-indigo-400" />
            Environment Setup
          </h2>
          <p className="text-slate-400 text-sm mt-1">
            We need to install some local dependencies for {provider} and Remotion.
          </p>
          </div>
          {onClose && (
            <button
              onClick={async () => {
                if (!installing) {
                  onClose();
                  return;
                }

                const ok = confirm(
                  "Installation is still running.\n\nCancel the installation and close this window?\n\n(You can restart setup later from the main screen.)",
                );
                if (!ok) return;

                await handleCancelInstall();
                onClose();
              }}
              className="p-2 rounded-xl hover:bg-slate-800/60 text-slate-400 hover:text-slate-200 transition-colors"
              title="Close"
            >
              <X size={18} />
            </button>
          )}
        </div>

        {/* Status List */}
        <div className="p-6 flex-1 overflow-y-auto">
          {loading && (
            <div className="flex items-center gap-2 text-slate-400 text-sm mb-4">
              <Loader2 className="animate-spin" />
              Checking environment...
            </div>
          )}

          {!status && !loading && (
            <div className="text-slate-400 text-sm">
              No environment status available.
            </div>
          )}

          {status && (
          <div className="space-y-4">
            <StatusItem label="Node.js Check" passed={status.node_valid} detail={status.details.node?.version} />
            <StatusItem label="pnpm Check" passed={status.pnpm_valid} detail={status.details.pnpm?.version} />
            <StatusItem label={`${provider} CLI`} passed={status.provider_cli_valid} detail={status.details.provider_cli?.version} />
            <StatusItem label="Workspace Dependencies" passed={status.workspace_deps_valid} />
            {requested.length > 0 && (
              <StatusItem
                label="Effect Packages"
                passed={false}
                detail={`${requested.length} missing`}
              />
            )}
            <StatusItem label="Remotion Skills" passed={status.skills_valid} />
            <StatusItem label="Voice-to-Text (whisper.cpp) (Optional)" passed={Boolean(stt?.installed)} detail={stt?.model} />
          </div>
          )}

          {error && (
            <div className="mt-6 p-4 bg-red-500/10 border border-red-500/20 rounded-lg text-red-400 text-sm">
              <AlertTriangle className="inline-block mr-2 w-4 h-4" />
              {error}
            </div>
          )}

          {installing && (
            <div className="mt-6 space-y-4">
              {progress && (
                <div className="space-y-2">
                  <div className="flex justify-between text-xs text-slate-400 uppercase font-bold tracking-wider">
                    <span>{progress.step}</span>
                    <span>{Math.round(progress.percent)}%</span>
                  </div>
                  <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                    <div 
                      className="h-full bg-indigo-500 transition-all duration-300 ease-out" 
                      style={{ width: `${progress.percent}%` }}
                    />
                  </div>
                </div>
              )}

              {liveInstallLine && (
                <div className="text-xs text-slate-400 font-mono bg-slate-950/40 border border-slate-800 rounded-lg px-3 py-2">
                  {liveInstallLine}
                </div>
              )}
              
              <div className="bg-slate-950 rounded-lg p-3 font-mono text-xs text-slate-400 h-32 overflow-y-auto border border-slate-800">
                {logs.length === 0 ? (
                  <span className="opacity-50 italic">Waiting for logs...</span>
                ) : (
                  logs.map((line, i) => (
                    <div key={i} className="whitespace-pre-wrap font-mono">{line}</div>
                  ))
                )}
              </div>

              <div className="flex items-center justify-end gap-3">
                <button
                  onClick={handleCancelInstall}
                  className="px-4 py-2 rounded-xl bg-slate-900/50 border border-slate-700 text-slate-200 font-bold hover:bg-slate-900 transition-colors"
                  title="Stop installation"
                >
                  Cancel install
                </button>
              </div>
            </div>
          )}

          {!installing && requested.length > 0 && (
            <div className="mt-6 p-4 bg-slate-950/40 border border-slate-800 rounded-xl">
              <div className="text-slate-200 font-bold">Effect packages needed by the last prompt</div>
              <div className="text-xs text-slate-500 mt-1">
                The last generated code imported extra packages that are not installed in this project yet.
              </div>
              <div className="mt-3 max-h-32 overflow-y-auto rounded-lg bg-slate-950 border border-slate-800 p-3 text-xs text-slate-200 font-mono">
                {requested.join('\n')}
              </div>
              <div className="mt-4 flex items-center justify-end gap-3">
                <button
                  onClick={handleInstallRequestedPackages}
                  className="px-4 py-2 rounded-xl bg-indigo-600 text-white font-black hover:bg-indigo-500 transition-colors"
                  title="Install the extra packages required by the generated code"
                >
                  Install effect packages
                </button>
              </div>
            </div>
          )}

          {/* Optional STT installer */}
          <div className="mt-8 p-4 bg-slate-950/40 border border-slate-800 rounded-xl">
            <div className="flex items-center justify-between gap-4">
              <div>
                <div className="text-slate-200 font-bold">Voice-to-Text (Optional)</div>
                <div className="text-xs text-slate-500 mt-1">
                  Installs whisper.cpp + a small model for offline transcription (CPU-only).
                </div>
              </div>
              <div className="flex items-center gap-2">
                <select
                  value={sttModel}
                  onChange={(e) => setSttModel(e.target.value as any)}
                  className="bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-200"
                  disabled={sttInstalling}
                  title="Model"
                >
                  <option value="tiny">tiny (Multilingual, fastest)</option>
                  <option value="tiny.en">tiny.en (English)</option>
                  <option value="small">small (Multilingual)</option>
                </select>
                <button
                  onClick={handleInstallStt}
                  disabled={sttInstalling}
                  className="px-4 py-2 bg-slate-800 hover:bg-slate-700 disabled:opacity-50 text-slate-200 rounded-lg font-bold transition-colors flex items-center gap-2"
                >
                  {sttInstalling ? (
                    <>
                      <RefreshCw className="animate-spin w-4 h-4" />
                      Installing...
                    </>
                  ) : (
                    <>
                      <Download className="w-4 h-4" />
                      Install
                    </>
                  )}
                </button>
              </div>
	            </div>
	            {sttInstalling && sttProgress && (
	              <div className="mt-3 space-y-2">
                  {sttProgressPercent !== null && (
                    <div className="h-2 bg-slate-800 rounded-full overflow-hidden">
                      <div
                        className="h-full bg-indigo-500 transition-all duration-300 ease-out"
                        style={{ width: `${Math.max(0, Math.min(100, sttProgressPercent))}%` }}
                      />
                    </div>
                  )}
                  <div className="text-xs text-slate-300 font-mono">{sttProgress}</div>
                </div>
	            )}
	            {sttLogs.length > 0 && (
	              <div className="mt-4 bg-slate-950 rounded-lg p-3 font-mono text-xs text-slate-400 max-h-28 overflow-y-auto border border-slate-800">
	                {sttLogs.map((line, i) => (
	                  <div key={i} className="whitespace-pre-wrap font-mono">{line}</div>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-slate-800 bg-slate-900/50 flex justify-end gap-3">
          <button
            onClick={checkEnv}
            className="px-4 py-2 hover:bg-slate-800 rounded-lg text-slate-400 text-sm font-medium transition-colors"
            disabled={installing}
          >
            Re-check
          </button>
          <button
            onClick={handleInstall}
            disabled={installing}
            className="px-6 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white rounded-lg font-bold shadow-lg shadow-indigo-500/20 transition-all flex items-center gap-2"
          >
            {installing ? (
              <>
                <RefreshCw className="animate-spin w-4 h-4" />
                Installing...
              </>
            ) : (
              <>
                <Download className="w-4 h-4" />
                Install Missing Dependencies
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}

function StatusItem({ label, passed, detail }: { label: string, passed: boolean, detail?: string }) {
  return (
    <div className="flex items-center justify-between p-3 bg-slate-800/50 rounded-lg border border-slate-700/50">
      <div className="flex items-center gap-3">
        <div className={`w-6 h-6 rounded-full flex items-center justify-center ${passed ? 'bg-emerald-500/20 text-emerald-400' : 'bg-slate-700 text-slate-500'}`}>
          {passed ? <Check size={14} /> : <div className="w-2 h-2 rounded-full bg-slate-500" />}
        </div>
        <span className={passed ? 'text-slate-200' : 'text-slate-400'}>{label}</span>
      </div>
      {detail ? (
        <span className="text-xs font-mono text-slate-500 bg-slate-900 px-2 py-1 rounded border border-slate-800">
          {detail}
        </span>
      ) : (
        <span className={`text-xs font-medium ${passed ? 'text-emerald-500' : 'text-amber-500'}`}>
          {passed ? 'READY' : 'MISSING'}
        </span>
      )}
    </div>
  );
}
