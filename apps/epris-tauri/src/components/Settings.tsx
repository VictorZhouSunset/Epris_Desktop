import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { X, Cpu, Key, LogIn, CheckCircle2, AlertCircle, Download, RefreshCw, Terminal, FolderOpen } from 'lucide-react';
import { IpcService } from '../lib/ipc';
import type { SttStatus } from '../types/backend';

interface SettingsProps {
  onClose: () => void;
  currentProvider: string;
  onProviderChange: (provider: string) => void;
}

interface EnvStatus {
  node_valid: boolean;
  pnpm_valid: boolean;
  provider_cli_valid: boolean;
  missing: string[];
  details: {
    node?: { version: string; source: string };
    pnpm?: { version: string; source: string };
    provider_cli?: { version: string; source: string };
  };
}

const PROVIDERS = [
  { id: 'opencode', name: 'OpenCode (Local)' },
  { id: 'gemini', name: 'Google Gemini' }
];

export function Settings({ onClose, currentProvider, onProviderChange }: SettingsProps) {
  const [apiKeyStatus, setApiKeyStatus] = useState<'checking' | 'valid' | 'invalid'>('checking');
  const [geminiKey, setGeminiKey] = useState('');
  
  const [envStatus, setEnvStatus] = useState<EnvStatus | null>(null);
  const [checkingEnv, setCheckingEnv] = useState(false);
  const [installing, setInstalling] = useState(false);

  const [projectsRoot, setProjectsRoot] = useState<string>('');
  const [movingProjects, setMovingProjects] = useState(false);

  const [stt, setStt] = useState<SttStatus | null>(null);
  const [sttInstalling, setSttInstalling] = useState(false);
  const [sttModel, setSttModel] = useState<'tiny' | 'tiny.en' | 'small'>('tiny.en');
  const [sttTask, setSttTask] = useState<'transcribe' | 'translate'>('transcribe');
  const [sttLogs, setSttLogs] = useState<string[]>([]);
  const [sttProgress, setSttProgress] = useState<string>('');

  useEffect(() => {
    const saved = window.localStorage.getItem('epris_stt_model_selection');
    if (saved === 'tiny' || saved === 'tiny.en' || saved === 'small') {
      setSttModel(saved as 'tiny' | 'tiny.en' | 'small');
    }
    const savedTask = window.localStorage.getItem('epris_stt_task_selection');
    if (savedTask === 'transcribe' || savedTask === 'translate') {
      setSttTask(savedTask);
    }
  }, []);

  useEffect(() => {
    if (currentProvider === 'gemini') {
      checkAuth();
    }
    checkEnvironment();
    checkProjectsRoot();
    checkStt();
  }, [currentProvider]);

  useEffect(() => {
    const unlisten = listen<string>('stt_install_log', (e) => {
      setSttLogs((prev) => [...prev.slice(-99), e.payload]);
    });
    const unlistenProgress = listen<any>('stt_install_progress', (e) => {
      const p = e.payload || {};
      const label = String(p.label || 'Downloading');
      const downloaded = typeof p.downloaded_bytes === 'number' ? p.downloaded_bytes : 0;
      const total = typeof p.total_bytes === 'number' ? p.total_bytes : null;
      const percent = typeof p.percent === 'number' ? p.percent : null;
      const mb = (n: number) => (n / (1024 * 1024)).toFixed(1);

      if (total && percent !== null && total > 0) {
        setSttProgress(`${label}: ${mb(downloaded)} / ${mb(total)} MB (${Math.round(percent)}%)`);
      } else {
        setSttProgress(`${label}: ${mb(downloaded)} MB`);
      }
    });
    return () => {
      unlisten.then((f) => f());
      unlistenProgress.then((f) => f());
    };
  }, []);

  const checkAuth = async () => {
    setApiKeyStatus('checking');
    try {
      const isValid = await IpcService.call<boolean>('GET_GEMINI_AUTH_STATUS');
      setApiKeyStatus(isValid ? 'valid' : 'invalid');
    } catch (e) {
      console.error('Failed to check auth status:', e);
      setApiKeyStatus('invalid');
    }
  };

  const checkEnvironment = async () => {
    setCheckingEnv(true);
    try {
      const status = await IpcService.call<EnvStatus>('GET_ENVIRONMENT_STATUS', { provider: currentProvider });
      setEnvStatus(status);
    } catch (e) {
      console.error('Failed to check environment:', e);
    } finally {
      setCheckingEnv(false);
    }
  };

  const checkProjectsRoot = async () => {
    try {
      const overview = await IpcService.call<any>('GET_PROJECTS_OVERVIEW');
      if (overview?.projects_root) {
        setProjectsRoot(overview.projects_root);
      } else {
        const def = await IpcService.call<string>('GET_DEFAULT_PROJECTS_ROOT');
        setProjectsRoot(def);
      }
    } catch (e) {
      console.error('Failed to check projects root:', e);
    }
  };

  const checkStt = async () => {
    try {
      const s = await invoke<SttStatus>('get_stt_status');
      setStt(s);
      const saved = window.localStorage.getItem('epris_stt_model_selection');
      if (saved === 'tiny' || saved === 'tiny.en' || saved === 'small') {
        setSttModel(saved as 'tiny' | 'tiny.en' | 'small');
      } else {
        const model = String(s.model || '').toLowerCase();
        if (model.includes('ggml-small')) setSttModel('small');
        else if (model.includes('ggml-tiny.') && !model.includes('tiny.en')) setSttModel('tiny');
        else if (model.includes('ggml-tiny.en')) setSttModel('tiny.en');
      }
      const savedTask = window.localStorage.getItem('epris_stt_task_selection');
      if (savedTask === 'transcribe' || savedTask === 'translate') {
        setSttTask(savedTask);
      } else {
        const t = String(s.task || '').toLowerCase();
        if (t === 'translate') setSttTask('translate');
        else setSttTask('transcribe');
      }
    } catch {
      setStt({ installed: false, binary_ok: false, model_ok: false });
    }
  };

  const handleInstallStt = async () => {
    setSttInstalling(true);
    setSttLogs([]);
    setSttProgress('');
    try {
      const s = await invoke<SttStatus>('install_whispercpp', { model: sttModel });
      setStt(s);
      alert('Voice-to-Text installed.');
    } catch (e) {
      alert('Voice-to-Text installation failed: ' + (e instanceof Error ? e.message : String(e)));
    } finally {
      setSttInstalling(false);
    }
  };

  const handleUseSttModel = async () => {
    setSttInstalling(true);
    setSttLogs([]);
    setSttProgress('');
    try {
      const s = await invoke<SttStatus>('set_stt_model', { model: sttModel });
      setStt(s);
      alert('Voice-to-Text model switched.');
    } catch (e) {
      alert('Failed to switch model: ' + (e instanceof Error ? e.message : String(e)));
    } finally {
      setSttInstalling(false);
    }
  };

  const applySttTask = async (task: 'transcribe' | 'translate', silent?: boolean) => {
    setSttInstalling(true);
    setSttLogs([]);
    setSttProgress('');
    try {
      const s = await invoke<SttStatus>('set_stt_task', { task });
      setStt(s);
      if (!silent) alert('Voice-to-Text output mode updated.');
    } catch (e) {
      if (!silent) alert('Failed to update output mode: ' + (e instanceof Error ? e.message : String(e)));
      console.error('Failed to update STT task:', e);
    } finally {
      setSttInstalling(false);
    }
  };

  const handleChangeProjectsRoot = async () => {
    try {
      const picked = await IpcService.call<string | null>('PICK_PROJECTS_ROOT', { initial: projectsRoot });
      if (!picked) return;

      if (
        !confirm(
          `Move all projects to this folder?\n\nFrom:\n${projectsRoot}\n\nTo:\n${picked}\n\nThis will move folders on disk.`,
        )
      ) {
        return;
      }

      setMovingProjects(true);
      await IpcService.call('SET_PROJECTS_ROOT', { newRoot: picked, moveExisting: true });
      setProjectsRoot(picked);
      alert('Projects folder updated.');
    } catch (e) {
      alert('Failed to change projects folder: ' + (e instanceof Error ? e.message : String(e)));
    } finally {
      setMovingProjects(false);
    }
  };

  const handleInstallDependencies = async () => {
    setInstalling(true);
    try {
      await IpcService.call('INSTALL_MISSING_DEPENDENCIES');
      await checkEnvironment();
      alert('Installation complete!'); // Simplified feedback
    } catch (e) {
      alert('Installation failed: ' + e);
    } finally {
      setInstalling(false);
    }
  };

  const handleLogin = async () => {
    try {
      await invoke('open_gemini_auth_terminal');
      // Poll for auth status
      const interval = setInterval(async () => {
        const isValid = await IpcService.call<boolean>('GET_GEMINI_AUTH_STATUS');
        if (isValid) {
          setApiKeyStatus('valid');
          clearInterval(interval);
        }
      }, 5000);
    } catch (e) {
      console.error('Failed to open terminal:', e);
    }
  };

  const saveKey = async () => {
    if (!geminiKey.trim()) return;
    try {
      await IpcService.call('SET_GEMINI_API_KEY', { key: geminiKey });
      setGeminiKey('');
      checkAuth();
    } catch (e) {
      alert('Failed to save key: ' + e);
    }
  };

  return (
    <div className="fixed inset-0 z-[100] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-slate-950/80 backdrop-blur-md" onClick={onClose} />
      <div className="relative w-full max-w-lg bg-slate-900 border border-slate-700 rounded-3xl p-8 shadow-3xl flex flex-col gap-6 max-h-[90vh] overflow-y-auto">
        
        <header className="flex items-center justify-between">
          <h2 className="text-2xl font-bold text-white flex items-center gap-2">
            <Cpu className="text-indigo-400" />
            AI Settings
          </h2>
          <button onClick={onClose} className="p-2 hover:bg-slate-800 rounded-full text-slate-400 hover:text-white transition-colors">
            <X size={24} />
          </button>
        </header>

        <div className="space-y-6">
          {/* Projects Root */}
          <div className="p-4 bg-slate-800/30 rounded-xl border border-slate-700 space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-slate-300">Projects Folder</span>
              <button
                onClick={handleChangeProjectsRoot}
                className="px-3 py-2 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-xs font-bold text-slate-200 transition-colors flex items-center gap-2 disabled:opacity-50"
                disabled={movingProjects}
                title="Change projects folder"
              >
                <FolderOpen size={14} />
                {movingProjects ? 'Moving…' : 'Change'}
              </button>
            </div>
            <div className="text-[11px] text-slate-500 break-all">{projectsRoot || '(not set)'}</div>
            <div className="text-[11px] text-slate-500">
              Changing this will move existing project folders to the new destination.
            </div>
          </div>

          {/* Provider Selection */}
          <div className="space-y-3">
            <label className="text-xs uppercase font-black text-slate-500 tracking-widest">AI Provider</label>
            <div className="grid grid-cols-2 gap-3">
              {PROVIDERS.map(p => (
                <button
                  key={p.id}
                  onClick={() => {
                    onProviderChange(p.id);
                  }}
                  className={`p-4 rounded-xl border-2 text-left transition-all ${
                    currentProvider === p.id 
                      ? 'bg-indigo-600/20 border-indigo-500 text-white' 
                      : 'bg-slate-950 border-slate-800 text-slate-400 hover:border-slate-700'
                  }`}
                >
                  <div className="font-bold text-sm">{p.name}</div>
                </button>
              ))}
            </div>
          </div>

          {/* Environment Check */}
          <div className="p-4 bg-slate-800/30 rounded-xl border border-slate-700 space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-slate-300">Environment Status</span>
              {checkingEnv ? (
                <RefreshCw size={14} className="animate-spin text-indigo-400" />
              ) : envStatus?.missing?.length === 0 ? (
                 <span className="flex items-center gap-1.5 text-xs font-bold text-emerald-400">
                    <CheckCircle2 size={14} /> Ready
                 </span>
              ) : (
                 <span className="flex items-center gap-1.5 text-xs font-bold text-amber-400">
                    <AlertCircle size={14} /> Missing Dependencies
                 </span>
              )}
            </div>
            
            {/* Dependency List */}
            {envStatus && (
              <div className="grid grid-cols-2 gap-2 text-xs">
                 <div className={`flex flex-col gap-1 px-3 py-2 rounded-lg border ${envStatus.node_valid ? 'bg-emerald-500/10 border-emerald-500/20 text-emerald-300' : 'bg-red-500/10 border-red-500/20 text-red-300'}`}>
                    <div className="flex items-center gap-2">
                       <div className={`w-2 h-2 rounded-full ${envStatus.node_valid ? 'bg-emerald-400' : 'bg-red-400'}`} />
                       Node.js
                    </div>
                    {envStatus.details.node && (
                       <span className="text-[10px] opacity-70 ml-4 font-mono">{envStatus.details.node.version}</span>
                    )}
                 </div>
                 <div className={`flex flex-col gap-1 px-3 py-2 rounded-lg border ${envStatus.pnpm_valid ? 'bg-emerald-500/10 border-emerald-500/20 text-emerald-300' : 'bg-red-500/10 border-red-500/20 text-red-300'}`}>
                    <div className="flex items-center gap-2">
                       <div className={`w-2 h-2 rounded-full ${envStatus.pnpm_valid ? 'bg-emerald-400' : 'bg-red-400'}`} />
                       pnpm
                    </div>
                    {envStatus.details.pnpm && (
                       <span className="text-[10px] opacity-70 ml-4 font-mono">{envStatus.details.pnpm.version}</span>
                    )}
                 </div>
                 <div className={`col-span-2 flex items-center justify-between px-3 py-2 rounded-lg border ${envStatus.provider_cli_valid ? 'bg-emerald-500/10 border-emerald-500/20 text-emerald-300' : 'bg-red-500/10 border-red-500/20 text-red-300'}`}>
                    <div className="flex items-center gap-2">
                       <div className={`w-2 h-2 rounded-full ${envStatus.provider_cli_valid ? 'bg-emerald-400' : 'bg-red-400'}`} />
                       CLI ({currentProvider})
                    </div>
                    {envStatus.details.provider_cli && (
                       <span className="text-[10px] opacity-70 font-mono italic">{envStatus.details.provider_cli.version}</span>
                    )}
                 </div>
              </div>
            )}

            {envStatus && envStatus.missing.length > 0 && (
               <button 
                  onClick={handleInstallDependencies}
                  disabled={installing}
                  className="w-full py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-xs font-bold flex items-center justify-center gap-2 disabled:opacity-50 transition-all"
               >
                 {installing ? <RefreshCw size={14} className="animate-spin" /> : <Download size={14} />}
                 {installing ? 'Installing to Toolchain...' : 'Install Missing Dependencies'}
               </button>
            )}
          </div>

          {/* Voice-to-Text (Optional) */}
          <div className="p-4 bg-slate-800/30 rounded-xl border border-slate-700 space-y-3">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-slate-300">Voice-to-Text (Optional)</span>
              {stt?.installed ? (
                <span className="flex items-center gap-1.5 text-xs font-bold text-emerald-400">
                  <CheckCircle2 size={14} /> Installed
                </span>
              ) : (
                <span className="flex items-center gap-1.5 text-xs font-bold text-slate-400">
                  <AlertCircle size={14} /> Not installed
                </span>
              )}
            </div>

            <div className="text-[11px] text-slate-500">
              Installs whisper.cpp + a small model for offline transcription (Windows only).
            </div>

            <div className="flex items-center gap-2">
              <select
                className="flex-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs text-slate-200"
                value={sttModel}
                onChange={(e) => {
                  const v = e.target.value as 'tiny' | 'tiny.en' | 'small';
                  setSttModel(v);
                  window.localStorage.setItem('epris_stt_model_selection', v);
                }}
                disabled={sttInstalling}
              >
                <option value="tiny">tiny (multilingual, fastest)</option>
                <option value="tiny.en">tiny.en (English-only)</option>
                <option value="small">small (multilingual, better accuracy)</option>
              </select>
              <button
                onClick={handleUseSttModel}
                className="px-3 py-2 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-xs font-bold text-slate-200 transition-colors flex items-center gap-2 disabled:opacity-50"
                disabled={sttInstalling}
                title="Use selected model (no download)"
              >
                <CheckCircle2 size={14} />
                Use
              </button>
              <button
                onClick={handleInstallStt}
                className="px-3 py-2 bg-indigo-600 hover:bg-indigo-500 rounded-lg text-xs font-bold text-white transition-colors flex items-center gap-2 disabled:opacity-50"
                disabled={sttInstalling}
              >
                <Download size={14} />
                {sttInstalling ? 'Installing…' : 'Install'}
              </button>
            </div>

            <div className="flex items-center gap-2">
              <select
                className="flex-1 px-3 py-2 bg-slate-950 border border-slate-800 rounded-lg text-xs text-slate-200"
                value={sttTask}
                onChange={(e) => {
                  const v = e.target.value as 'transcribe' | 'translate';
                  setSttTask(v);
                  window.localStorage.setItem('epris_stt_task_selection', v);
                  // Apply immediately to avoid confusion (best-effort, no modal alerts).
                  void applySttTask(v, true);
                }}
                disabled={sttInstalling}
              >
                <option value="transcribe">Transcribe (keep original language)</option>
                <option value="translate">Translate to English</option>
              </select>
              <button
                onClick={() => void applySttTask(sttTask)}
                className="px-3 py-2 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-xs font-bold text-slate-200 transition-colors flex items-center gap-2 disabled:opacity-50"
                disabled={sttInstalling}
                title="Apply output mode"
              >
                <CheckCircle2 size={14} />
                Apply
              </button>
            </div>

            {!!stt?.model && (
              <div className="text-[11px] text-slate-500 break-all">Model: {stt.model}</div>
            )}
            {!!stt?.model && sttModel && (
              <div className="text-[11px] text-slate-500">
                Selected: {sttModel} {sttModel === 'tiny.en' ? '(English-only)' : '(multilingual)'}
              </div>
            )}
            {!!stt?.task && (
              <div className="text-[11px] text-slate-500">Output: {stt.task}</div>
            )}

            {sttInstalling && sttProgress && (
              <div className="text-[11px] text-slate-300 font-mono">{sttProgress}</div>
            )}

            {sttLogs.length > 0 && (
              <div className="bg-slate-950/60 border border-slate-800 rounded-lg p-2 text-[11px] text-slate-300 font-mono max-h-32 overflow-auto whitespace-pre-wrap">
                {sttLogs.slice(-20).join('\n')}
              </div>
            )}
          </div>



          {/* Gemini Auth Logic */}
          {currentProvider === 'gemini' && (
            <div className="p-4 bg-slate-800/50 rounded-xl border border-slate-700 space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-sm font-medium text-slate-300">Authentication Status</span>
                {apiKeyStatus === 'valid' ? (
                  <span className="flex items-center gap-1.5 text-xs font-bold text-emerald-400 bg-emerald-400/10 px-2 py-1 rounded">
                    <CheckCircle2 size={14} /> Logged In
                  </span>
                ) : (
                  <span className="flex items-center gap-1.5 text-xs font-bold text-amber-400 bg-amber-400/10 px-2 py-1 rounded">
                    <AlertCircle size={14} /> Not Logged In
                  </span>
                )}
              </div>

              {apiKeyStatus !== 'valid' && (
                <div className="space-y-3">
                  <p className="text-xs text-slate-400 leading-relaxed">
                    Choose your authentication method:
                  </p>
                  
                  <div className="grid grid-cols-1 gap-4">
                     {/* Subscription Option (Primary) */}
                     <div className="p-4 bg-gradient-to-br from-indigo-900/40 to-slate-900 rounded-xl border border-indigo-500/30 shadow-lg relative overflow-hidden">
                        <div className="absolute top-0 right-0 p-2 opacity-10 pointer-events-none">
                           <div className="w-16 h-16 rounded-full bg-indigo-400 blur-2xl"></div>
                        </div>
                        
                        <div className="flex items-center justify-between mb-3">
                           <div className="flex items-center gap-2">
                              <div className="w-5 h-5 rounded-full bg-gradient-to-r from-blue-500 to-purple-500 flex items-center justify-center text-[10px] font-bold text-white shadow-md">★</div>
                              <span className="text-sm font-bold text-white">Gemini Advanced (Subscription)</span>
                           </div>
                           <span className="text-[10px] font-bold bg-indigo-500/20 text-indigo-300 px-2 py-0.5 rounded border border-indigo-500/20">RECOMMENDED</span>
                        </div>
                        
                        <p className="text-xs text-slate-400 mb-4 leading-relaxed">
                           Use your existing Google subscription. This requires authenticating via the system terminal.
                        </p>
                        
                        <button 
                           onClick={handleLogin}
                           className="w-full py-3 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-sm font-bold flex items-center justify-center gap-2 transition-all shadow-lg shadow-indigo-900/20 active:scale-95"
                        >
                           <Terminal size={16} />
                           Authenticate via Terminal
                        </button>
                        
                        <p className="text-[10px] text-slate-500 mt-3 text-center">
                           A new window will open. Follow the instructions to log in.
                        </p>
                     </div>

                     {/* API Key Option (Fallback) */}
                     <div className="p-4 bg-slate-950/30 rounded-xl border border-slate-800/50">
                        <div className="flex items-center gap-2 mb-3">
                           <Key size={14} className="text-slate-500" />
                           <span className="text-xs font-bold text-slate-400">API Key (Fallback)</span>
                        </div>
                        <div className="flex gap-2">
                            <input 
                              type="password" 
                              value={geminiKey}
                              onChange={(e) => setGeminiKey(e.target.value)}
                              placeholder="Paste API Key..."
                              className="flex-1 bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-xs text-white outline-none focus:border-indigo-500 transition-colors"
                            />
                            <button 
                              onClick={saveKey}
                              disabled={!geminiKey}
                              className="px-4 bg-slate-700 hover:bg-slate-600 text-white rounded-lg text-xs font-bold disabled:opacity-50 transition-colors"
                            >
                              Save
                            </button>
                        </div>
                        <button 
                           onClick={handleLogin}
                           className="mt-3 text-[10px] text-slate-500 hover:text-slate-300 flex items-center gap-1 transition-colors"
                        >
                          Need a key? Get one from AI Studio <LogIn size={10} />
                        </button>
                     </div>
                  </div>
                </div>
              )}
            </div>
          )}
        </div>

        <div className="flex justify-end pt-4">
          <button 
             onClick={onClose}
             className="px-6 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl font-bold transition-colors shadow-lg shadow-indigo-500/10"
          >
            Done
          </button>
        </div>
      </div>
    </div>
  );
}
