import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { IpcService } from '../lib/ipc';
import {
  EprisControlsSpec,
  EprisControlsSpecSchema,
  EprisControlSpec,
  EprisPropsFile,
  EprisPropsFileSchema,
  formatZodError,
} from '../types/ui-controls';

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function stableStringify(value: unknown): string {
  try {
    return JSON.stringify(value);
  } catch {
    return '';
  }
}

function normalizeControl(control: EprisControlSpec): EprisControlSpec {
  if (control.type !== 'select') return control;
  return {
    ...control,
    options: control.options.map((opt) => (typeof opt === 'string' ? { value: opt, label: opt } : opt)),
  };
}

export type UiPropsLoadState = {
  controlsSpec: EprisControlsSpec | null;
  propsFile: EprisPropsFile | null;
  draftValues: Record<string, unknown>;
  dirty: boolean;
  error: string | null;
  loading: boolean;
};

export type UiPropsActions = {
  reload: () => Promise<void>;
  setDraftValue: (id: string, value: unknown) => void;
  revertDraft: () => void;
  saveDraft: () => Promise<void>;
  saveIfDirty: () => Promise<void>;
};

type UseUiPropsOptions = {
  sendToPreview: (values: Record<string, unknown>) => void;
};

export function useUiProps(workspacePath: string | undefined, options: UseUiPropsOptions) {
  const [state, setState] = useState<UiPropsLoadState>({
    controlsSpec: null,
    propsFile: null,
    draftValues: {},
    dirty: false,
    error: null,
    loading: false,
  });

  const savedValuesRef = useRef<Record<string, unknown>>({});
  const sendToPreview = options.sendToPreview;

  const reload = useCallback(async () => {
    if (!workspacePath) return;
    setState((s) => ({ ...s, loading: true, error: null }));
    try {
      const [controlsRaw, propsRaw] = await Promise.all([
        IpcService.call<unknown>('GET_EPRIS_CONTROLS', { workspacePath }),
        IpcService.call<unknown>('GET_EPRIS_PROPS', { workspacePath }),
      ]);

      const parsedControls = EprisControlsSpecSchema.safeParse(controlsRaw);
      if (!parsedControls.success) {
        throw new Error(`Invalid src/epris-controls.json:\n${formatZodError(parsedControls.error)}`);
      }

      const parsedProps = EprisPropsFileSchema.safeParse(propsRaw);
      if (!parsedProps.success) {
        throw new Error(`Invalid src/epris-props.json:\n${formatZodError(parsedProps.error)}`);
      }

      const normalizedControls: EprisControlsSpec = {
        ...parsedControls.data,
        controls: parsedControls.data.controls.map(normalizeControl),
      };

      savedValuesRef.current = parsedProps.data.values;
      setState({
        controlsSpec: normalizedControls,
        propsFile: parsedProps.data,
        draftValues: parsedProps.data.values,
        dirty: false,
        error: null,
        loading: false,
      });

      sendToPreview(parsedProps.data.values);
    } catch (e) {
      setState((s) => ({
        ...s,
        controlsSpec: null,
        propsFile: null,
        draftValues: {},
        dirty: false,
        error: e instanceof Error ? e.message : String(e),
        loading: false,
      }));
    }
  }, [workspacePath, sendToPreview]);

  useEffect(() => {
    void reload();
  }, [reload]);

  // When draft changes, postMessage immediately for live preview.
  const lastSentRef = useRef<string>('');
  useEffect(() => {
    const key = stableStringify(state.draftValues);
    if (key && key === lastSentRef.current) return;
    lastSentRef.current = key;
    sendToPreview(state.draftValues);
  }, [state.draftValues, sendToPreview]);

  const setDraftValue = useCallback((id: string, value: unknown) => {
    setState((s) => {
      const next = { ...s.draftValues, [id]: value };
      const dirty = stableStringify(next) !== stableStringify(savedValuesRef.current);
      return { ...s, draftValues: next, dirty };
    });
  }, []);

  const revertDraft = useCallback(() => {
    const saved = savedValuesRef.current;
    setState((s) => ({ ...s, draftValues: saved, dirty: false }));
    sendToPreview(saved);
  }, [sendToPreview]);

  const saveDraft = useCallback(async () => {
    if (!workspacePath) return;
    const values = state.draftValues;
    if (!isPlainObject(values)) throw new Error('Draft values must be an object');

    const updated = await IpcService.call<EprisPropsFile>('SET_EPRIS_PROPS', { workspacePath, values });
    const parsed = EprisPropsFileSchema.safeParse(updated);
    if (!parsed.success) {
      throw new Error(`Backend returned invalid props:\n${formatZodError(parsed.error)}`);
    }

    savedValuesRef.current = parsed.data.values;
    setState((s) => ({
      ...s,
      propsFile: parsed.data,
      draftValues: parsed.data.values,
      dirty: false,
    }));
  }, [workspacePath, state.draftValues]);

  const saveIfDirty = useCallback(async () => {
    if (!state.dirty) return;
    await saveDraft();
  }, [state.dirty, saveDraft]);

  const groupedControls = useMemo(() => {
    const controls = state.controlsSpec?.controls ?? [];
    const groups = new Map<string, EprisControlSpec[]>();
    for (const c of controls) {
      const g = c.group ?? 'General';
      const arr = groups.get(g) ?? [];
      arr.push(c);
      groups.set(g, arr);
    }
    return Array.from(groups.entries()).sort(([a], [b]) => a.localeCompare(b));
  }, [state.controlsSpec]);

  return {
    state,
    groupedControls,
    actions: { reload, setDraftValue, revertDraft, saveDraft, saveIfDirty } satisfies UiPropsActions,
  };
}
