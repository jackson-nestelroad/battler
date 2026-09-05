import type { BattleState } from "battler-state";
import type { MonBattleData } from "battler-types";
import { normalizeStatusCode } from "../../utils/monHelpers";
import HpBar from "./HpBar";
import StatusBadge from "./StatusBadge";
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
  actingBadgeText = "Acting",
  monBattleData,
  battleState,
}: MonCardProps) {
  const isFainted = hp <= 0 || normalizeStatusCode(status) === "fnt";

  const cardContent = (
    <div
      onClick={isClickable ? onClick : undefined}
      className={`${styles.teamSummaryCard} ${active ? styles.summaryActive : ""} ${
        isActing ? styles.summaryActing : ""
      } ${isFainted ? styles.summaryFainted : ""} ${
        isClickable ? styles.clickableSummaryCard : ""
      } ${selectionOrder != null ? styles.selectedCard : ""}`}
    >
      {selectionOrder != null && (
        <div className={styles.selectionBadge}>{selectionOrder}</div>
      )}
      {isActing && <div className={styles.actingBadge}>{actingBadgeText}</div>}
      <div className={styles.summaryCardHeader}>
        <span className={styles.summaryMonName}>{name}</span>
        <span className={styles.summaryMonLevel}>L{level}</span>
      </div>

      <div className={styles.summaryCardMetaRow}>
        <StatusBadge status={status} isFainted={isFainted} />
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
        as="div"
        className="flex-col w-full h-full"
      >
        {cardContent}
      </MonTooltipTrigger>
    );
  }

  return cardContent;
}
