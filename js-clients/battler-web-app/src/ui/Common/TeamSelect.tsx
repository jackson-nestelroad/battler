import { useMemo } from "react";
import type { MonData } from "battler-types";
import CustomSelect, { type CustomSelectOption } from "./Select/CustomSelect";
import TeamMonIcons from "./TeamMonIcons";

export interface TeamSelectProps {
  id?: string;
  className?: string;
  value: string;
  onChange: (value: string) => void;
  teamNames: string[];
  teams: Record<string, MonData[] | Partial<MonData>[]>;
  disabled?: boolean;
  required?: boolean;
  placeholder?: string;
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
  placeholder = "Select team",
}: TeamSelectProps) {
  const options: CustomSelectOption<string>[] = useMemo(
    () =>
      teamNames.map((name) => {
        const members = teams[name];
        return {
          value: name,
          label: name,
          endContent:
            members && members.length > 0 ? (
              <TeamMonIcons members={members} size="md" />
            ) : undefined,
        };
      }),
    [teamNames, teams],
  );

  return (
    <CustomSelect
      id={id}
      className={className}
      value={value}
      onChange={onChange}
      options={options}
      placeholder={placeholder}
      disabled={disabled}
      required={required}
      ariaLabel={value ? `Selected team: ${value}` : placeholder}
    />
  );
}
