import { useCallback, useEffect, useMemo, useState } from 'react';
import { SlidersHorizontal, RotateCcw, Check, ChevronRight } from 'lucide-react';
import { IpcService } from '../lib/ipc';
import { PromptResponse, VideoConfig } from '../types/backend';
import type { UiPropsActions, UiPropsLoadState } from '../hooks/useUiProps';
import type { EprisControlSpec } from '../types/ui-controls';
import { ColorControl, NumberControl, SelectControl, TextControl, ToggleControl } from './controls';

interface ParametersPanelProps {
  workspacePath: string;
  onApplied: () => void;
  onCollapse: () => void;
  provider: string;
  model: string;
  uiPropsState: UiPropsLoadState;
  uiPropsActions: UiPropsActions;
  objectsSnapshot: any[] | null;
  onScanObjects: () => void;
}

const PRESETS = [
  { id: '16:9', label: '16:9', w: 1920, h: 1080 },
  { id: '9:16', label: '9:16', w: 1080, h: 1920 },
  { id: '1:1', label: '1:1', w: 1080, h: 1080 },
  { id: '4:3', label: '4:3', w: 1440, h: 1080 },
];

export function ParametersPanel({
  workspacePath,
  onApplied,
  onCollapse,
  provider,
  model,
  uiPropsState,
  uiPropsActions,
  objectsSnapshot,
  onScanObjects,
}: ParametersPanelProps) {
  const [uiActionError, setUiActionError] = useState<string | null>(null);
  const [selectedObjectId, setSelectedObjectId] = useState<string | null>(null);
  const [uiAgentPrompt, setUiAgentPrompt] = useState('');
  const [uiAgentBusy, setUiAgentBusy] = useState(false);
  const [uiAgentError, setUiAgentError] = useState<string | null>(null);
  const [uiAgentLast, setUiAgentLast] = useState<string | null>(null);
  const [uiAgentOpen, setUiAgentOpen] = useState(false);
  const [config, setConfig] = useState<VideoConfig | null>(null);
  const [widthInput, setWidthInput] = useState<string>('1920');
  const [heightInput, setHeightInput] = useState<string>('1080');
  const [durationSecondsInput, setDurationSecondsInput] = useState<string>('5');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const width = useMemo(() => Number.parseInt(widthInput, 10), [widthInput]);
  const height = useMemo(() => Number.parseInt(heightInput, 10), [heightInput]);
  const durationSeconds = useMemo(() => Number.parseFloat(durationSecondsInput), [durationSecondsInput]);

  const objects = useMemo(() => {
    const arr = Array.isArray(objectsSnapshot) ? objectsSnapshot : [];
    return arr
      .filter((o: any) => o && typeof o === 'object')
      .map((o: any) => ({
        id: String(o.id || ''),
        label: String(o.label || o.id || (String(o.id || '') === 'root' ? 'Root' : 'Object')),
        kind: o.kind != null ? String(o.kind) : '',
        tags: Array.isArray(o.tags) ? o.tags.map(String) : [],
      }))
      .filter((o) => Boolean(o.id));
  }, [objectsSnapshot]);

  useEffect(() => {
    if (selectedObjectId && objects.some((o) => o.id === selectedObjectId)) return;
    setSelectedObjectId(objects.length ? objects[0].id : null);
  }, [objects, selectedObjectId]);

  const load = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const c = await IpcService.call<VideoConfig>('GET_VIDEO_CONFIG', { workspacePath });
      setConfig(c);
      setWidthInput(String(c.width));
      setHeightInput(String(c.height));
      setDurationSecondsInput(String(Math.max(1, Number(c.duration_seconds.toFixed(2)))));
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
    if (!Number.isFinite(width) || !Number.isFinite(height)) return null;
    return PRESETS.find((p) => p.w === width && p.h === height)?.id ?? null;
  }, [width, height]);

  const apply = useCallback(async () => {
    setBusy(true);
    setError(null);
    try {
      const safeWidth = Number.isFinite(width) && width > 0 ? width : 1920;
      const safeHeight = Number.isFinite(height) && height > 0 ? height : 1080;
      const safeDuration = Number.isFinite(durationSeconds) && durationSeconds > 0 ? durationSeconds : 1;

      const updated = await IpcService.call<VideoConfig>('SET_VIDEO_CONFIG', {
        workspacePath,
        width: safeWidth,
        height: safeHeight,
        durationSeconds: Math.max(1, safeDuration),
      });
      setConfig(updated);
      setWidthInput(String(updated.width));
      setHeightInput(String(updated.height));
      setDurationSecondsInput(String(Math.max(1, Number(updated.duration_seconds.toFixed(2)))));
      onApplied();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }, [workspacePath, width, height, durationSeconds, onApplied]);

  const canApply =
    Number.isFinite(width) &&
    width > 0 &&
    Number.isFinite(height) &&
    height > 0 &&
    Number.isFinite(durationSeconds) &&
    durationSeconds > 0;

  const handleKeyDownApply = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        if (!busy && canApply) void apply();
      }
    },
    [apply, busy, canApply],
  );

  const sendUiAgentPrompt = useCallback(async () => {
    const text = uiAgentPrompt.trim();
    if (!text) return;
    setUiAgentBusy(true);
    setUiAgentError(null);
    setUiAgentLast(null);
    try {
      const res = await IpcService.call<PromptResponse>('SEND_UI_PROMPT', {
        workspacePath,
        prompt: text,
        provider,
        model,
        draftProps: uiPropsState.draftValues,
      });
      setUiAgentLast(res.message || (res.success ? 'Done' : 'Failed'));
      if (res.success && !res.canceled) {
        setUiAgentPrompt('');
      }
      await uiPropsActions.reload();
      onScanObjects();
      onApplied();
    } catch (e) {
      setUiAgentError(e instanceof Error ? e.message : String(e));
    } finally {
      setUiAgentBusy(false);
    }
  }, [uiAgentPrompt, workspacePath, provider, model, uiPropsState.draftValues, uiPropsActions, onScanObjects, onApplied]);

  const controls = uiPropsState.controlsSpec?.controls ?? [];

  const globalControls = useMemo(() => controls.filter((c) => !c.objectId), [controls]);
  const selectedControls = useMemo(
    () => (selectedObjectId ? controls.filter((c) => c.objectId === selectedObjectId) : []),
    [controls, selectedObjectId],
  );

  const visibleObjects = useMemo(() => {
    if (objects.length <= 1) return objects;
    const nonRoot = objects.filter((o) => o.id !== 'root');
    return nonRoot.length ? nonRoot : objects;
  }, [objects]);

  const groupControls = useCallback((list: EprisControlSpec[]) => {
    const groups = new Map<string, EprisControlSpec[]>();
    for (const c of list) {
      const g = c.group ?? 'General';
      const arr = groups.get(g) ?? [];
      arr.push(c);
      groups.set(g, arr);
    }
    return Array.from(groups.entries())
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([name, items]) => [
        name,
        items.slice().sort((a, b) => a.label.localeCompare(b.label)),
      ]) as Array<[string, EprisControlSpec[]]>;
  }, []);

  const selectedObject = useMemo(
    () => (selectedObjectId ? objects.find((o) => o.id === selectedObjectId) ?? null : null),
    [objects, selectedObjectId],
  );

  const renderControl = useCallback(
    (c: EprisControlSpec) => {
      const v = uiPropsState.draftValues[c.id];
      if (c.type === 'number') {
        const num = typeof v === 'number' ? v : typeof c.min === 'number' ? c.min : 0;
        return (
          <NumberControl
            key={c.id}
            label={c.label}
            value={num}
            onChange={(next) => uiPropsActions.setDraftValue(c.id, next)}
            ui={c.ui}
            min={c.min}
            max={c.max}
            step={c.step}
            disabled={uiPropsState.loading}
          />
        );
      }
      if (c.type === 'color') {
        const color = typeof v === 'string' && v ? v : '#ffffff';
        return (
          <ColorControl
            key={c.id}
            label={c.label}
            value={color}
            onChange={(next) => uiPropsActions.setDraftValue(c.id, next)}
            disabled={uiPropsState.loading}
          />
        );
      }
      if (c.type === 'select') {
        const first = c.options[0];
        const firstValue = typeof first === 'string' ? first : first.value;
        const current = typeof v === 'string' ? v : firstValue;
        return (
          <SelectControl
            key={c.id}
            label={c.label}
            value={current}
            options={c.options}
            onChange={(next) => uiPropsActions.setDraftValue(c.id, next)}
            disabled={uiPropsState.loading}
          />
        );
      }
      if (c.type === 'boolean') {
        const b = typeof v === 'boolean' ? v : false;
        return (
          <ToggleControl
            key={c.id}
            label={c.label}
            value={b}
            onChange={(next) => uiPropsActions.setDraftValue(c.id, next)}
            disabled={uiPropsState.loading}
          />
        );
      }
      if (c.type === 'text') {
        const s = typeof v === 'string' ? v : '';
        return (
          <TextControl
            key={c.id}
            label={c.label}
            value={s}
            placeholder={c.placeholder}
            onChange={(next) => uiPropsActions.setDraftValue(c.id, next)}
            disabled={uiPropsState.loading}
          />
        );
      }
      return null;
    },
    [uiPropsActions, uiPropsState.draftValues, uiPropsState.loading],
  );

  return (
    <aside className="relative w-80 shrink-0 bg-slate-900/60 border-l border-slate-800 flex flex-col min-h-0">
      <div className="p-4 border-b border-slate-800 flex items-center justify-between">
        <div className="flex items-center gap-2 text-slate-200 font-bold">
          <SlidersHorizontal size={18} className="text-indigo-400" />
          Parameters
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={load}
            className="p-2 rounded-xl hover:bg-slate-800 text-slate-400 hover:text-slate-200 transition-colors"
            title="Reload"
            disabled={busy}
          >
            <RotateCcw size={16} />
          </button>
        </div>
      </div>

      <button
        onClick={onCollapse}
        className="absolute top-1/2 left-0 -translate-y-1/2 -translate-x-1/2 w-8 h-14 rounded-xl bg-slate-900/80 border border-slate-800 hover:bg-slate-800/60 text-slate-400 hover:text-slate-200 transition-colors flex items-center justify-center shadow-lg z-20"
        title="Collapse Parameters"
      >
        <ChevronRight size={16} />
      </button>

      <div className="p-4 flex-1 overflow-y-auto space-y-5">
        <div className="space-y-3 rounded-2xl bg-slate-950/30 border border-slate-800 p-3">
          <div className="flex items-center justify-between gap-2">
            <div className="text-xs uppercase font-black text-slate-500 tracking-widest">Video</div>
            {uiPropsState.dirty && <div className="text-[11px] font-bold text-amber-400">Unsaved</div>}
          </div>

          <div className="grid grid-cols-2 gap-2">
            <div className="space-y-1">
              <div className="text-[11px] font-black text-slate-500 uppercase tracking-widest">Preset</div>
              <select
                value={activePresetId ?? 'custom'}
                onChange={(e) => {
                  const preset = PRESETS.find((p) => p.id === e.target.value);
                  if (!preset) return;
                  setWidthInput(String(preset.w));
                  setHeightInput(String(preset.h));
                }}
                className="w-full bg-slate-950 border border-slate-700 rounded-xl px-2 py-2 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
                disabled={busy}
              >
                <option value="custom">Custom</option>
                {PRESETS.map((p) => (
                  <option key={p.id} value={p.id}>
                    {p.label}
                  </option>
                ))}
              </select>
            </div>
            <div className="space-y-1">
              <div className="text-[11px] font-black text-slate-500 uppercase tracking-widest">Duration (s)</div>
              <input
                type="number"
                min={1}
                step={0.1}
                value={durationSecondsInput}
                onChange={(e) => setDurationSecondsInput(e.target.value)}
                onKeyDown={handleKeyDownApply}
                onBlur={() => {
                  if (durationSecondsInput.trim() === '') {
                    setDurationSecondsInput('1');
                    return;
                  }
                  const v = Number.parseFloat(durationSecondsInput);
                  if (!Number.isFinite(v) || v <= 0) setDurationSecondsInput('1');
                }}
                className="w-full bg-slate-950 border border-slate-700 rounded-xl px-2 py-2 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
                disabled={busy}
              />
            </div>
          </div>

          <div className="text-[11px] text-slate-500">
            FPS is fixed to {config?.fps ?? 30}. Export resolution is determined at export time.
          </div>

          <button
            onClick={apply}
            disabled={busy || !canApply}
            className="w-full px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl font-black transition-colors shadow-lg shadow-indigo-500/10 disabled:opacity-50 flex items-center justify-center gap-2"
          >
            <Check size={16} />
            Apply
          </button>
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between gap-2">
            <div className="text-xs uppercase font-black text-slate-500 tracking-widest">Objects</div>
            <button
              onClick={onScanObjects}
              className="px-3 py-2 rounded-xl bg-slate-950/30 border border-slate-800 text-slate-300 hover:bg-slate-800/40 transition-colors text-xs font-bold"
            >
              Scan
            </button>
          </div>

          {objects.length === 0 ? (
            <div className="text-sm text-slate-500">
              No objects found. Wrap elements with `EprisGroup` (id/label/kind) to make them scannable.
            </div>
          ) : (
            <div className="space-y-1">
              {visibleObjects.map((o) => (
                <button
                  key={o.id}
                  onClick={() => setSelectedObjectId(o.id)}
                  className={`w-full px-3 py-2 rounded-xl border text-left transition-colors ${
                    selectedObjectId === o.id
                      ? 'bg-indigo-600/15 border-indigo-500/25 text-white'
                      : 'bg-slate-950/30 border-slate-800 text-slate-300 hover:bg-slate-800/40'
                  }`}
                >
                  <div className="font-black text-sm truncate">{o.label}</div>
                  <div className="text-[11px] text-slate-500 truncate">
                    {o.id} {o.kind ? `• ${o.kind}` : ''}
                  </div>
                </button>
              ))}
            </div>
          )}
        </div>

        <div className="space-y-3">
          <div className="flex items-center justify-between gap-2">
            <div className="text-xs uppercase font-black text-slate-500 tracking-widest">
              {selectedObject ? `Properties • ${selectedObject.label}` : 'Properties'}
            </div>
            <div className="flex items-center gap-2">
              <button
                onClick={() => uiPropsActions.reload().catch((e) => setUiActionError(e instanceof Error ? e.message : String(e)))}
                className="px-3 py-2 rounded-xl bg-slate-950/30 border border-slate-800 text-slate-300 hover:bg-slate-800/40 transition-colors text-xs font-bold"
                disabled={uiPropsState.loading}
                title="Reload controls/props from disk"
              >
                Reload
              </button>
              <button
                onClick={() => uiPropsActions.revertDraft()}
                className="px-3 py-2 rounded-xl bg-slate-950/30 border border-slate-800 text-slate-300 hover:bg-slate-800/40 transition-colors text-xs font-bold"
                disabled={!uiPropsState.dirty || uiPropsState.loading}
                title="Revert to last saved props (top Save persists)"
              >
                Revert
              </button>
            </div>
          </div>

          {(uiPropsState.error || uiActionError) && (
            <div className="p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-red-400 text-sm whitespace-pre-wrap">
              {uiPropsState.error || uiActionError}
            </div>
          )}

          {!uiPropsState.error && (
            <div className="space-y-5">
              {globalControls.length > 0 && (
                <div className="space-y-3">
                  <div className="text-[11px] font-black text-slate-500 uppercase tracking-widest">Global</div>
                  <div className="space-y-4">{globalControls.map(renderControl)}</div>
                </div>
              )}

              {selectedObjectId ? (
                selectedControls.length === 0 ? (
                  <div className="text-sm text-slate-500">
                    No controls for this object yet. Use the UI Agent to expose props and set `objectId: &quot;{selectedObjectId}&quot;`.
                  </div>
                ) : (
                  groupControls(selectedControls).map(([groupName, list]) => (
                    <div key={groupName} className="space-y-3">
                      <div className="text-[11px] font-black text-slate-500 uppercase tracking-widest">{groupName}</div>
                      <div className="space-y-4">{list.map(renderControl)}</div>
                    </div>
                  ))
                )
              ) : (
                <div className="text-sm text-slate-500">Select an object to edit its properties.</div>
              )}

              <div className="text-[11px] text-slate-500">
                Changes preview immediately. Use the top <span className="font-bold">Save</span> button to persist and create a snapshot.
                Export auto-saves props.
              </div>
            </div>
          )}
        </div>

        <div className="space-y-2 rounded-2xl bg-slate-950/30 border border-slate-800 p-3">
          <button
            onClick={() => setUiAgentOpen((v) => !v)}
            className="w-full flex items-center justify-between text-left"
            type="button"
          >
            <div className="text-xs uppercase font-black text-slate-500 tracking-widest">UI Agent</div>
            <div className="text-[11px] font-bold text-slate-400">{uiAgentOpen ? 'Hide' : 'Show'}</div>
          </button>

          {uiAgentOpen && (
            <div className="space-y-3 pt-2">
              <div className="text-sm text-slate-400">
                Prompt the agent to expose props (and update `Composition.tsx`, `epris-controls.json`, `epris-props.json` atomically).
              </div>
              <textarea
                value={uiAgentPrompt}
                onChange={(e) => setUiAgentPrompt(e.target.value)}
                className="w-full min-h-[96px] bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 transition-colors resize-y"
                placeholder="e.g. I want to change the background color"
                disabled={uiAgentBusy}
              />
              <div className="flex items-center justify-between gap-2">
                <div className="text-[11px] text-slate-500">
                  Model: <span className="font-mono">{model}</span>
                </div>
                <button
                  onClick={sendUiAgentPrompt}
                  disabled={uiAgentBusy || !uiAgentPrompt.trim()}
                  className="px-4 py-2 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-black text-xs uppercase tracking-widest transition-colors disabled:opacity-50"
                >
                  {uiAgentBusy ? 'Running…' : 'Run'}
                </button>
              </div>

              {(uiAgentError || uiAgentLast) && (
                <div className="p-3 bg-slate-950/40 border border-slate-800 rounded-xl text-slate-300 text-sm whitespace-pre-wrap">
                  {uiAgentError ? `Error: ${uiAgentError}` : uiAgentLast}
                </div>
              )}
            </div>
          )}
        </div>

        {error && (
          <div className="p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-red-400 text-sm">
            {error}
          </div>
        )}
      </div>
    </aside>
  );
}
