type TextControlProps = {
  label: string;
  value: string;
  onChange: (next: string) => void;
  placeholder?: string;
  disabled?: boolean;
};

export function TextControl({ label, value, onChange, placeholder, disabled }: TextControlProps) {
  return (
    <div className="space-y-2">
      <div className="text-sm font-bold text-slate-200">{label}</div>
      <input
        type="text"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
        disabled={disabled}
      />
    </div>
  );
}

