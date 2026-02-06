type ToggleControlProps = {
  label: string;
  value: boolean;
  onChange: (next: boolean) => void;
  disabled?: boolean;
};

export function ToggleControl({ label, value, onChange, disabled }: ToggleControlProps) {
  return (
    <div className="flex items-center justify-between gap-3">
      <div className="text-sm font-bold text-slate-200">{label}</div>
      <button
        type="button"
        onClick={() => onChange(!value)}
        disabled={disabled}
        className={`w-12 h-7 rounded-full border transition-colors relative ${
          value ? 'bg-indigo-600/40 border-indigo-500/40' : 'bg-slate-950 border-slate-700'
        } ${disabled ? 'opacity-50' : 'hover:border-indigo-500/50'}`}
        title={value ? 'On' : 'Off'}
      >
        <span
          className={`absolute top-0.5 w-6 h-6 rounded-full bg-white/90 transition-transform ${
            value ? 'translate-x-5' : 'translate-x-0.5'
          }`}
        />
      </button>
    </div>
  );
}

