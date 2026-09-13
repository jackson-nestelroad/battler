import { stateSelectors, type BattleState, type MonBattleAppearanceReference, type UiMon } from "battler-state";
import type { MonBattleData } from "battler-types";
import { monIconUrl } from "../../utils/assets";
import { normalizeStatusCode } from "../../utils/monHelpers";
import HpBar from "./HpBar";
import StatusBadge from "./StatusBadge";
import styles from "./MonCard.module.scss";
import MonTooltipTrigger from "./Tooltip/MonTooltipTrigger";

export interface MonCardProps {
  name?: string;
  species?: string;
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
  species,
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

  const resolvedSpecies =
    species ||
    monBattleData?.species ||
    (appearanceRef && battleState
      ? stateSelectors.monPhysicalAppearance(battleState, appearanceRef)?.species ||
        stateSelectors.monSpecies(battleState, appearanceRef)
      : undefined) ||
    name;

  const iconSrc = resolvedSpecies ? monIconUrl(resolvedSpecies) : null;

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
        {iconSrc && (
          <img
            src={iconSrc}
            alt=""
            aria-hidden="true"
            className={styles.rowMonIcon}
            draggable={false}
          />
        )}
        <span className={styles.summaryMonName} title={name}>{name}</span>
        {isActing && <span className={styles.rowActingBadge}>{actingBadgeText}</span>}
      </div>

      <div className={styles.rowMeta}>
        <StatusBadge status={status} isFainted={isFainted} isUnbrought={isUnbrought} />
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
      <div className={styles.cardMain}>
        {iconSrc && (
          <img
            src={iconSrc}
            alt=""
            aria-hidden="true"
            className={styles.cardMonIcon}
            draggable={false}
          />
        )}
        <div className={styles.cardDetails}>
          <div className={styles.summaryCardHeader}>
            <span className={styles.summaryMonName} title={name}>{name}</span>
          </div>

          <div className={styles.summaryCardMetaRow}>
            <StatusBadge status={status} isFainted={isFainted} isUnbrought={isUnbrought} />
            <span className={styles.summaryHpText}>
              {hpText ?? `${hp}/${maxHp}`}
            </span>
          </div>
        </div>
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
