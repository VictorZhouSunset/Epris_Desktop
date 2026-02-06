import { HexColorInput, HexColorPicker } from 'react-colorful';
import { useState } from 'react';

type ColorControlProps = {
  label: string;
  value: string;
  onChange: (next: string) => void;
  disabled?: boolean;
};

export function ColorControl({ label, value, onChange, disabled }: ColorControlProps) {
  const [open, setOpen] = useState(false);

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <button
          type="button"
          className="flex items-center gap-2 text-sm font-bold text-slate-200 truncate"
          onClick={() => setOpen((v) => !v)}
          disabled={disabled}
          title="Toggle color picker"
        >
          <span
            className="w-4 h-4 rounded border border-slate-700"
            style={{ background: value || '#000000' }}
          />
          <span className="truncate">{label}</span>
        </button>

        <div className="flex items-center gap-2">
          <div className="text-xs font-mono text-slate-400">{value || ''}</div>
        </div>
      </div>

      {open && (
        <div className="p-3 rounded-xl bg-slate-950/40 border border-slate-800 space-y-3">
          <HexColorPicker color={value || '#000000'} onChange={onChange} />
          <div className="flex items-center gap-2">
            <div className="text-xs font-black text-slate-500 uppercase tracking-widest">Hex</div>
            <HexColorInput
              color={value || '#000000'}
              onChange={onChange}
              className="flex-1 bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
              disabled={disabled}
              prefixed
            />
          </div>
        </div>
      )}
    </div>
  );
}

