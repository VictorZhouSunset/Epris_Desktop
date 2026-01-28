import { useEffect, useState } from 'react';
import { X, FolderOpen, Plus } from 'lucide-react';

interface CreateProjectModalProps {
  title: string;
  defaultRoot: string;
  initialRoot?: string;
  onPickRoot: (initial?: string) => Promise<string | null>;
  onCreate: (projectsRoot: string, name?: string) => Promise<void>;
  onClose: () => void;
}

export function CreateProjectModal({
  title,
  defaultRoot,
  initialRoot,
  onPickRoot,
  onCreate,
  onClose,
}: CreateProjectModalProps) {
  const [projectsRoot, setProjectsRoot] = useState(initialRoot || defaultRoot);
  const [name, setName] = useState('My Video');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setProjectsRoot(initialRoot || defaultRoot);
  }, [initialRoot, defaultRoot]);

  const handleBrowse = async () => {
    try {
      const picked = await onPickRoot(projectsRoot);
      if (picked) setProjectsRoot(picked);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleCreate = async () => {
    if (!projectsRoot.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await onCreate(projectsRoot, name.trim() || undefined);
      onClose();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[90] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-slate-950/80 backdrop-blur-md" onClick={onClose} />
      <div className="relative w-full max-w-xl bg-slate-900 border border-slate-700 rounded-3xl p-8 shadow-3xl flex flex-col gap-6">
        <header className="flex items-center justify-between">
          <h2 className="text-2xl font-bold text-white flex items-center gap-2">
            <Plus className="text-indigo-400" />
            {title}
          </h2>
          <button onClick={onClose} className="p-2 hover:bg-slate-800 rounded-full text-slate-400 hover:text-white transition-colors">
            <X size={24} />
          </button>
        </header>

        <div className="space-y-4">
          <div className="space-y-2">
            <label className="text-xs uppercase font-black text-slate-500 tracking-widest">Project Name</label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="My Video"
              className="w-full bg-slate-950 border border-slate-700 rounded-xl px-4 py-3 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
              disabled={busy}
            />
          </div>

          <div className="space-y-2">
            <label className="text-xs uppercase font-black text-slate-500 tracking-widest">Projects Folder</label>
            <div className="flex gap-2">
              <input
                value={projectsRoot}
                onChange={(e) => setProjectsRoot(e.target.value)}
                placeholder={defaultRoot}
                className="flex-1 bg-slate-950 border border-slate-700 rounded-xl px-4 py-3 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
                disabled={busy}
              />
              <button
                onClick={handleBrowse}
                className="px-4 py-3 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-xl text-slate-200 transition-colors flex items-center gap-2"
                disabled={busy}
                title="Browse"
              >
                <FolderOpen size={16} />
                Browse
              </button>
            </div>
            <p className="text-xs text-slate-500">
              Tip: you can just click Create to use the default folder.
            </p>
          </div>

          {error && (
            <div className="p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-red-400 text-sm">
              {error}
            </div>
          )}
        </div>

        <div className="flex items-center justify-end gap-3 pt-2">
          <button
            onClick={onClose}
            className="px-5 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-xl font-bold transition-colors"
            disabled={busy}
          >
            Cancel
          </button>
          <button
            onClick={handleCreate}
            className="px-6 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl font-bold transition-colors shadow-lg shadow-indigo-500/10 disabled:opacity-50"
            disabled={busy || !projectsRoot.trim()}
          >
            {busy ? 'Creating…' : 'Create'}
          </button>
        </div>
      </div>
    </div>
  );
}

