import {
  Clock,
  Download,
  FolderOpen,
  Plus,
  Pencil,
  RefreshCw,
  Mic,
  Upload,
  X,
  Save,
  Send,
  SlidersHorizontal,
  Settings as SettingsIcon,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { openPath } from '@tauri-apps/plugin-opener';
import { listen } from '@tauri-apps/api/event';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { CreateProjectModal } from './components/CreateProjectModal';
import { AssetsModal } from './components/AssetsModal';
import { FirstRunWizard } from './components/FirstRunWizard';
import { ParametersPanel } from './components/ParametersPanel';
import { ProjectsPanel } from './components/ProjectsPanel';
import { Settings } from './components/Settings';
import { SnapshotHistory } from './components/SnapshotHistory';
import { WelcomePage } from './components/WelcomePage';
import { useExport } from './hooks/useExport';
import { useOpenCode } from './hooks/useOpenCode';
import { usePreviewServer } from './hooks/usePreviewServer';
import { useProjects } from './hooks/useProjects';
import { usePrompt } from './hooks/usePrompt';
import { useSnapshot } from './hooks/useSnapshot';
import { useWaitPort } from './hooks/useWaitPort';
import type { SttStatus } from './types/backend';

function App() {
  const [promptInput, setPromptInput] = useState('');
  const [iframeKey, setIframeKey] = useState(0);
  const [showHistory, setShowHistory] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showAssets, setShowAssets] = useState(false);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const [showWizard, setShowWizard] = useState(false);
  const [wizardDismissed, setWizardDismissed] = useState(false);
  const [isEnvReady, setIsEnvReady] = useState(false);
  const [linkingDeps, setLinkingDeps] = useState(false);
  const [linkingProgress, setLinkingProgress] = useState<{ step: string; percent: number } | null>(null);
  const linkingForWorkspaceRef = useRef<string>('');
  const baselinePromptedSignatureRef = useRef<string>('');

  const [showCreateProject, setShowCreateProject] = useState(false);
  const [defaultProjectsRoot, setDefaultProjectsRoot] = useState('');
  const [isProjectsPanelOpen, setIsProjectsPanelOpen] = useState(() => {
    try {
      return localStorage.getItem('epris:projectsPanelOpen') !== '0';
    } catch {
      return true;
    }
  });
  const [isParametersPanelOpen, setIsParametersPanelOpen] = useState(() => {
    try {
      return localStorage.getItem('epris:parametersPanelOpen') !== '0';
    } catch {
      return true;
    }
  });

  const [sttStatus, setSttStatus] = useState<SttStatus | null>(null);
  const [isRecording, setIsRecording] = useState(false);
  const [isTranscribing, setIsTranscribing] = useState(false);

  // Provider State
  const [provider, setProvider] = useState('opencode');
  const [model, setModel] = useState('opencode/big-pickle');
  const [showModelMenu, setShowModelMenu] = useState(false);

  const MODELS = {
    opencode: [
      { id: 'opencode/big-pickle', name: 'Big Pickle (Default)' },
      { id: 'opencode/minimax-m2.1-free', name: 'MiniMax M2.1' },
    ],
    gemini: [
      { id: 'gemini-3-flash-preview', name: 'Gemini 3.0 Flash Preview' },
      { id: 'gemini-3-pro-preview', name: 'Gemini 3.0 Pro Preview' },
      { id: 'gemini-2.0-flash-exp', name: 'Gemini 2.0 Flash Exp' },
      { id: 'gemini-1.5-pro', name: 'Gemini 1.5 Pro' },
      { id: 'gemini-1.5-flash', name: 'Gemini 1.5 Flash' },
    ],
  };

  const {
    overview,
    activeProject,
    createProject,
    setActiveProject,
    deleteProject,
    renameProject,
    getDefaultProjectsRoot,
    pickProjectsRoot,
  } = useProjects();
  const workspacePath = activeProject?.path;

  const micButtonRef = useRef<HTMLButtonElement | null>(null);
  const mediaStreamRef = useRef<MediaStream | null>(null);
  const audioContextRef = useRef<AudioContext | null>(null);
  const sourceNodeRef = useRef<MediaStreamAudioSourceNode | null>(null);
  const processorNodeRef = useRef<ScriptProcessorNode | null>(null);
  const audioChunksRef = useRef<Float32Array[]>([]);
  const audioSampleRateRef = useRef<number>(48000);

  const {
    status: previewStatus,
    port,
    error: previewError,
    start: startPreview,
  } = usePreviewServer(workspacePath, isEnvReady);
  const { status: openCodeStatus } = useOpenCode(workspacePath, isEnvReady);
  const previewUrl = previewStatus === 'running' && port ? `http://127.0.0.1:${port}` : null;

  const { waitPort } = useWaitPort();
  const reloadPreview = useCallback(async () => {
    if (previewUrl) {
      await waitPort(previewUrl);
    }
    setIframeKey((k) => k + 1);
  }, [previewUrl, waitPort]);

  const previewLogPath = useMemo(() => {
    if (!workspacePath) return '';
    const sep = workspacePath.includes('\\') ? '\\' : '/';
    return `${workspacePath}${sep}logs${sep}preview.log`;
  }, [workspacePath]);

  const {
    status: promptStatus,
    progress: promptProgress,
    currentStep: promptCurrentStep,
    error: promptError,
    lastResponse,
    sendPrompt,
    clearSession,
    revalidateGate,
  } = usePrompt(
    workspacePath,
    {
      onSuccess: () => {
        reloadPreview();
      },
    },
    provider,
    model,
  );

  // STT status
  useEffect(() => {
    const run = async () => {
      try {
        const s = await invoke<SttStatus>('get_stt_status');
        setSttStatus(s);
      } catch {
        setSttStatus({ installed: false, binary_ok: false, model_ok: false });
      }
    };
    void run();
  }, [showWizard]);

  // STT streaming events
  useEffect(() => {
    const unlistenChunk = listen<string>('stt_chunk', (event) => {
      const text = String(event.payload || '').trim();
      if (!text) return;
      setPromptInput((prev) => (prev ? `${prev} ${text}` : text));
    });
    const unlistenLog = listen<string>('stt_log', (event) => {
      const line = String(event.payload || '').trim();
      if (line) console.log('[STT]', line);
    });
    return () => {
      // React 18 StrictMode can mount/unmount effects quickly in dev; keep unlisten promises
      // so we don't leak duplicate listeners.
      void unlistenChunk.then((f) => f());
      void unlistenLog.then((f) => f());
    };
  }, []);

  const base64FromBytes = useCallback((bytes: Uint8Array) => {
    let binary = '';
    const chunk = 0x8000;
    for (let i = 0; i < bytes.length; i += chunk) {
      binary += String.fromCharCode(...bytes.subarray(i, i + chunk));
    }
    return btoa(binary);
  }, []);

  const encodeWavPcm16 = useCallback((samples: Float32Array, sampleRate: number) => {
    const buffer = new ArrayBuffer(44 + samples.length * 2);
    const view = new DataView(buffer);

    const writeString = (offset: number, s: string) => {
      for (let i = 0; i < s.length; i++) view.setUint8(offset + i, s.charCodeAt(i));
    };

    writeString(0, 'RIFF');
    view.setUint32(4, 36 + samples.length * 2, true);
    writeString(8, 'WAVE');
    writeString(12, 'fmt ');
    view.setUint32(16, 16, true); // PCM chunk size
    view.setUint16(20, 1, true); // PCM format
    view.setUint16(22, 1, true); // mono
    view.setUint32(24, sampleRate, true);
    view.setUint32(28, sampleRate * 2, true); // byte rate
    view.setUint16(32, 2, true); // block align
    view.setUint16(34, 16, true); // bits per sample
    writeString(36, 'data');
    view.setUint32(40, samples.length * 2, true);

    let offset = 44;
    for (let i = 0; i < samples.length; i++) {
      const s = Math.max(-1, Math.min(1, samples[i]));
      view.setInt16(offset, s < 0 ? s * 0x8000 : s * 0x7fff, true);
      offset += 2;
    }

    return new Uint8Array(buffer);
  }, []);

  const trimSilence = useCallback((samples: Float32Array, sampleRate: number) => {
    // Frame-based trimming: remove leading/trailing silence to speed up whisper.cpp.
    // Uses average absolute amplitude per 10ms frame.
    const frameSize = Math.max(1, Math.floor(sampleRate * 0.01)); // 10ms
    const frameCount = Math.ceil(samples.length / frameSize);
    if (frameCount <= 2) return samples;

    const threshold = 0.01; // avg |amp|; tweakable
    let startFrame = 0;
    for (let f = 0; f < frameCount; f++) {
      const start = f * frameSize;
      const end = Math.min(samples.length, start + frameSize);
      let sum = 0;
      for (let i = start; i < end; i++) sum += Math.abs(samples[i]);
      const avg = sum / Math.max(1, end - start);
      if (avg > threshold) {
        startFrame = f;
        break;
      }
      if (f === frameCount - 1) return samples; // all silence (or too quiet)
    }

    let endFrame = frameCount - 1;
    for (let f = frameCount - 1; f >= 0; f--) {
      const start = f * frameSize;
      const end = Math.min(samples.length, start + frameSize);
      let sum = 0;
      for (let i = start; i < end; i++) sum += Math.abs(samples[i]);
      const avg = sum / Math.max(1, end - start);
      if (avg > threshold) {
        endFrame = f;
        break;
      }
    }

    const pad = Math.floor(sampleRate * 0.15); // 150ms padding
    const start = Math.max(0, startFrame * frameSize - pad);
    const end = Math.min(samples.length, (endFrame + 1) * frameSize + pad);

    // Avoid trimming into an unusably short clip.
    const minSamples = Math.floor(sampleRate * 0.2); // 200ms
    if (end - start < minSamples) return samples;
    return samples.subarray(start, end);
  }, []);

  const startRecording = useCallback(async () => {
    if (!sttStatus?.installed) return;
    if (isRecording || isTranscribing) return;

    const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
    const audioContext = new AudioContext();
    const source = audioContext.createMediaStreamSource(stream);
    const processor = audioContext.createScriptProcessor(4096, 1, 1);
    const zeroGain = audioContext.createGain();
    zeroGain.gain.value = 0;

    audioChunksRef.current = [];
    audioSampleRateRef.current = audioContext.sampleRate;

    processor.onaudioprocess = (e) => {
      const input = e.inputBuffer.getChannelData(0);
      audioChunksRef.current.push(new Float32Array(input));
    };

    source.connect(processor);
    processor.connect(zeroGain);
    zeroGain.connect(audioContext.destination);

    mediaStreamRef.current = stream;
    audioContextRef.current = audioContext;
    sourceNodeRef.current = source;
    processorNodeRef.current = processor;

    setIsRecording(true);
  }, [isRecording, isTranscribing, sttStatus?.installed]);

  const stopRecordingAndTranscribe = useCallback(async () => {
    if (!workspacePath) return;
    if (!isRecording) return;

    setIsRecording(false);
    setIsTranscribing(true);
    try {
      try {
        processorNodeRef.current?.disconnect();
      } catch {}
      try {
        sourceNodeRef.current?.disconnect();
      } catch {}
      try {
        audioContextRef.current && (await audioContextRef.current.close());
      } catch {}
      try {
        mediaStreamRef.current?.getTracks().forEach((t) => t.stop());
      } catch {}

      const chunks = audioChunksRef.current;
      const total = chunks.reduce((acc, c) => acc + c.length, 0);
      const merged = new Float32Array(total);
      let off = 0;
      for (const c of chunks) {
        merged.set(c, off);
        off += c.length;
      }

      const trimmed = trimSilence(merged, audioSampleRateRef.current);
      const wavBytes = encodeWavPcm16(trimmed, audioSampleRateRef.current);
      const wavBase64 = base64FromBytes(wavBytes);
      await invoke('transcribe_whispercpp', { workspacePath, wavBase64 });
    } catch (e) {
      console.error('STT failed:', e);
    } finally {
      setIsTranscribing(false);
      audioChunksRef.current = [];
      mediaStreamRef.current = null;
      audioContextRef.current = null;
      sourceNodeRef.current = null;
      processorNodeRef.current = null;
    }
  }, [base64FromBytes, encodeWavPcm16, isRecording, trimSilence, workspacePath]);

  // Click anywhere else to stop recording.
  useEffect(() => {
    if (!isRecording) return;
    const onDocClick = (e: MouseEvent) => {
      const target = e.target as Node | null;
      if (target && micButtonRef.current && micButtonRef.current.contains(target)) return;
      void stopRecordingAndTranscribe();
    };
    document.addEventListener('mousedown', onDocClick, true);
    return () => {
      document.removeEventListener('mousedown', onDocClick, true);
    };
  }, [isRecording, stopRecordingAndTranscribe]);

  const {
    status: exportStatus,
    progress: exportProgress,
    outputPath,
    error: exportError,
    exportVideo,
    openOutputFolder,
  } = useExport(workspacePath);
  const { getHead, checkUnsavedChanges, manualSaveSnapshot } = useSnapshot(workspacePath);

  // Poll for unsaved changes
  useEffect(() => {
    if (!workspacePath) return;
    const check = async () => {
      const changed = await checkUnsavedChanges();
      setHasUnsavedChanges(changed);
    };
    check();
    const interval = setInterval(check, 5000);
    return () => clearInterval(interval);
  }, [workspacePath, checkUnsavedChanges]);

  // Check Environment on mount/provider change
  useEffect(() => {
    if (!workspacePath) return;
    // While the setup wizard is open, let the wizard own environment checks.
    // Otherwise, this background poll can race the install flow and close/reopen the modal.
    let canceled = false;
    if (showWizard) {
      return () => {
        canceled = true;
      };
    }
    const checkEnv = async () => {
      try {
        const status = await invoke<any>('get_environment_status', {
          provider,
          workspacePath,
        });
        if (canceled) return;
        const toolchainReady =
          status.node_valid && status.pnpm_valid && status.provider_cli_valid && status.skills_valid;
        const depsReady = Boolean(status.workspace_deps_valid);
        const ready = toolchainReady && depsReady;

        const maybePromptBaselinePackages = async (manageProgress: boolean) => {
          if (!toolchainReady) return;
          try {
            const info = await invoke<{
              signature: string;
              missing_in_workspace: string[];
              missing_in_template: string[];
            }>('get_baseline_packages_info', { workspacePath });
            const appState = await invoke<any>('get_app_state');
            const ack = String(appState?.baseline_packages_ack || '');
            const signature = String(info?.signature || '');
            const missing = [
              ...(info?.missing_in_template || []),
              ...(info?.missing_in_workspace || []),
            ].filter(Boolean);
            if (!signature || missing.length === 0) return;
            if (ack === signature) return;
            if (baselinePromptedSignatureRef.current === signature) return;
            baselinePromptedSignatureRef.current = signature;

            const ok = confirm(
              `This version adds baseline JavaScript packages:\n\n- ${missing.join(
                '\n- ',
              )}\n\nInstall them now? (You will only be asked once per app update.)`,
            );
            if (!ok) {
              await invoke('set_baseline_packages_ack', { signature });
              return;
            }

            if (manageProgress) {
              setLinkingDeps(true);
              setLinkingProgress({ step: 'Installing baseline packages', percent: 0 });
            }
            try {
              await invoke('install_baseline_packages', { workspacePath });
              await invoke('set_baseline_packages_ack', { signature });
            } catch (e) {
              baselinePromptedSignatureRef.current = '';
              throw e;
            } finally {
              if (manageProgress) setLinkingDeps(false);
            }
          } catch (e) {
            console.error('Baseline packages check failed:', e);
          }
        };

        if (ready) {
          setIsEnvReady(true);
          setShowWizard(false);
          setWizardDismissed(false);
          setLinkingDeps(false);
          void maybePromptBaselinePackages(true);
          return;
        }

        // If only workspace deps are missing, silently link them (no modal wizard).
        if (toolchainReady && !depsReady) {
          setIsEnvReady(false);
          setShowWizard(false);
          if (linkingForWorkspaceRef.current !== workspacePath) {
            linkingForWorkspaceRef.current = workspacePath;
            setLinkingDeps(true);
            setLinkingProgress({ step: 'Linking dependencies', percent: 0 });
            try {
              await maybePromptBaselinePackages(false);
              await invoke('link_workspace_dependencies', { workspacePath });
              setIsEnvReady(true);
            } catch (e) {
              console.error('Failed to link deps:', e);
              setShowWizard(true);
            } finally {
              setLinkingDeps(false);
            }
          }
          return;
        }

        setIsEnvReady(false);
        if (wizardDismissed) {
          setShowWizard(false);
        } else {
          setShowWizard(true);
        }
      } catch (e) {
        console.error('Failed to check env:', e);
      }
    };
    checkEnv();
    return () => {
      canceled = true;
    };
  }, [workspacePath, provider, wizardDismissed, showWizard]);

  // Step 13: default provider when opening a project
  useEffect(() => {
    setIsEnvReady(false);
    setShowWizard(false);
    setWizardDismissed(false);
    setLinkingDeps(false);
    setLinkingProgress(null);
    linkingForWorkspaceRef.current = '';
    if (!workspacePath) return;
    setProvider('opencode');
    setModel('opencode/big-pickle');
  }, [workspacePath]);

  useEffect(() => {
    try {
      localStorage.setItem('epris:projectsPanelOpen', isProjectsPanelOpen ? '1' : '0');
    } catch {}
  }, [isProjectsPanelOpen]);

  useEffect(() => {
    try {
      localStorage.setItem('epris:parametersPanelOpen', isParametersPanelOpen ? '1' : '0');
    } catch {}
  }, [isParametersPanelOpen]);

  // Track env install progress even when wizard isn't shown (e.g. linking deps).
  useEffect(() => {
    const unlisten = listen<any>('env_install_progress', (e) => {
      const p = e.payload || {};
      const step = String(p.step || '');
      const percent = typeof p.percent === 'number' ? p.percent : null;
      if (!step || percent === null) return;
      setLinkingProgress({ step, percent });
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, []);

  const handleProviderChange = useCallback((newProvider: string) => {
    setProvider(newProvider);
    if (newProvider === 'gemini') setModel('gemini-3-flash-preview');
    else setModel('opencode/big-pickle');
  }, []);

  const handleManualSave = useCallback(async () => {
    if (!workspacePath) return;
    const name = prompt('Enter snapshot name:', `Manual ${new Date().toLocaleTimeString()}`);
    if (!name) return;
    const desc = prompt('Enter description (optional):', '');
    try {
      const currentHead = await getHead();
      await manualSaveSnapshot(name, desc || '', currentHead);
    } catch (err) {
      alert('Save failed: ' + (err instanceof Error ? err.message : String(err)));
    }
  }, [workspacePath, manualSaveSnapshot, getHead]);

  const handleSubmit = useCallback(async () => {
    if (!promptInput.trim()) return;
    const currentPrompt = promptInput;
    setPromptInput('');
    await sendPrompt(currentPrompt, provider, model);
  }, [promptInput, sendPrompt, provider, model]);

  const handleExport = useCallback(async () => {
    try {
      await exportVideo();
    } catch (err) {
      alert('Export failed: ' + (err instanceof Error ? err.message : String(err)));
    }
  }, [exportVideo]);

  const handleViewLogs = useCallback(async () => {
    if (!workspacePath) return;
    try {
      const logPath = `${workspacePath}\\logs\\gate.jsonl`;
      await openPath(logPath);
    } catch (err) {
      console.error('Failed to open logs:', err);
      alert('Failed to open logs. Check: ' + workspacePath + '\\logs\\gate.jsonl');
    }
  }, [workspacePath]);

  const handleCreateProjectClick = useCallback(async () => {
    // Only ask for ProjectsRoot the first time.
    if (overview?.projects_root) {
      try {
        await createProject(undefined, undefined);
      } catch (e) {
        alert('Failed to create project: ' + (e instanceof Error ? e.message : String(e)));
      }
      return;
    }

    try {
      const def = await getDefaultProjectsRoot();
      setDefaultProjectsRoot(def);
    } catch (e) {
      console.warn('Failed to fetch default projects root:', e);
    } finally {
      setShowCreateProject(true);
    }
  }, [createProject, getDefaultProjectsRoot, overview?.projects_root]);

  const handleSelectProject = useCallback(
    async (projectId: string) => {
      if (projectId === overview?.active_project_id) return;
      try {
        if (workspacePath) {
          await invoke('auto_save_checkpoint', {
            workspacePath,
            reason: 'Before switching project',
            force: false,
          });
        }
      } catch {}
      try {
        await invoke('stop_gate_validation');
      } catch {}
      try {
        await invoke('stop_preview_server');
      } catch {}
      try {
        await invoke('stop_opencode');
      } catch {}
      await setActiveProject(projectId);
      setIframeKey((k) => k + 1);
    },
    [overview?.active_project_id, setActiveProject, workspacePath],
  );

  const handleDeleteProject = useCallback(
    async (projectId: string) => {
      const target = overview?.projects.find((p) => p.id === projectId);
      if (!target) return;
      if (
        !confirm(
          `Delete project "${target.name}"?\n\nThis will permanently delete the folder on disk:\n${target.path}`,
        )
      ) {
        return;
      }

      if (projectId === overview?.active_project_id) {
        try {
          if (workspacePath) {
            await invoke('auto_save_checkpoint', {
              workspacePath,
              reason: 'Before deleting project',
              force: true,
            });
          }
        } catch {}
        try {
          await invoke('stop_gate_validation');
        } catch {}
        try {
          await invoke('stop_preview_server');
        } catch {}
        try {
          await invoke('stop_opencode');
        } catch {}
      }
      await deleteProject(projectId);
      setIframeKey((k) => k + 1);
    },
    [deleteProject, overview],
  );

  const handleRenameProject = useCallback(
    async (projectId: string) => {
      const target = overview?.projects.find((p) => p.id === projectId);
      const currentName = target?.name || 'Project';
      const next = prompt('Rename project:', currentName);
      if (!next) return;
      await renameProject(projectId, next);
    },
    [overview, renameProject],
  );

  const isReady = Boolean(workspacePath) && previewStatus === 'running' && openCodeStatus === 'running' && isEnvReady;
  const isProcessing = promptStatus === 'sending';
  const hasGateError = promptStatus === 'error';
  const gateFailed = Boolean(lastResponse?.gate_result && !lastResponse.gate_result.passed);

  const missingPackages = useMemo(() => {
    const err = lastResponse?.gate_result?.error_output || lastResponse?.gate_result?.error || '';
    const found: string[] = [];

    const parseDepRequestLine = (line: string) => {
      const after = line.split(':').slice(1).join(':');
      for (const raw of after.split(',')) {
        const name = raw.trim().split(/\s+/)[0];
        if (!name) continue;
        if (name.startsWith('.') || name.startsWith('/') || name.includes('\\')) continue;
        if (!found.includes(name)) found.push(name);
      }
    };

    const msg = String(lastResponse?.message || '');
    const reqLineMsg = msg.split('\n').find((l) => l.trim().toUpperCase().startsWith('DEPENDENCY_REQUEST:'));
    if (reqLineMsg) parseDepRequestLine(reqLineMsg);

    const reqLineErr = err.split('\n').find((l) => l.trim().toUpperCase().startsWith('DEPENDENCY_REQUEST:'));
    if (reqLineErr) parseDepRequestLine(reqLineErr);

    if (!err) return found;

    const patterns = [
      /Cannot find module ['"]([^'"]+)['"]/g,
      /Error:\s*Cannot find module ['"]([^'"]+)['"]/g,
    ];

    for (const re of patterns) {
      let m: RegExpExecArray | null;
      while ((m = re.exec(err))) {
        const name = String(m[1] || '').trim();
        if (!name) continue;
        if (name.startsWith('.') || name.startsWith('/') || name.includes('\\')) continue;
        if (!found.includes(name)) found.push(name);
      }
    }
    return found;
  }, [lastResponse?.gate_result, lastResponse?.message]);

  const overallStatus = useMemo(() => {
    if (!workspacePath) return { text: 'No Project', color: 'bg-slate-500' };
    if (previewStatus === 'starting') return { text: 'Starting Preview...', color: 'bg-yellow-500' };
    if (previewStatus === 'error') return { text: 'Preview Error', color: 'bg-red-500' };
    if (openCodeStatus === 'starting') return { text: 'Starting AI...', color: 'bg-yellow-500' };
    if (openCodeStatus === 'error') return { text: 'AI Error', color: 'bg-red-500' };
    if (promptStatus === 'sending') return { text: 'Validating...', color: 'bg-blue-500' };
    if (promptStatus === 'error') return { text: 'Gate Error', color: 'bg-red-500' };
    if (promptStatus === 'success') {
      const gate = lastResponse?.gate_result;
      if (gate) {
        if (gate.passed) return { text: 'Gate Passed', color: 'bg-emerald-500' };
        if (missingPackages.length > 0) return { text: 'Needs dependencies', color: 'bg-amber-500' };
        return { text: 'Gate Failed', color: 'bg-amber-500' };
      }
      return { text: 'Ready', color: 'bg-green-500' };
    }
    if (exportStatus === 'exporting') return { text: 'Exporting...', color: 'bg-purple-500' };
    if (exportStatus === 'done') return { text: 'Export Complete', color: 'bg-green-500' };
    if (exportStatus === 'error') return { text: 'Export Error', color: 'bg-red-500' };
    if (previewStatus === 'running' && openCodeStatus === 'running') return { text: 'Ready', color: 'bg-emerald-500' };
    return { text: 'Loading...', color: 'bg-slate-500' };
  }, [workspacePath, previewStatus, openCodeStatus, promptStatus, exportStatus, lastResponse, missingPackages.length]);

  const [showDependencyModal, setShowDependencyModal] = useState(false);
  const [isInstallingMissingPackages, setIsInstallingMissingPackages] = useState(false);

  useEffect(() => {
    setShowDependencyModal(false);
    setIsInstallingMissingPackages(false);
  }, [workspacePath]);

  useEffect(() => {
    if (!workspacePath) return;
    if (isProcessing) return;
    if (!gateFailed) return;
    if (missingPackages.length === 0) return;
    setShowDependencyModal(true);
  }, [gateFailed, isProcessing, missingPackages.length, workspacePath]);

  const handleRevertPreFlight = useCallback(async (opts?: { skipConfirm?: boolean }) => {
    if (!workspacePath) return;
    if (!opts?.skipConfirm) {
      const ok = confirm(
        `Revert this workspace back to the state before the last prompt?\n\nThis will discard the AI changes from the last run.`,
      );
      if (!ok) return;
    }
    try {
      await invoke('restore_pre_flight_backup', { workspacePath });
      await clearSession();
      setIframeKey((k) => k + 1);
      alert('Reverted to pre-prompt state.');
    } catch (err) {
      alert(`Revert failed: ${String(err)}`);
    }
  }, [clearSession, workspacePath]);

  const handleInstallMissingPackages = useCallback(async (opts?: { skipConfirm?: boolean }) => {
    if (!workspacePath) return;
    if (missingPackages.length === 0) return;
    if (!opts?.skipConfirm) {
      const ok = confirm(
        `Install missing packages into this project?\n\n${missingPackages.join('\n')}\n\nThis will modify the project's package.json and pnpm-lock.yaml.`,
      );
      if (!ok) {
        void handleRevertPreFlight();
        return;
      }
    }
    try {
      setIsInstallingMissingPackages(true);
      setLinkingDeps(true);
      setLinkingProgress({ step: 'Installing dependencies', percent: 0 });
      await invoke('install_js_packages', { workspacePath, packages: missingPackages });
      setLinkingProgress({ step: 'Re-validating...', percent: 95 });
      await revalidateGate();
    } catch (err) {
      alert(`Install failed: ${String(err)}`);
      throw err;
    } finally {
      setIsInstallingMissingPackages(false);
      setLinkingDeps(false);
    }
  }, [missingPackages, workspacePath, handleRevertPreFlight, revalidateGate]);

  const handleFixGateError = useCallback(async () => {
    if (!workspacePath || !lastResponse?.gate_result) return;
    const err = lastResponse.gate_result.error_output || lastResponse.gate_result.error || 'Unknown gate error';
    const fixPrompt =
      `Fix the workspace so Gate passes.\n` +
      `Only modify files inside src/** and public/**.\n` +
      `Do not refactor unrelated code.\n\n` +
      `Gate error output:\n${err}\n`;
    setPromptInput(fixPrompt);
    await sendPrompt(fixPrompt);
  }, [lastResponse?.gate_result, sendPrompt, workspacePath]);

  const handleSkipValidation = useCallback(async () => {
    if (!workspacePath) return;
    try {
      await invoke('auto_save_checkpoint', {
        workspacePath,
        reason: 'Skip validation',
        force: true,
      });
    } catch {}
    try {
      await invoke('stop_gate_validation');
    } catch {}
  }, [workspacePath]);

  const handleCancelRun = useCallback(async () => {
    if (!workspacePath) return;
    try {
      await invoke('auto_save_checkpoint', {
        workspacePath,
        reason: 'Cancel run',
        force: true,
      });
    } catch {}
    try {
      await invoke('cancel_current_run');
    } catch {}
  }, [workspacePath]);

  return (
    <div className="flex flex-col h-screen bg-slate-900 text-slate-100 font-sans">
      {/* Header */}
      <header className="flex items-center justify-between px-6 py-4 border-b border-slate-800 bg-slate-900/50 backdrop-blur-sm sticky top-0 z-10">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-lg bg-indigo-500 flex items-center justify-center">
            <span className="font-bold text-lg">E</span>
          </div>
          <div className="flex items-center gap-3 min-w-0">
            <h1 className="text-xl font-semibold tracking-tight">Epris Desktop</h1>
            {activeProject && (
              <button
                onClick={() => handleRenameProject(activeProject.id)}
                className="max-w-[320px] px-3 py-1.5 rounded-xl bg-slate-800/60 border border-slate-700 text-slate-200 hover:bg-slate-800 transition-colors flex items-center gap-2 min-w-0"
                title="Rename project"
              >
                <span className="text-sm font-bold truncate">{activeProject.name}</span>
                <Pencil size={14} className="text-slate-400 shrink-0" />
              </button>
            )}
          </div>
        </div>
        <div className="flex items-center gap-2">
          {isProcessing && (
            <div className="hidden lg:flex items-center gap-2 px-3 py-2 rounded-xl bg-slate-800/40 border border-slate-700 text-slate-200 max-w-[720px]">
              <RefreshCw size={16} className="animate-spin text-indigo-400 shrink-0" />
              <div className="min-w-0 flex-1">
                <div className="text-xs font-bold truncate">{promptCurrentStep || 'Working...'}</div>
                <div className="w-full bg-slate-800 rounded-full h-1.5 mt-1 overflow-hidden border border-slate-700">
                  <div
                    className="bg-indigo-500 h-full transition-all duration-500 ease-out"
                    style={{ width: `${promptProgress}%` }}
                  />
                </div>
              </div>
              <div className="text-[11px] text-slate-400 font-bold tabular-nums shrink-0">
                {Math.round(promptProgress)}%
              </div>
              <span className="w-px h-6 bg-slate-700/70 mx-1" />
              {String(promptCurrentStep || '').startsWith('Gate:') && (
                <button
                  onClick={handleSkipValidation}
                  className="px-2 py-1 rounded-lg text-[11px] font-black uppercase tracking-widest bg-slate-900/50 border border-slate-700 hover:bg-slate-900 transition-colors"
                  title="Stop validation checks and continue"
                >
                  Skip Checks
                </button>
              )}
              <button
                onClick={handleCancelRun}
                className="p-1.5 rounded-lg hover:bg-red-500/10 text-slate-400 hover:text-red-300 transition-colors"
                title="Cancel this run (stop AI + validation)"
              >
                <X size={14} />
              </button>
            </div>
          )}

          {gateFailed && !isProcessing && missingPackages.length === 0 && (
            <div className="hidden lg:flex items-center gap-2">
              <button
                onClick={handleFixGateError}
                className="flex items-center gap-2 px-3 py-2 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-300 hover:bg-amber-500/15 transition-colors"
                title="Send Gate error to AI and ask it to fix"
              >
                <span className="text-xs font-black uppercase tracking-widest">Gate Failed</span>
                <span className="text-xs font-bold">Fix with AI</span>
              </button>
            </div>
          )}

          <button
            onClick={handleManualSave}
            className="p-2 hover:bg-slate-800 rounded-lg transition-colors flex items-center gap-1 text-sm bg-indigo-600/20 text-indigo-400 border border-indigo-500/20"
            title="Save Current Version"
            disabled={!isReady || isProcessing}
          >
            <Save size={16} />
            <span className="hidden sm:inline">Save</span>
          </button>

          {exportStatus === 'done' && outputPath ? (
            <button
              onClick={openOutputFolder}
              className="p-2 hover:bg-slate-800 rounded-lg transition-colors flex items-center gap-1 text-sm bg-green-600/20"
              title="Open Export Folder"
            >
              <FolderOpen size={16} />
              <span className="hidden sm:inline">Open Folder</span>
            </button>
          ) : (
            <button
              onClick={handleExport}
              className="relative p-2 hover:bg-slate-800 rounded-lg transition-colors flex items-center gap-1 text-sm overflow-hidden"
              title="Export Video"
              disabled={!isReady || exportStatus === 'exporting'}
            >
              {exportStatus === 'exporting' && (
                <div
                  className="absolute bottom-0 left-0 h-1 bg-indigo-500 transition-all duration-300"
                  style={{ width: `${exportProgress}%` }}
                />
              )}
              <Download size={16} className={exportStatus === 'exporting' ? 'animate-pulse' : ''} />
              <span className="hidden sm:inline">
                {exportStatus === 'exporting' ? `Exporting ${Math.round(exportProgress)}%` : 'Export'}
              </span>
            </button>
          )}

          <button
            onClick={clearSession}
            className="p-2 hover:bg-slate-800 rounded-lg transition-colors flex items-center gap-1 text-sm"
            title="New Session"
          >
            <Plus size={16} />
            <span className="hidden sm:inline">New Session</span>
          </button>

          <button
            onClick={() => setShowHistory(true)}
            className="p-2 hover:bg-slate-800 rounded-lg transition-colors flex items-center gap-1 text-sm text-slate-400 hover:text-indigo-400"
            title="Snapshot Timeline"
            disabled={!workspacePath}
          >
            <Clock size={18} />
          </button>

          <button
            onClick={reloadPreview}
            className="p-2 hover:bg-slate-800 rounded-lg transition-colors"
            title="Reload Preview"
            disabled={!workspacePath}
          >
            <RefreshCw size={18} className={isProcessing ? 'animate-spin' : ''} />
          </button>

          <button
            onClick={() => setShowAssets(true)}
            className="p-2 hover:bg-slate-800 rounded-lg transition-colors flex items-center gap-1 text-sm text-slate-400"
            title="Project Assets"
            disabled={!workspacePath}
          >
            <Upload size={18} />
          </button>

          <button
            onClick={() => setShowSettings(true)}
            className="p-2 hover:bg-slate-800 rounded-lg transition-colors flex items-center gap-1 text-sm text-slate-400"
            title="Settings"
          >
            <SettingsIcon size={18} />
          </button>

          {hasGateError && (
            <button
              onClick={handleViewLogs}
              className="p-2 hover:bg-slate-800 rounded-lg transition-colors flex items-center gap-1 text-sm text-amber-400 border border-amber-400/30"
              title="View Gate Logs"
            >
              <span className="hidden sm:inline">View Logs</span>
            </button>
          )}
        </div>
      </header>

      {/* Content */}
      <div className="flex-1 min-h-0 flex overflow-hidden">
        {overview && isProjectsPanelOpen && (
          <ProjectsPanel
            projects={overview.projects}
            activeProjectId={overview.active_project_id}
            onCollapse={() => setIsProjectsPanelOpen(false)}
            onCreate={handleCreateProjectClick}
            onSelect={handleSelectProject}
            onDelete={handleDeleteProject}
            onRename={handleRenameProject}
          />
        )}

        {!isProjectsPanelOpen && (
          <button
            onClick={() => setIsProjectsPanelOpen(true)}
            className="w-10 shrink-0 bg-slate-900/60 border-r border-slate-800 hover:bg-slate-800/40 transition-colors flex items-center justify-center text-slate-400 hover:text-slate-200"
            title="Show Projects"
          >
            <FolderOpen size={18} />
          </button>
        )}

        <main className="flex-1 min-h-0 flex flex-col gap-6 p-6 overflow-hidden">
          {!workspacePath ? (
            <WelcomePage onCreateFirst={handleCreateProjectClick} />
          ) : (
            <>
              {/* Preview Section */}
              <div className="flex-1 min-h-0 bg-slate-800/50 rounded-2xl border border-slate-700/50 shadow-2xl overflow-hidden relative group">
                {previewStatus === 'error' ? (
                  <div className="absolute inset-0 flex items-center justify-center text-slate-200">
                    <div className="text-center max-w-lg px-6">
                      <div className="text-lg font-bold mb-2">Preview failed to start</div>
                      <div className="text-sm text-slate-400 mb-4 break-words">
                        {previewError?.message || 'Unknown error'}
                      </div>
                      <div className="flex items-center justify-center gap-3">
                        <button
                          onClick={() => startPreview().catch(console.error)}
                          className="px-4 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-bold text-sm transition-colors"
                        >
                          Retry
                        </button>
                        {previewLogPath && (
                          <button
                            onClick={() => openPath(previewLogPath).catch(console.error)}
                            className="px-4 py-2 rounded-xl bg-slate-800 hover:bg-slate-700 border border-slate-600 text-slate-200 font-bold text-sm transition-colors"
                          >
                            Open preview.log
                          </button>
                        )}
                      </div>
                      {!isEnvReady && (
                        <div className="text-xs text-slate-500 mt-4">
                          If this is a new project, run the First Run Wizard to install workspace dependencies.
                        </div>
                      )}
                    </div>
                  </div>
                ) : previewUrl ? (
                  <iframe
                    key={iframeKey}
                    src={`${previewUrl}?t=${iframeKey}`}
                    className="w-full h-full border-0"
                    title="Remotion Preview"
                  />
                ) : (
                  <div className="absolute inset-0 flex items-center justify-center text-slate-400">
                    <div className="text-center">
                      <div className="text-lg mb-2">Loading Preview...</div>
                      <div className="text-sm opacity-60">Starting Remotion Studio server</div>
                    </div>
                  </div>
                )}

                {/* Progress moved to header */}
              </div>

              {/* Input Section */}
              <div className="w-full max-w-4xl mx-auto flex flex-col gap-3">
                <div className="relative group flex gap-3 items-center">
                  {/* Model Badge / Selection (Left of Input) */}
                  <div className="relative">
                    <button
                      onClick={() => setShowModelMenu(!showModelMenu)}
                      className="flex-shrink-0 px-4 py-4 bg-slate-800 hover:bg-slate-700 border-2 border-slate-700 rounded-2xl flex items-center gap-2 transition-colors group/badge"
                      title="Change AI Model"
                    >
                      <span
                        className={`w-2 h-2 rounded-full ${
                          provider === 'gemini'
                            ? 'bg-blue-400 shadow-[0_0_8px_rgba(96,165,250,0.5)]'
                            : 'bg-green-400 shadow-[0_0_8px_rgba(74,222,128,0.5)]'
                        }`}
                      />
                      <div className="flex flex-col items-start leading-none gap-1">
                        <span className="text-[10px] font-bold text-slate-500 uppercase tracking-wider">
                          {provider === 'gemini' ? 'Gemini' : 'OpenCode'}
                        </span>
                        <span className="text-xs font-bold text-slate-300 max-w-[100px] truncate">
                          {model.split('/').pop()}
                        </span>
                      </div>
                    </button>

                    {showModelMenu && (
                      <>
                        <div className="fixed inset-0 z-10" onClick={() => setShowModelMenu(false)} />
                        <div className="absolute bottom-full mb-2 left-0 w-64 bg-slate-900 border border-slate-700 rounded-xl shadow-xl z-20 overflow-hidden py-1">
                          <div className="px-3 py-2 text-[10px] font-bold text-slate-500 uppercase tracking-wider bg-slate-900 border-b border-slate-800">
                            Select Model
                          </div>
                          {MODELS[provider as keyof typeof MODELS]?.map((m) => (
                            <button
                              key={m.id}
                              onClick={() => {
                                setModel(m.id);
                                setShowModelMenu(false);
                              }}
                              className={`w-full text-left px-4 py-3 text-sm hover:bg-slate-800 transition-colors flex items-center justify-between ${
                                model === m.id ? 'text-indigo-400 bg-slate-800/50' : 'text-slate-300'
                              }`}
                            >
                              <span className="truncate">{m.name}</span>
                              {model === m.id && <div className="w-1.5 h-1.5 rounded-full bg-indigo-500" />}
                            </button>
                          ))}
                          <div className="border-t border-slate-800 mt-1 pt-1">
                            <button
                              onClick={() => {
                                setShowSettings(true);
                                setShowModelMenu(false);
                              }}
                              className="w-full text-left px-4 py-2 text-xs text-slate-500 hover:text-slate-300 hover:bg-slate-800/50 transition-colors flex items-center gap-2"
                            >
                              <SettingsIcon size={12} />
                              Change Provider...
                            </button>
                          </div>
                        </div>
                      </>
                    )}
                  </div>

                  <div className="relative flex-1">
                    <input
                      type="text"
                      value={promptInput}
                      onChange={(e) => setPromptInput(e.target.value)}
                      placeholder={isReady ? 'Describe animation...' : 'Waiting services...'}
                      className="w-full bg-slate-800 border-2 border-slate-700 rounded-2xl px-6 py-4 pr-28 text-lg focus:outline-none focus:border-indigo-500 transition-all placeholder:text-slate-500 shadow-xl disabled:opacity-50"
                      onKeyDown={(e) => e.key === 'Enter' && !isProcessing && handleSubmit()}
                      disabled={!isReady}
                    />
                    <button
                      ref={micButtonRef}
                      className={`absolute right-14 top-1/2 -translate-y-1/2 p-3 rounded-xl transition-all shadow-lg active:scale-95 disabled:opacity-50 disabled:active:scale-100 ${
                        isRecording
                          ? 'bg-red-600 hover:bg-red-500 text-white'
                          : isTranscribing
                            ? 'bg-amber-600 hover:bg-amber-500 text-white'
                            : 'bg-slate-700 hover:bg-slate-600 text-slate-100'
                      }`}
                      disabled={!isReady || !sttStatus?.installed}
                      title={
                        !sttStatus?.installed
                          ? 'Voice-to-text is not installed (run First Run Wizard)'
                          : isRecording
                            ? 'Recording... click anywhere to stop'
                            : isTranscribing
                              ? 'Transcribing...'
                              : 'Voice-to-text'
                      }
                      onClick={async () => {
                        if (isTranscribing) {
                          try {
                            await invoke('cancel_stt');
                          } catch {}
                          return;
                        }
                        if (isRecording) {
                          await stopRecordingAndTranscribe();
                          return;
                        }
                        await startRecording();
                      }}
                    >
                      <Mic size={20} />
                    </button>
                    <button
                      className="absolute right-3 top-1/2 -translate-y-1/2 p-3 bg-indigo-600 hover:bg-indigo-500 rounded-xl transition-all shadow-lg active:scale-95 text-white disabled:opacity-50 disabled:active:scale-100"
                      disabled={!promptInput.trim() || !isReady || isProcessing}
                      onClick={handleSubmit}
                    >
                      <Send size={20} />
                    </button>
                  </div>
                </div>

                {promptError && (
                  <p className="px-4 text-sm text-red-400">
                    Gate Error: {promptError}
                    {hasGateError && (
                      <button onClick={handleViewLogs} className="ml-2 underline text-amber-400 hover:text-amber-300">
                        View Logs
                      </button>
                    )}
                  </p>
                )}

                {exportError && (
                  <p className="px-4 text-sm text-red-400">
                    Export Error: {exportError}
                  </p>
                )}

                <p className="px-4 text-sm text-slate-500">
                  {isReady ? 'Press Enter or click send to generate code with AI.' : 'Waiting for preview server and OpenCode to start...'}
                </p>
              </div>
            </>
          )}
        </main>

        {workspacePath && isParametersPanelOpen && (
          <ParametersPanel
            workspacePath={workspacePath}
            onApplied={reloadPreview}
            onCollapse={() => setIsParametersPanelOpen(false)}
          />
        )}
        {workspacePath && !isParametersPanelOpen && (
          <button
            onClick={() => setIsParametersPanelOpen(true)}
            className="w-10 shrink-0 bg-slate-900/60 border-l border-slate-800 hover:bg-slate-800/40 transition-colors flex items-center justify-center text-slate-400 hover:text-slate-200"
            title="Show Parameters"
          >
            <SlidersHorizontal size={18} />
          </button>
        )}
      </div>

      {/* Snapshot History Side Panel */}
      {showHistory && (
        <SnapshotHistory
          workspacePath={workspacePath}
          onClose={() => setShowHistory(false)}
          onReloadPreview={reloadPreview}
          hasUnsavedChanges={hasUnsavedChanges}
        />
      )}

      {/* Settings Modal */}
      {showSettings && (
        <Settings onClose={() => setShowSettings(false)} currentProvider={provider} onProviderChange={handleProviderChange} />
      )}

      {/* Assets Modal */}
      {showAssets && workspacePath && (
        <AssetsModal workspacePath={workspacePath} previewUrl={previewUrl || undefined} onClose={() => setShowAssets(false)} />
      )}

      {/* First Run Wizard */}
      {showWizard && workspacePath && (
        <FirstRunWizard
          workspacePath={workspacePath}
          provider={provider}
          requestedPackages={missingPackages}
          onComplete={() => {
            setShowWizard(false);
            setWizardDismissed(false);
            setIsEnvReady(true);
          }}
          onClose={() => {
            setShowWizard(false);
            setWizardDismissed(true);
          }}
        />
      )}

      {/* Create Project Modal */}
      {showCreateProject && (
        <CreateProjectModal
          title={overview?.projects.length ? 'Create Project' : 'Create Your First Video'}
          defaultRoot={defaultProjectsRoot || overview?.projects_root || ''}
          initialRoot={overview?.projects_root}
          onPickRoot={(initial) => pickProjectsRoot(initial || undefined)}
          onCreate={async (root, name) => {
            await createProject(root, name);
            setShowCreateProject(false);
          }}
          onClose={() => setShowCreateProject(false)}
        />
      )}

      {/* Dependency Request Modal (missing npm packages detected by Gate) */}
      {showDependencyModal && workspacePath && missingPackages.length > 0 && (
        <div className="fixed inset-0 z-[60] flex items-center justify-center">
          <div className="absolute inset-0 bg-black/60" />
          <div className="relative w-full max-w-lg mx-4 rounded-2xl bg-slate-900 border border-slate-700 shadow-2xl p-6">
            <div className="text-lg font-black text-slate-100">
              AI thinks this video needs a dependency package that is not yet installed
            </div>
            <div className="text-sm text-slate-400 mt-2">
              Required packages:
              <div className="mt-2 max-h-40 overflow-auto rounded-xl bg-slate-950/60 border border-slate-800 p-3">
                <ul className="list-disc pl-5 space-y-1">
                  {missingPackages.map((p) => (
                    <li key={p} className="text-slate-200 font-semibold break-words">
                      {p}
                    </li>
                  ))}
                </ul>
              </div>
            </div>
            {isInstallingMissingPackages && (
              <div className="mt-4 flex items-center gap-2 text-sm text-indigo-300 font-bold">
                <RefreshCw size={16} className="animate-spin" />
                Installing… this may take a moment.
              </div>
            )}
            <div className="mt-5 flex items-center justify-end gap-3">
              <button
                onClick={() => setShowDependencyModal(false)}
                disabled={isInstallingMissingPackages}
                className="px-4 py-2 rounded-xl bg-slate-800/40 border border-slate-700 text-slate-200 font-bold hover:bg-slate-800 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                title="Skip for now"
              >
                Skip for now
              </button>
              <button
                onClick={async () => {
                  await handleRevertPreFlight({ skipConfirm: true });
                  setShowDependencyModal(false);
                }}
                disabled={isInstallingMissingPackages}
                className="px-4 py-2 rounded-xl bg-slate-900/50 border border-slate-700 text-slate-200 font-bold hover:bg-slate-900 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                title="Discard the last AI changes"
              >
                Revert pre-prompt
              </button>
              <button
                onClick={async () => {
                  try {
                    await handleInstallMissingPackages({ skipConfirm: true });
                    setShowDependencyModal(false);
                  } catch {
                    // handleInstallMissingPackages already alerts on failure.
                  }
                }}
                disabled={isInstallingMissingPackages}
                className="px-4 py-2 rounded-xl bg-indigo-600 text-white font-black hover:bg-indigo-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                title="Install packages required by the generated code"
              >
                {isInstallingMissingPackages ? 'Installing…' : 'Install dependencies'}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Status Bar */}
      <footer className="px-6 py-3 border-t border-slate-800 bg-slate-900 flex items-center justify-between text-xs font-medium text-slate-500">
        <div className="flex items-center gap-4">
          <span className="flex items-center gap-1.5">
            <span className={`w-2 h-2 rounded-full ${overallStatus.color} shadow-[0_0_8px_rgba(16,185,129,0.5)]`} />
            {overallStatus.text}
          </span>
          <span className="text-slate-700">|</span>
          <span className="truncate max-w-xs flex items-center gap-2">
            {workspacePath || 'No project'}
            {hasUnsavedChanges && (
              <span className="px-1.5 py-0.5 bg-amber-500/10 text-amber-500 rounded text-[10px] font-bold border border-amber-500/20">
                MODIFIED
              </span>
            )}
          </span>
        </div>
        <div className="flex items-center gap-4">
          {!isEnvReady && workspacePath && !showWizard && !linkingDeps && (
            <button
              onClick={() => {
                setWizardDismissed(false);
                setShowWizard(true);
              }}
              className="hidden lg:flex items-center gap-2 px-3 py-2 rounded-xl bg-amber-500/10 border border-amber-500/20 text-amber-200 hover:bg-amber-500/15 transition-colors"
              title="Open environment setup"
            >
              <span className="text-[11px] font-black uppercase tracking-widest">Setup</span>
              <span className="text-xs font-bold">Install missing deps</span>
            </button>
          )}
          {linkingDeps && linkingProgress && (
            <span className="hidden lg:flex items-center gap-2 px-3 py-2 rounded-xl bg-slate-800/40 border border-slate-700 text-slate-200 max-w-[420px]">
              <span className="text-[11px] font-black uppercase tracking-widest text-slate-400">Deps</span>
              <div className="min-w-0 flex-1">
                <div className="text-[11px] font-bold truncate">{linkingProgress.step}</div>
                <div className="w-full bg-slate-800 rounded-full h-1.5 mt-1 overflow-hidden border border-slate-700">
                  <div
                    className="bg-indigo-500 h-full transition-all duration-300 ease-out"
                    style={{ width: `${linkingProgress.percent}%` }}
                  />
                </div>
              </div>
              <div className="text-[11px] text-slate-400 font-bold tabular-nums shrink-0">
                {Math.round(linkingProgress.percent)}%
              </div>
            </span>
          )}
          {gateFailed && missingPackages.length > 0 && !isProcessing && !showDependencyModal && (
            <button
              onClick={() => setShowDependencyModal(true)}
              className="hidden lg:flex items-center gap-2 px-3 py-2 rounded-xl bg-indigo-600/20 border border-indigo-500/30 text-indigo-200 hover:bg-indigo-600/25 transition-colors"
              title="Install missing packages for the generated code"
            >
              <span className="text-[11px] font-black uppercase tracking-widest">Deps</span>
              <span className="text-xs font-bold">Install Missing ({missingPackages.length})</span>
            </button>
          )}
          {lastResponse?.gate_result && (
            <span
              className={`px-2 py-0.5 rounded-full ${
                lastResponse.gate_result.passed ? 'bg-emerald-500/20 text-emerald-400' : 'bg-amber-500/20 text-amber-400'
              }`}
            >
              G: {lastResponse.gate_result.passed ? 'PASS' : missingPackages.length > 0 ? 'NEEDS DEPS' : 'FAIL'}
              {lastResponse.gate_result.typecheck ? ' T' : ''}
              {lastResponse.gate_result.smoke_0 ? ' S0' : ''}
              {lastResponse.gate_result.smoke_mid ? ' SM' : ''}
            </span>
          )}
          {lastResponse?.snapshot_id && <span className="text-slate-600">SN: {lastResponse.snapshot_id.substring(0, 8)}...</span>}
          <span>Preview: {previewStatus === 'running' ? `Port ${port}` : previewStatus}</span>
          <span className="text-slate-700">|</span>
          <span>AI: {openCodeStatus === 'running' ? 'Ready' : openCodeStatus}</span>
        </div>
      </footer>
    </div>
  );
}

export default App;
