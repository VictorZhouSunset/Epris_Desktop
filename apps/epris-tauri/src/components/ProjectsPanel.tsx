import { Folder, Plus, Trash2, ChevronLeft, Pencil } from 'lucide-react';
import { ProjectInfo } from '../types/backend';

interface ProjectsPanelProps {
  projects: ProjectInfo[];
  activeProjectId?: string;
  onCollapse: () => void;
  onCreate: () => void;
  onSelect: (projectId: string) => void;
  onDelete: (projectId: string) => void;
  onRename: (projectId: string) => void;
}

export function ProjectsPanel({
  projects,
  activeProjectId,
  onCollapse,
  onCreate,
  onSelect,
  onDelete,
  onRename,
}: ProjectsPanelProps) {
  return (
    <aside className="relative w-72 shrink-0 bg-slate-900/60 border-r border-slate-800 flex flex-col min-h-0">
      <div className="p-4 border-b border-slate-800 flex items-center justify-between">
        <div className="flex items-center gap-2 text-slate-200 font-bold">
          <Folder size={18} className="text-indigo-400" />
          Projects
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={onCreate}
            className="p-2 rounded-xl bg-indigo-600/20 hover:bg-indigo-600/30 border border-indigo-500/20 text-indigo-300 transition-colors"
            title="Create Project"
          >
            <Plus size={16} />
          </button>
        </div>
      </div>

      <button
        onClick={onCollapse}
        className="absolute top-1/2 right-0 -translate-y-1/2 translate-x-1/2 w-8 h-14 rounded-xl bg-slate-900/80 border border-slate-800 hover:bg-slate-800/60 text-slate-400 hover:text-slate-200 transition-colors flex items-center justify-center shadow-lg z-20"
        title="Collapse Projects"
      >
        <ChevronLeft size={16} />
      </button>

      <div className="p-2 flex-1 overflow-y-auto">
        {projects.length === 0 ? (
          <div className="p-4 text-sm text-slate-500">No projects yet.</div>
        ) : (
          <div className="space-y-1">
            {projects.map((p) => {
              const active = p.id === activeProjectId;
              return (
                <button
                  key={p.id}
                  onClick={() => onSelect(p.id)}
                  className={`w-full flex items-center justify-between gap-2 px-3 py-3 rounded-xl border transition-colors text-left ${
                    active
                      ? 'bg-indigo-600/15 border-indigo-500/25 text-white'
                      : 'bg-slate-950/30 border-slate-800 text-slate-300 hover:bg-slate-800/40'
                  }`}
                  title={p.path}
                >
                  <div className="min-w-0">
                    <div className="font-bold text-sm truncate">{p.name}</div>
                    <div className="text-[11px] text-slate-500 truncate">{p.path}</div>
                  </div>
                  <div className="flex items-center gap-1">
                    <span className="text-[10px] font-black text-slate-500 uppercase tracking-widest">
                      {p.provider}
                    </span>
                    <span className="w-px h-5 bg-slate-800 mx-1" />
                    <span
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        onRename(p.id);
                      }}
                      className="p-1.5 rounded-lg hover:bg-slate-700/60 text-slate-500 hover:text-slate-200 transition-colors"
                      role="button"
                      title="Rename project"
                    >
                      <Pencil size={14} />
                    </span>
                    <span
                      onClick={(e) => {
                        e.preventDefault();
                        e.stopPropagation();
                        onDelete(p.id);
                      }}
                      className="p-1.5 rounded-lg hover:bg-red-500/10 text-slate-500 hover:text-red-400 transition-colors"
                      role="button"
                      title="Delete project"
                    >
                      <Trash2 size={14} />
                    </span>
                  </div>
                </button>
              );
            })}
          </div>
        )}
      </div>
    </aside>
  );
}
