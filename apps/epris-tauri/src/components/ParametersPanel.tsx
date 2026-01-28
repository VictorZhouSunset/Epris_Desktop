import { useCallback, useEffect, useMemo, useState } from 'react';
import { SlidersHorizontal, RotateCcw, Check } from 'lucide-react';
import { IpcService } from '../lib/ipc';
import { VideoConfig } from '../types/backend';

interface ParametersPanelProps {
  workspacePath: string;
  onApplied: () => void;
}

const PRESETS = [
  { id: '16:9', label: '16:9', w: 1920, h: 1080 },
  { id: '9:16', label: '9:16', w: 1080, h: 1920 },
  { id: '1:1', label: '1:1', w: 1080, h: 1080 },
  { id: '4:3', label: '4:3', w: 1440, h: 1080 },
];

export function ParametersPanel({ workspacePath, onApplied }: ParametersPanelProps) {
  const [config, setConfig] = useState<VideoConfig | null>(null);
  const [width, setWidth] = useState<number>(1920);
  const [height, setHeight] = useState<number>(1080);
  const [durationSeconds, setDurationSeconds] = useState<number>(5);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const c = await IpcService.call<VideoConfig>('GET_VIDEO_CONFIG', { workspacePath });
      setConfig(c);
      setWidth(c.width);
      setHeight(c.height);
      setDurationSeconds(Math.max(0.1, Number(c.duration_seconds.toFixed(2))));
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }, [workspacePath]);

  useEffect(() => {
    void load();
  }, [load]);

  const activePresetId = useMemo(() => {
    return PRESETS.find((p) => p.w === width && p.h === height)?.id ?? null;
  }, [width, height]);

  const apply = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const updated = await IpcService.call<VideoConfig>('SET_VIDEO_CONFIG', {
        workspacePath,
        width,
        height,
        durationSeconds,
      });
      setConfig(updated);
      onApplied();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }, [workspacePath, width, height, durationSeconds, onApplied]);

  return (
    <aside className="w-80 shrink-0 bg-slate-900/60 border-l border-slate-800 flex flex-col min-h-0">
      <div className="p-4 border-b border-slate-800 flex items-center justify-between">
        <div className="flex items-center gap-2 text-slate-200 font-bold">
          <SlidersHorizontal size={18} className="text-indigo-400" />
          Parameters
        </div>
        <button
          onClick={load}
          className="p-2 rounded-xl hover:bg-slate-800 text-slate-400 hover:text-slate-200 transition-colors"
          title="Reload"
          disabled={busy}
        >
          <RotateCcw size={16} />
        </button>
      </div>

      <div className="p-4 flex-1 overflow-y-auto space-y-6">
        <div className="space-y-2">
          <div className="text-xs uppercase font-black text-slate-500 tracking-widest">Aspect Ratio</div>
          <div className="grid grid-cols-2 gap-2">
            {PRESETS.map((p) => (
              <button
                key={p.id}
                onClick={() => {
                  setWidth(p.w);
                  setHeight(p.h);
                }}
                className={`p-3 rounded-xl border text-left transition-colors ${
                  activePresetId === p.id
                    ? 'bg-indigo-600/15 border-indigo-500/25 text-white'
                    : 'bg-slate-950/30 border-slate-800 text-slate-300 hover:bg-slate-800/40'
                }`}
                disabled={busy}
              >
                <div className="font-black">{p.label}</div>
                <div className="text-[11px] text-slate-500">{p.w} × {p.h}</div>
              </button>
            ))}
          </div>
        </div>

        <div className="grid grid-cols-2 gap-3">
          <div className="space-y-2">
            <div className="text-xs uppercase font-black text-slate-500 tracking-widest">Width</div>
            <input
              type="number"
              min={1}
              value={width}
              onChange={(e) => setWidth(Number(e.target.value))}
              className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
              disabled={busy}
            />
          </div>
          <div className="space-y-2">
            <div className="text-xs uppercase font-black text-slate-500 tracking-widest">Height</div>
            <input
              type="number"
              min={1}
              value={height}
              onChange={(e) => setHeight(Number(e.target.value))}
              className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
              disabled={busy}
            />
          </div>
        </div>

        <div className="space-y-2">
          <div className="text-xs uppercase font-black text-slate-500 tracking-widest">Duration (seconds)</div>
          <input
            type="number"
            min={0.1}
            step={0.1}
            value={durationSeconds}
            onChange={(e) => setDurationSeconds(Number(e.target.value))}
            className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
            disabled={busy}
          />
          <div className="text-[11px] text-slate-500">
            FPS is fixed to {config?.fps ?? 30}. Export resolution is determined at export time.
          </div>
        </div>

        {error && (
          <div className="p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-red-400 text-sm">
            {error}
          </div>
        )}
      </div>

      <div className="p-4 border-t border-slate-800">
        <button
          onClick={apply}
          disabled={busy || width <= 0 || height <= 0 || durationSeconds <= 0}
          className="w-full px-4 py-3 bg-indigo-600 hover:bg-indigo-500 text-white rounded-2xl font-black transition-colors shadow-lg shadow-indigo-500/10 disabled:opacity-50 flex items-center justify-center gap-2"
        >
          <Check size={18} />
          Apply
        </button>
      </div>
    </aside>
  );
}

