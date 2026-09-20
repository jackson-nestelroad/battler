import type { TimerSettingsState } from "./proposalTypes";
import { TIMER_PRESETS } from "./proposalTypes";
import styles from "./ProposalForm.module.scss";

export type { TimerSettingsState };
export { TIMER_PRESETS };

type TimerFieldKey = "actionTimer" | "teamPreviewTimer" | "playerTimer" | "battleTimer";

interface TimerFieldConfig {
  id: string;
  field: TimerFieldKey;
  label: string;
  min: string;
  placeholder: string;
}

const TIMER_INPUT_FIELDS: readonly TimerFieldConfig[] = [
  {
    id: "customActionTimer",
    field: "actionTimer",
    label: "Action (secs)",
    min: "5",
    placeholder: "e.g., 45",
  },
  {
    id: "customTeamPreviewTimer",
    field: "teamPreviewTimer",
    label: "Team Preview (secs)",
    min: "5",
    placeholder: "e.g., 60",
  },
  {
    id: "customPlayerTimer",
    field: "playerTimer",
    label: "Player (secs)",
    min: "10",
    placeholder: "e.g., 420",
  },
  {
    id: "customBattleTimer",
    field: "battleTimer",
    label: "Battle (secs)",
    min: "30",
    placeholder: "e.g., 1200",
  },
] as const;

interface TimerSettingsSectionProps {
  timerSettings: TimerSettingsState;
  onChange: (fields: Partial<TimerSettingsState>) => void;
}

export default function TimerSettingsSection({
  timerSettings,
  onChange,
}: TimerSettingsSectionProps) {
  const isCustom = timerSettings.preset === "custom";

  const getTimerValue = (field: TimerFieldKey) => {
    if (timerSettings.preset === "custom") return timerSettings[field];
    if (timerSettings.preset === "none") return "";
    return TIMER_PRESETS[timerSettings.preset][field];
  };

  return (
    <div className={styles.advancedSection}>
      <h4 className="mb-s">Timers</h4>
      <div className="flex-row flex-mobile-col gap-m align-end">
        <div className="form-group flex-1">
          <label htmlFor="timerPreset">Timer preset</label>
          <select
            id="timerPreset"
            value={timerSettings.preset}
            onChange={(e) => onChange({ preset: e.target.value as TimerSettingsState["preset"] })}
          >
            <option value="none">None</option>
            <option value="blitz">Blitz</option>
            <option value="standard">Standard</option>
            <option value="custom">Custom</option>
          </select>
        </div>

        <div className="form-group flex-1">
          <label htmlFor="proposalTimeout">Proposal timeout (secs)</label>
          <input
            id="proposalTimeout"
            type="number"
            min="10"
            value={timerSettings.proposalTimeout}
            onChange={(e) => onChange({ proposalTimeout: Number(e.target.value) })}
          />
        </div>
      </div>

      <div className={`${styles.settingsInputsGrid} mt-m`}>
        {TIMER_INPUT_FIELDS.map(({ id, field, label, min, placeholder }) => (
          <div key={id} className="form-group">
            <label htmlFor={id}>{label}</label>
            <input
              id={id}
              type="number"
              min={min}
              placeholder={isCustom ? placeholder : "None"}
              value={getTimerValue(field)}
              onChange={isCustom ? (e) => onChange({ [field]: e.target.value }) : undefined}
              disabled={!isCustom}
            />
          </div>
        ))}
      </div>
    </div>
  );
}
