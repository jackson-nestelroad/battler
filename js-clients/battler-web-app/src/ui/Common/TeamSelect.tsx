
import type { MonData } from "battler-types";

interface TeamSelectProps {
  id?: string;
  className?: string;
  value: string;
  onChange: (value: string) => void;
  teamNames: string[];
  teams: Record<string, MonData[]>;
  disabled?: boolean;
  required?: boolean;
}

export default function TeamSelect({
  id,
  className,
  value,
  onChange,
  teamNames,
  teams,
  disabled = false,
  required = false,
}: TeamSelectProps) {
  const isValidValue = value && teamNames.includes(value);

  return (
    <select
      id={id}
      className={className}
      value={isValidValue ? value : ""}
      onChange={(e) => onChange(e.target.value)}
      disabled={disabled}
      required={required}
    >
      <option value="" disabled>
        Select team
      </option>
      {teamNames.map((name) => (
        <option key={name} value={name}>
          {name} ({teams[name]?.length || 0})
        </option>
      ))}
    </select>
  );
}
