import type { BattleState } from "battler-state";
import { stateSelectors } from "battler-state";
import type { MonMoveSlotData, PlayerBattleData, Request } from "battler-types";
import { 
  canSlotSelect,
  canSlotSwitch,
  getMonDisplayName, 
  getMonTeamPosition, 
  getRequestSlotCount,
  getSelectReason,
  getTeamPreviewTargetSize,
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
  onSelect?: (playerTeamPosition: number, totalSlots: number) => void;
  selectedTeamIndices?: number[];
  onSelectMon?: (idx: number) => void;
  activeMonTeamPosition?: number | null;
  actingBadgeText?: string;
  battleState?: BattleState | null;
  rules?: string[] | null;
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
  onSelect,
  selectedTeamIndices = [],
  onSelectMon,
  activeMonTeamPosition,
  actingBadgeText,
  battleState,
  rules,
}: TeamSummaryProps) {
  if (!playerData || !playerData.mons) return null;

  const targetSize = request?.type === "team" ? getTeamPreviewTargetSize(request, playerData) : 0;
  const totalSlots = getRequestSlotCount(request);
  const isPlayerLeft = Boolean(
    battleState &&
      stateSelectors.player(battleState, playerData.id)?.left_battle,
  );

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
            activeMonTeamPosition != null && monPos === activeMonTeamPosition;

          // Check if card is clickable for switching, team preview, or selection
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
            } else if (canSlotSelect(request, currentSlotIndex) && onSelect) {
              const reason = getSelectReason(request, currentSlotIndex);
              if (reason === "Revive") {
                isClickable = !mon.active && (mon.hp ?? 0) <= 0;
                if (isClickable) {
                  handleClick = () => onSelect(monPos, totalSlots);
                }
              }
            } else if (canSlotSwitch(request, currentSlotIndex, selectedMove)) {
              isClickable = !mon.active && mon.hp > 0;
              if (isClickable) {
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
              active={!isPlayerLeft && Boolean(mon.active)}
              isClickable={isClickable}
              onClick={handleClick}
              selectionOrder={selectionOrder}
              isActing={isActing}
              actingBadgeText={actingBadgeText}
              monBattleData={mon}
              battleState={battleState}
              rules={rules}
            />
          );
        })}
      </div>
    </div>
  );
}
