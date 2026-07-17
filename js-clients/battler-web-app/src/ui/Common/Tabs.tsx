interface TabOption<T extends string> {
  value: T;
  label: string;
}

interface TabsProps<T extends string> {
  options: TabOption<T>[];
  active: T;
  onChange: (value: T) => void;
}

export default function Tabs<T extends string>({ options, active, onChange }: TabsProps<T>) {
  return (
    <div className="tabs-row">
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          className={`tab-btn ${active === option.value ? "active" : ""}`}
          onClick={() => onChange(option.value)}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}
