import { useEffect, useState } from 'react';
import { Save, X } from 'lucide-react';

interface SaveSnapshotModalProps {
  isOpen: boolean;
  title?: string;
  defaultName: string;
  defaultDescription?: string;
  busy?: boolean;
  error?: string | null;
  onClose: () => void;
  onSubmit: (name: string, description: string) => Promise<void> | void;
}

export function SaveSnapshotModal({
  isOpen,
  title = 'Save Snapshot',
  defaultName,
  defaultDescription = '',
  busy = false,
  error,
  onClose,
  onSubmit,
}: SaveSnapshotModalProps) {
  const [name, setName] = useState(defaultName);
  const [description, setDescription] = useState(defaultDescription);

  useEffect(() => {
    if (!isOpen) return;
    setName(defaultName);
    setDescription(defaultDescription);
  }, [isOpen, defaultName, defaultDescription]);

  if (!isOpen) return null;

  const canSubmit = !busy && Boolean(name.trim());

  return (
    <div className="fixed inset-0 z-[95] flex items-center justify-center p-4">
      <div className="absolute inset-0 bg-slate-950/80 backdrop-blur-md" onClick={busy ? undefined : onClose} />
      <div className="relative w-full max-w-xl bg-slate-900 border border-slate-700 rounded-3xl p-8 shadow-3xl">
        <header className="flex items-center justify-between mb-6">
          <h2 className="text-2xl font-bold text-white flex items-center gap-2">
            <Save className="text-indigo-400" size={20} />
            {title}
          </h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-slate-800 rounded-full text-slate-400 hover:text-white transition-colors disabled:opacity-50"
            disabled={busy}
            aria-label="Close"
          >
            <X size={22} />
          </button>
        </header>

        <form
          className="space-y-5"
          onSubmit={(e) => {
            e.preventDefault();
            if (!canSubmit) return;
            void onSubmit(name.trim(), description.trim());
          }}
        >
          <div className="space-y-2">
            <label className="text-xs uppercase font-black text-slate-500 tracking-widest">
              Name <span className="text-red-400">*</span>
            </label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Manual snapshot"
              className="w-full bg-slate-950 border border-slate-700 rounded-xl px-4 py-3 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
              disabled={busy}
              autoFocus
            />
          </div>

          <div className="space-y-2">
            <label className="text-xs uppercase font-black text-slate-500 tracking-widest">Description</label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Optional"
              className="w-full min-h-[120px] bg-slate-950 border border-slate-700 rounded-xl px-4 py-3 text-sm text-white outline-none focus:border-indigo-500 transition-colors resize-y"
              disabled={busy}
            />
          </div>

          {error && (
            <div className="p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-red-400 text-sm">
              {error}
            </div>
          )}

          <div className="flex items-center justify-end gap-3 pt-1">
            <button
              type="button"
              onClick={onClose}
              className="px-5 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded-xl font-bold transition-colors disabled:opacity-50"
              disabled={busy}
            >
              Cancel
            </button>
            <button
              type="submit"
              className="px-6 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl font-bold transition-colors shadow-lg shadow-indigo-500/10 disabled:opacity-50"
              disabled={!canSubmit}
            >
              {busy ? 'Saving…' : 'Save'}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}
