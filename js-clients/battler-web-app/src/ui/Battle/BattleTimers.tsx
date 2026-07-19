import { useMemo, useState } from "react";
import type { BattleState } from "battler-state";
import type { Battle } from "battler-service-client";
import { getPlayerName, formatSeconds } from "../../utils/battle";
import CountdownTimer from "../Common/CountdownTimer";
import type { ActiveTimerState } from "../../store/battlesSlice";
import styles from "./BattleScreen.module.scss";

interface BattleTimersProps {
  activeTimers?: Record<string, ActiveTimerState>;
  playerId?: string;
  battleState?: BattleState | null;
  serviceBattle?: Battle | null;
  isReplay?: boolean;
}

export default function BattleTimers({
  activeTimers,
  playerId,
  battleState,
  serviceBattle,
  isReplay = false,
}: BattleTimersProps) {
  const [showOtherTimers, setShowOtherTimers] = useState(false);

  const activeTimersList = useMemo(() => {
    if (!activeTimers) return [];

    return Object.entries(activeTimers)
      .map(([key, timer]) => {
        const isMe = timer.playerId === playerId;
        let label = "";

        if (timer.type === "battle") {
          label = "Battle";
        } else {
          const name = timer.playerId
            ? getPlayerName(timer.playerId, battleState, serviceBattle)
            : "";

          if (timer.type === "player") {
            label = isMe ? "Your Time" : name;
          } else if (timer.type === "action") {
            label = isMe ? "Turn Timer" : `${name} Turn`;
          } else if (timer.type === "teampreview") {
            label = isMe ? "Team Preview" : `${name} Preview`;
          }
        }

        return {
          key,
          label,
          deadlineSecs: timer.deadlineSecs,
          remainingSecs: timer.remainingSecs,
          type: timer.type,
          playerId: timer.playerId,
          isMe,
          isInactive: isReplay || !!timer.isInactive,
          isDone: !!timer.isDone,
        };
      })
      .sort((a, b) => {
        // 1. Battle timer first
        if (a.type === "battle") return -1;
        if (b.type === "battle") return 1;

        // 2. Local player timers second
        if (a.isMe && !b.isMe) return -1;
        if (!a.isMe && b.isMe) return 1;

        // 3. Group by player ID (so a player's bank and action timers are adjacent)
        if (a.playerId && b.playerId && a.playerId !== b.playerId) {
          return a.playerId.localeCompare(b.playerId);
        }

        // 4. Bank timer before action/preview timer
        const aIsBank = a.type === "player";
        const bIsBank = b.type === "player";
        if (aIsBank && !bIsBank) return -1;
        if (!aIsBank && bIsBank) return 1;

        return 0;
      });
  }, [activeTimers, playerId, battleState, serviceBattle, isReplay]);

  if (activeTimersList.length === 0) return null;

  const primaryTimers = activeTimersList.filter((t) => t.type === "battle" || t.isMe);
  const otherTimers = activeTimersList.filter((t) => t.type !== "battle" && !t.isMe);

  const renderTimerBadge = (timer: (typeof activeTimersList)[0]) => {
    if (timer.isDone) {
      return (
        <div key={timer.key} className="badge badge-danger badge-timer">
          {timer.label}: 0:00
        </div>
      );
    }
    if (timer.isInactive) {
      return (
        <div key={timer.key} className="badge badge-secondary badge-timer">
          {timer.label}: {formatSeconds(timer.remainingSecs)}
        </div>
      );
    }
    return (
      <CountdownTimer
        key={timer.key}
        deadlineSecs={timer.deadlineSecs}
        prefix={`${timer.label}: `}
        badgeMode={true}
        badgeClassOverride={timer.type !== "battle" && !timer.isMe ? "badge-secondary" : undefined}
      />
    );
  };

  return (
    <div className="flex-col gap-xs">
      <div className="flex-row align-center gap-s flex-wrap">
        {primaryTimers.map(renderTimerBadge)}
        {otherTimers.length > 0 && (
          <button
            className={`btn btn-sm btn-secondary ${styles.toggleOthersBtn}`}
            onClick={() => setShowOtherTimers(!showOtherTimers)}
          >
            {showOtherTimers ? "Hide" : "Show"} others ({otherTimers.length}){" "}
            {showOtherTimers ? "▲" : "▼"}
          </button>
        )}
      </div>

      {showOtherTimers && otherTimers.length > 0 && (
        <div className="flex-row align-center gap-s flex-wrap">
          {otherTimers.map(renderTimerBadge)}
        </div>
      )}
    </div>
  );
}
