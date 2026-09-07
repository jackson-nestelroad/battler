import type { BattleState, MonBattleAppearanceReference, UiMon } from "battler-state";
import type { MonBattleData } from "battler-types";
import { normalizeStatusCode } from "../../utils/monHelpers";
import HpBar from "./HpBar";
import StatusBadge from "./StatusBadge";
import styles from "./MonCard.module.scss";
import MonTooltipTrigger from "./Tooltip/MonTooltipTrigger";

export interface MonCardProps {
  name?: string;
  level?: number | null;
  hp?: number;
  maxHp?: number;
  hpText?: string;
  status?: string | null;
  active?: boolean;
  isClickable?: boolean;
  onClick?: () => void;
  selectionOrder?: number;
  isActing?: boolean;
  actingBadgeText?: string;
  monBattleData?: MonBattleData;
  monRef?: UiMon;
  appearanceRef?: MonBattleAppearanceReference;
  battleState?: BattleState | null;
  rules?: string[] | null;
  variant?: "card" | "row";
  isUnrevealed?: boolean;
  isUnbrought?: boolean;
  preferredPlacement?: "top" | "bottom" | "left" | "right";
}

export default function MonCard({
  name = "Mon",
  level,
  hp = 100,
  maxHp = 100,
  hpText,
  status = null,
  active = false,
  isClickable = false,
  onClick,
  selectionOrder,
  isActing,
  actingBadgeText = "Acting",
  monBattleData,
  monRef,
  appearanceRef,
  battleState,
  rules,
  variant = "card",
  isUnrevealed = false,
  isUnbrought = false,
  preferredPlacement,
}: MonCardProps) {
  if (isUnrevealed) {
    return (
      <div
        className={`${variant === "row" ? styles.teamSummaryRow : styles.teamSummaryCard} ${
          styles.unrevealed
        }`}
      >
        <div className={variant === "row" ? styles.rowIdentity : styles.summaryCardHeader}>
          <span className={styles.unrevealedPip} aria-hidden="true">
            ○
          </span>
          <span className={styles.summaryMonName}>Unrevealed</span>
        </div>
      </div>
    );
  }

  const isFainted = hp <= 0 || normalizeStatusCode(status) === "fnt";

  const stateClasses = [
    active && styles.summaryActive,
    isActing && styles.summaryActing,
    isFainted && styles.summaryFainted,
    isUnbrought && styles.summaryUnbrought,
    isClickable && styles.clickableSummaryCard,
  ]
    .filter(Boolean)
    .join(" ");

  const rowContent = (
    <div
      onClick={isClickable ? onClick : undefined}
      className={`${styles.teamSummaryRow} ${stateClasses}`.trim()}
    >
      <div className={styles.rowIdentity}>
        <span
          className={`${styles.rowPip} ${active ? styles.activePip : ""} ${
            isFainted ? styles.faintedPip : ""
          } ${isUnbrought ? styles.unbroughtPip : ""}`}
          aria-hidden="true"
        >
          ●
        </span>
        <span className={styles.summaryMonName} title={name}>{name}</span>
        {isActing && <span className={styles.rowActingBadge}>{actingBadgeText}</span>}
      </div>

      <div className={styles.rowMeta}>
        <StatusBadge status={status} isFainted={isFainted} />
        <div className={styles.rowHpGroup}>
          <span className={styles.summaryHpText}>{hpText ?? `${hp}/${maxHp}`}</span>
          <div className={styles.rowHpBar}>
            <HpBar hp={hp} maxHp={maxHp} />
          </div>
        </div>
      </div>
    </div>
  );

  const cardContent = (
    <div
      onClick={isClickable ? onClick : undefined}
      className={`${styles.teamSummaryCard} ${stateClasses} ${
        selectionOrder != null ? styles.selectedCard : ""
      }`.trim()}
    >
      {selectionOrder != null && (
        <div className={styles.selectionBadge}>{selectionOrder}</div>
      )}
      {isActing && <div className={styles.actingBadge}>{actingBadgeText}</div>}
      <div className={styles.summaryCardHeader}>
        <span className={styles.summaryMonName} title={name}>{name}</span>
        {level != null && <span className={styles.summaryMonLevel}>L{level}</span>}
      </div>

      <div className={styles.summaryCardMetaRow}>
        <StatusBadge status={status} isFainted={isFainted} />
        <span className={styles.summaryHpText}>
          {hpText ?? `${hp}/${maxHp}`}
        </span>
      </div>

      <HpBar hp={hp} maxHp={maxHp} />
    </div>
  );

  const content = variant === "row" ? rowContent : cardContent;
  const tooltipPlacement = preferredPlacement ?? (variant === "row" ? "left" : "top");

  if (monBattleData || appearanceRef || monRef) {
    return (
      <MonTooltipTrigger
        mon={monBattleData}
        monRef={monRef}
        appearanceRef={appearanceRef}
        battleState={battleState}
        rules={rules}
        as="div"
        className={variant === "row" ? "w-full" : "flex-col w-full h-full"}
        preferredPlacement={tooltipPlacement}
      >
        {content}
      </MonTooltipTrigger>
    );
  }

  return content;
}
