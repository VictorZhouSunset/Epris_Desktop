type Option = string | { value: string; label?: string };

type SelectControlProps = {
  label: string;
  value: string;
  options: Option[];
  onChange: (next: string) => void;
  disabled?: boolean;
};

function optionValue(opt: Option) {
  return typeof opt === 'string' ? opt : opt.value;
}

function optionLabel(opt: Option) {
  return typeof opt === 'string' ? opt : opt.label ?? opt.value;
}

export function SelectControl({ label, value, options, onChange, disabled }: SelectControlProps) {
  return (
    <div className="space-y-2">
      <div className="text-sm font-bold text-slate-200">{label}</div>
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        disabled={disabled}
        className="w-full bg-slate-950 border border-slate-700 rounded-xl px-3 py-2 text-sm text-white outline-none focus:border-indigo-500 transition-colors"
      >
        {options.map((opt) => (
          <option key={optionValue(opt)} value={optionValue(opt)}>
            {optionLabel(opt)}
          </option>
        ))}
      </select>
    </div>
  );
}

