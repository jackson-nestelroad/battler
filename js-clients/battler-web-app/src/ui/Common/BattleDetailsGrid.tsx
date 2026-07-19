import type { BattleType } from "battler-types";
import type { Timers } from "battler-multiplayer-service-client";
import RulesList from "./RulesList";
import { formatSeconds } from "../../utils/battle";

interface BattleDetailsGridProps {
  battleType: BattleType;
  rules?: string[];
  timers?: Timers | null;
}

export default function BattleDetailsGrid({ battleType, rules, timers }: BattleDetailsGridProps) {
  const activeTimers = timers
    ? ([
        { key: "battle", label: "Battle", value: timers.battle },
        { key: "player", label: "Player", value: timers.player },
        { key: "team_preview", label: "Preview", value: timers.team_preview },
        { key: "action", label: "Action", value: timers.action },
      ] as const).filter((t) => t.value)
    : [];

  return (
    <div className="details-grid">
      <span className="details-label">Format</span>
      <div>
        <span className="badge badge-secondary">{battleType}</span>
      </div>

      {rules && rules.length > 0 && (
        <>
          <span className="details-label">Rules</span>
          <RulesList rules={rules} />
        </>
      )}

      {activeTimers.length > 0 && (
        <>
          <span className="details-label">Timers</span>
          <div className="flex-row gap-xs flex-wrap">
            {activeTimers.map(({ key, label, value }) => (
              <span key={key} className="badge badge-info badge-timer">
                {label}: {formatSeconds(Number(value!.secs))}
              </span>
            ))}
          </div>
        </>
      )}
    </div>
  );
}
