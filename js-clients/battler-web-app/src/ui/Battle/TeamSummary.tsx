import type { BattleState } from "battler-state";
import type { MonMoveSlotData, PlayerBattleData, Request } from "battler-types";
import { 
  getMonDisplayName, 
  getMonTeamPosition, 
  getRequestSlotCount,
  getTeamPreviewTargetSize,
  canSlotSwitch,
} from "../../utils/monHelpers";
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
  selectedTeamIndices?: number[];
  onSelectMon?: (idx: number) => void;
  activeMonTeamPosition?: number | null;
  actingBadgeText?: string;
  battleState?: BattleState | null;
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
  selectedTeamIndices = [],
  onSelectMon,
  activeMonTeamPosition,
  actingBadgeText,
  battleState,
}: TeamSummaryProps) {
  if (!playerData || !playerData.mons) return null;

  const targetSize = request?.type === "team" ? getTeamPreviewTargetSize(request, playerData) : 0;

  return (
    <div className={styles.teamSummarySection}>
      <div className={styles.columnHeaderRow}>
        <h4 className={styles.summaryTitle}>Team</h4>
      </div>
      <div className={styles.teamSummaryGrid}>
        {playerData.mons.map((mon, idx) => {
          const name = getMonDisplayName(mon);
          const monPos = getMonTeamPosition(mon, idx);
          const isActing =
            activeMonTeamPosition !== undefined &&
            activeMonTeamPosition !== null &&
            monPos === activeMonTeamPosition;

          // Check if card is clickable for switching or team preview
          let isClickable = false;
          let handleClick: (() => void) | undefined = undefined;

          if (request && !isMeReady && !playbackPending && !isLoading) {
            if (request.type === "team") {
              const isSelected = selectedTeamIndices.includes(idx);
              const hasReachedMax = selectedTeamIndices.length >= targetSize;

              isClickable = isSelected || !hasReachedMax;
              if (isClickable && onSelectMon) {
                handleClick = () => onSelectMon(idx);
              }
            } else if (canSlotSwitch(request, currentSlotIndex, selectedMove)) {
              isClickable = !mon.active && mon.hp > 0;
              if (isClickable) {
                const totalSlots = getRequestSlotCount(request);
                handleClick = () => onSwitch(monPos, totalSlots);
              }
            }
          }

          const selectedIdx = selectedTeamIndices.indexOf(idx);
          const selectionOrder = selectedIdx !== -1 ? selectedIdx + 1 : undefined;

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
              selectionOrder={selectionOrder}
              isActing={isActing}
              actingBadgeText={actingBadgeText}
              monBattleData={mon}
              battleState={battleState}
            />
          );
        })}
      </div>
    </div>
  );
}
