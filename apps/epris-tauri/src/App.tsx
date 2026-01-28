import {
  Clock,
  Download,
  FolderOpen,
  Plus,
  Pencil,
  RefreshCw,
  Save,
  Send,
  SlidersHorizontal,
  Settings as SettingsIcon,
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { openPath } from '@tauri-apps/plugin-opener';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { CreateProjectModal } from './components/CreateProjectModal';
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

function App() {
  const [promptInput, setPromptInput] = useState('');
  const [iframeKey, setIframeKey] = useState(0);
  const [showHistory, setShowHistory] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const [showWizard, setShowWizard] = useState(false);
  const [isEnvReady, setIsEnvReady] = useState(false);

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

  const { status: previewStatus, port } = usePreviewServer(workspacePath);
  const { status: openCodeStatus } = useOpenCode(workspacePath);
  const previewUrl = port ? `http://127.0.0.1:${port}` : null;

  const { waitPort } = useWaitPort();
  const reloadPreview = useCallback(async () => {
    if (previewUrl) {
      await waitPort(previewUrl);
    }
    setIframeKey((k) => k + 1);
  }, [previewUrl, waitPort]);

  const {
    status: promptStatus,
    progress: promptProgress,
    currentStep: promptCurrentStep,
    error: promptError,
    lastResponse,
    sendPrompt,
    clearSession,
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
    const checkEnv = async () => {
      try {
        const status = await invoke<any>('get_environment_status', {
          provider,
          workspacePath,
        });
        const ready =
          status.node_valid &&
          status.pnpm_valid &&
          status.provider_cli_valid &&
          status.workspace_deps_valid &&
          status.skills_valid;
        setIsEnvReady(ready);
        if (!ready) setShowWizard(true);
      } catch (e) {
        console.error('Failed to check env:', e);
      }
    };
    checkEnv();
  }, [workspacePath, provider]);

  // Step 13: default provider when opening a project
  useEffect(() => {
    setIsEnvReady(false);
    setShowWizard(false);
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
        await invoke('stop_preview_server');
      } catch {}
      try {
        await invoke('stop_opencode');
      } catch {}
      await setActiveProject(projectId);
      setIframeKey((k) => k + 1);
    },
    [overview?.active_project_id, setActiveProject],
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
        return { text: 'Gate Failed', color: 'bg-amber-500' };
      }
      return { text: 'Ready', color: 'bg-green-500' };
    }
    if (exportStatus === 'exporting') return { text: 'Exporting...', color: 'bg-purple-500' };
    if (exportStatus === 'done') return { text: 'Export Complete', color: 'bg-green-500' };
    if (exportStatus === 'error') return { text: 'Export Error', color: 'bg-red-500' };
    if (previewStatus === 'running' && openCodeStatus === 'running') return { text: 'Ready', color: 'bg-emerald-500' };
    return { text: 'Loading...', color: 'bg-slate-500' };
  }, [workspacePath, previewStatus, openCodeStatus, promptStatus, exportStatus, lastResponse]);

  const isReady = Boolean(workspacePath) && previewStatus === 'running' && openCodeStatus === 'running' && isEnvReady;
  const isProcessing = promptStatus === 'sending';
  const hasGateError = promptStatus === 'error';

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
                {previewUrl ? (
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

                {/* Processing Overlay */}
                {isProcessing && (
                  <div className="absolute inset-0 bg-slate-900/80 flex items-center justify-center z-30">
                    <div className="text-center w-full max-w-md px-10">
                      <RefreshCw size={48} className="animate-spin mx-auto mb-6 text-indigo-400" />
                      <div className="text-xl font-semibold text-white mb-2">
                        {promptCurrentStep || 'Generating Animation...'}
                      </div>
                      <div className="w-full bg-slate-800 rounded-full h-2 mb-4 overflow-hidden border border-slate-700">
                        <div
                          className="bg-indigo-500 h-full transition-all duration-500 ease-out"
                          style={{ width: `${promptProgress}%` }}
                        />
                      </div>
                      <div className="text-xs text-slate-500 uppercase tracking-widest font-bold tracking-widest">
                        {Math.round(promptProgress)}% Complete
                      </div>
                    </div>
                  </div>
                )}
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
                      className="w-full bg-slate-800 border-2 border-slate-700 rounded-2xl px-6 py-4 pr-16 text-lg focus:outline-none focus:border-indigo-500 transition-all placeholder:text-slate-500 shadow-xl disabled:opacity-50"
                      onKeyDown={(e) => e.key === 'Enter' && !isProcessing && handleSubmit()}
                      disabled={!isReady || isProcessing}
                    />
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

      {/* First Run Wizard */}
      {showWizard && workspacePath && (
        <FirstRunWizard
          workspacePath={workspacePath}
          provider={provider}
          onComplete={() => {
            setShowWizard(false);
            setIsEnvReady(true);
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
          {lastResponse?.gate_result && (
            <span
              className={`px-2 py-0.5 rounded-full ${
                lastResponse.gate_result.passed ? 'bg-emerald-500/20 text-emerald-400' : 'bg-amber-500/20 text-amber-400'
              }`}
            >
              G: {lastResponse.gate_result.passed ? 'PASS' : 'FAIL'}
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
