import { useMemo } from 'react';

type NumberControlProps = {
  label: string;
  value: number;
  onChange: (next: number) => void;
  ui?: 'slider' | 'input';
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
};

export function NumberControl({ label, value, onChange, ui = 'slider', min, max, step, disabled }: NumberControlProps) {
  const safeMin = useMemo(() => (typeof min === 'number' ? min : 0), [min]);
  const safeMax = useMemo(() => (typeof max === 'number' ? max : safeMin + 100), [max, safeMin]);
  const safeStep = useMemo(() => (typeof step === 'number' && step > 0 ? step : 1), [step]);

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <div className="text-sm font-bold text-slate-200 truncate">{label}</div>
        <input
          type="number"
          value={Number.isFinite(value) ? value : 0}
          min={min}
          max={max}
          step={safeStep}
          onChange={(e) => onChange(Number(e.target.value))}
          className="w-28 bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
          disabled={disabled}
        />
      </div>

      {ui === 'slider' && (
        <input
          type="range"
          value={Number.isFinite(value) ? value : 0}
          min={safeMin}
          max={safeMax}
          step={safeStep}
          onChange={(e) => onChange(Number(e.target.value))}
          className="w-full accent-indigo-500"
          disabled={disabled}
        />
      )}
    </div>
  );
}

