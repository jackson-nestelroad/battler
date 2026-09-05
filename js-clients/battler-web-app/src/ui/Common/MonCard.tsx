import type { BattleState } from "battler-state";
import type { MonBattleData } from "battler-types";
import { formatStatusBadge } from "../../utils/monHelpers";
import HpBar from "./HpBar";
import styles from "./MonCard.module.scss";
import MonTooltipTrigger from "./Tooltip/MonTooltipTrigger";

interface MonCardProps {
  name: string;
  level: number;
  hp: number;
  maxHp: number;
  status: string | null;
  active: boolean;
  isClickable: boolean;
  onClick?: () => void;
  selectionOrder?: number;
  isActing?: boolean;
  actingBadgeText?: string;
  monBattleData?: MonBattleData;
  battleState?: BattleState | null;
}

export default function MonCard({
  name,
  level,
  hp,
  maxHp,
  status,
  active,
  isClickable,
  onClick,
  selectionOrder,
  isActing,
  actingBadgeText = "ACTING",
  monBattleData,
  battleState,
}: MonCardProps) {
  const cardContent = (
    <div
      onClick={isClickable ? onClick : undefined}
      className={`${styles.teamSummaryCard} ${active ? styles.summaryActive : ""} ${
        isActing ? styles.summaryActing : ""
      } ${hp === 0 ? styles.summaryFainted : ""} ${
        isClickable ? styles.clickableSummaryCard : ""
      } ${selectionOrder !== undefined ? styles.selectedCard : ""}`}
    >
      {selectionOrder !== undefined && (
        <div className={styles.selectionBadge}>{selectionOrder}</div>
      )}
      {isActing && <div className={styles.actingBadge}>{actingBadgeText}</div>}
      <div className={styles.summaryCardHeader}>
        <span className={styles.summaryMonName}>{name}</span>
        <span className={styles.summaryMonLevel}>L{level}</span>
      </div>

      <div className={styles.summaryCardMetaRow}>
        <div className={styles.summaryCardMeta}>
          {(() => {
            const statusBadge = formatStatusBadge(status);
            return hp === 0 || statusBadge?.code === "fnt" ? (
              <span className="status-badge fnt">FNT</span>
            ) : statusBadge ? (
              <span className={`status-badge ${statusBadge.code}`}>{statusBadge.label}</span>
            ) : (
              <span className="status-badge ok">OK</span>
            );
          })()}
        </div>
        <span className={styles.summaryHpText}>
          {hp}/{maxHp}
        </span>
      </div>

      <HpBar hp={hp} maxHp={maxHp} />
    </div>
  );

  if (monBattleData) {
    return (
      <MonTooltipTrigger
        mon={monBattleData}
        battleState={battleState}
        className="flex-col w-full h-full"
      >
        {cardContent}
      </MonTooltipTrigger>
    );
  }

  return cardContent;
}
