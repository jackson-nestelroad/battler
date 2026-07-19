import styles from "./ProposalForm.module.scss";

export interface TimerSettingsState {
  preset: "none" | "blitz" | "standard" | "custom";
  battleTimer: string;
  playerTimer: string;
  actionTimer: string;
  proposalTimeout: number;
}

interface TimerSettingsSectionProps {
  timerSettings: TimerSettingsState;
  onChange: (fields: Partial<TimerSettingsState>) => void;
}

export default function TimerSettingsSection({
  timerSettings,
  onChange,
}: TimerSettingsSectionProps) {
  const getActionTimerValue = () => {
    if (timerSettings.preset === "custom") return timerSettings.actionTimer;
    if (timerSettings.preset === "blitz") return "15";
    if (timerSettings.preset === "standard") return "45";
    return "";
  };

  const getPlayerTimerValue = () => {
    if (timerSettings.preset === "custom") return timerSettings.playerTimer;
    if (timerSettings.preset === "standard") return "420";
    return "";
  };

  const getBattleTimerValue = () => {
    if (timerSettings.preset === "custom") return timerSettings.battleTimer;
    if (timerSettings.preset === "standard") return "1200";
    return "";
  };

  return (
    <div className={styles.advancedSection}>
      <h4 className="mb-s">Match timers</h4>
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
          <label htmlFor="proposalTimeout">Proposal timeout (seconds)</label>
          <input
            id="proposalTimeout"
            type="number"
            min="10"
            value={timerSettings.proposalTimeout}
            onChange={(e) => onChange({ proposalTimeout: Number(e.target.value) })}
          />
        </div>
      </div>

      <div className="flex-row flex-mobile-col gap-m mt-m">
        <div className="form-group flex-1">
          <label htmlFor="customActionTimer">Action timer (seconds)</label>
          <input
            id="customActionTimer"
            type="number"
            min="5"
            placeholder={timerSettings.preset === "custom" ? "e.g., 45" : "None"}
            value={getActionTimerValue()}
            onChange={
              timerSettings.preset === "custom"
                ? (e) => onChange({ actionTimer: e.target.value })
                : undefined
            }
            disabled={timerSettings.preset !== "custom"}
          />
        </div>

        <div className="form-group flex-1">
          <label htmlFor="customPlayerTimer">Player timer (seconds)</label>
          <input
            id="customPlayerTimer"
            type="number"
            min="10"
            placeholder={timerSettings.preset === "custom" ? "e.g., 300" : "None"}
            value={getPlayerTimerValue()}
            onChange={
              timerSettings.preset === "custom"
                ? (e) => onChange({ playerTimer: e.target.value })
                : undefined
            }
            disabled={timerSettings.preset !== "custom"}
          />
        </div>

        <div className="form-group flex-1">
          <label htmlFor="customBattleTimer">Overall match timer (seconds)</label>
          <input
            id="customBattleTimer"
            type="number"
            min="30"
            placeholder={timerSettings.preset === "custom" ? "e.g., 1200" : "None"}
            value={getBattleTimerValue()}
            onChange={
              timerSettings.preset === "custom"
                ? (e) => onChange({ battleTimer: e.target.value })
                : undefined
            }
            disabled={timerSettings.preset !== "custom"}
          />
        </div>
      </div>
    </div>
  );
}
