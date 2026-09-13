import type { MonData } from "battler-types";
import TeamMonIcons from "./TeamMonIcons";
import styles from "./TeamSelect.module.scss";

interface TeamSelectProps {
  id?: string;
  className?: string;
  value: string;
  onChange: (value: string) => void;
  teamNames: string[];
  teams: Record<string, MonData[] | Partial<MonData>[]>;
  disabled?: boolean;
  required?: boolean;
  showPreview?: boolean;
  previewSize?: "sm" | "md";
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
  showPreview = true,
  previewSize = "md",
}: TeamSelectProps) {
  const isValidValue = Boolean(value && teamNames.includes(value));
  const selectedMembers = isValidValue ? teams[value] : undefined;

  return (
    <div className={`${styles.teamSelectContainer} ${className || ""}`.trim()}>
      <select
        id={id}
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
            {name}
          </option>
        ))}
      </select>
      {showPreview && selectedMembers && selectedMembers.length > 0 && (
        <div className={styles.previewContainer}>
          <TeamMonIcons members={selectedMembers} size={previewSize} />
        </div>
      )}
    </div>
  );
}

