import styles from "./ProposalForm.module.scss";

export interface TimerSettingsState {
  preset: "none" | "blitz" | "standard" | "custom";
  battleTimer: string;
  playerTimer: string;
  actionTimer: string;
  teamPreviewTimer: string;
  proposalTimeout: number;
}

export const TIMER_PRESETS = {
  blitz: {
    actionTimer: "10",
    teamPreviewTimer: "15",
    playerTimer: "",
    battleTimer: "",
  },
  standard: {
    actionTimer: "45",
    teamPreviewTimer: "60",
    playerTimer: "420",
    battleTimer: "1200",
  },
} as const;

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
    if (timerSettings.preset === "none") return "";
    return TIMER_PRESETS[timerSettings.preset].actionTimer;
  };

  const getPlayerTimerValue = () => {
    if (timerSettings.preset === "custom") return timerSettings.playerTimer;
    if (timerSettings.preset === "none") return "";
    return TIMER_PRESETS[timerSettings.preset].playerTimer;
  };

  const getBattleTimerValue = () => {
    if (timerSettings.preset === "custom") return timerSettings.battleTimer;
    if (timerSettings.preset === "none") return "";
    return TIMER_PRESETS[timerSettings.preset].battleTimer;
  };

  const getTeamPreviewTimerValue = () => {
    if (timerSettings.preset === "custom") return timerSettings.teamPreviewTimer;
    if (timerSettings.preset === "none") return "";
    return TIMER_PRESETS[timerSettings.preset].teamPreviewTimer;
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

      <div className={`${styles.timerInputsGrid} mt-m`}>
        <div className="form-group">
          <label htmlFor="customActionTimer">Action (secs)</label>
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

        <div className="form-group">
          <label htmlFor="customTeamPreviewTimer">Team preview (secs)</label>
          <input
            id="customTeamPreviewTimer"
            type="number"
            min="5"
            placeholder={timerSettings.preset === "custom" ? "e.g., 60" : "None"}
            value={getTeamPreviewTimerValue()}
            onChange={
              timerSettings.preset === "custom"
                ? (e) => onChange({ teamPreviewTimer: e.target.value })
                : undefined
            }
            disabled={timerSettings.preset !== "custom"}
          />
        </div>

        <div className="form-group">
          <label htmlFor="customPlayerTimer">Player (secs)</label>
          <input
            id="customPlayerTimer"
            type="number"
            min="10"
            placeholder={timerSettings.preset === "custom" ? "e.g., 420" : "None"}
            value={getPlayerTimerValue()}
            onChange={
              timerSettings.preset === "custom"
                ? (e) => onChange({ playerTimer: e.target.value })
                : undefined
            }
            disabled={timerSettings.preset !== "custom"}
          />
        </div>

        <div className="form-group">
          <label htmlFor="customBattleTimer">Battle (secs)</label>
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
