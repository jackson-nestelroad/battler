import type { Request, PlayerBattleData, MonMoveSlotData } from "battler-types";
import MonCard from "../Common/MonCard";
import styles from "./ActionPanel.module.scss";

interface TeamSummaryProps {
  playerData: PlayerBattleData | null;
  request: Request | null;
  currentSlotIndex: number;
  selectedMove: MonMoveSlotData | null;
  isMeReady: boolean;
  playbackPending: boolean;
  isLoading: boolean;
  onSwitch: (playerTeamPosition: number, totalSlots: number) => void;
}

export default function TeamSummary({
  playerData,
  request,
  currentSlotIndex,
  selectedMove,
  isMeReady,
  playbackPending,
  isLoading,
  onSwitch,
}: TeamSummaryProps) {
  if (!playerData || !playerData.mons) return null;

  return (
    <div className={styles.teamSummarySection}>
      <h4 className={styles.summaryTitle}>Team</h4>
      <div className={styles.teamSummaryGrid}>
        {playerData.mons.map((mon, idx) => {
          const name = mon.summary?.name || mon.species;

          // Check if card is clickable for switching
          let isClickable = false;
          let handleClick: (() => void) | undefined = undefined;

          if (request && !isMeReady && !playbackPending && !isLoading) {
            let totalSlots = 0;
            let canSwitch = false;

            if (request.type === "switch") {
              const needsSwitch = request.needs_switch || [];
              totalSlots = needsSwitch.length;
              canSwitch = needsSwitch[currentSlotIndex] !== undefined;
            } else if (request.type === "turn" && selectedMove === null) {
              const activeRequests = request.active || [];
              totalSlots = activeRequests.length;
              const activeReq = activeRequests[currentSlotIndex];
              canSwitch = !!(activeReq && !activeReq.trapped);
            }

            if (canSwitch) {
              isClickable = !mon.active && mon.hp > 0;
              if (isClickable) {
                handleClick = () => {
                  onSwitch(mon.player_team_position, totalSlots);
                };
              }
            }
          }

          return (
            <MonCard
              key={idx}
              name={name}
              level={mon.summary?.level || 50}
              hp={mon.hp}
              maxHp={mon.max_hp}
              status={mon.status}
              active={!!mon.active}
              isClickable={isClickable}
              onClick={handleClick}
            />
          );
        })}
      </div>
    </div>
  );
}
