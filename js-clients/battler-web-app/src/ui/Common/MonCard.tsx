import { useContext } from "react";
import { stateSelectors, type BattleState, type MonBattleAppearanceReference, type UiMon } from "battler-state";
import type { MonBattleData } from "battler-types";
import { monIconUrl } from "../../utils/assets";
import { normalizeStatusCode } from "../../utils/monHelpers";
import HpBar from "./HpBar";
import InfoIcon from "./InfoIcon";
import StatusBadge from "./StatusBadge";
import styles from "./MonCard.module.scss";
import { MonTooltipContext } from "./Tooltip/TooltipContext";
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

interface MonCardContentProps {
  name: string;
  hp: number;
  maxHp: number;
  hpText?: string;
  status: string | null;
  isClickable: boolean;
  onClick?: () => void;
  selectionOrder?: number;
  isActing?: boolean;
  actingBadgeText: string;
  variant: "card" | "row";
  isUnbrought: boolean;
  iconSrc: string | null;
  stateClasses: string;
  isFainted: boolean;
  hasMonData: boolean;
}

function MonCardContent({
  name,
  hp,
  maxHp,
  hpText,
  status,
  isClickable,
  onClick,
  selectionOrder,
  isActing,
  actingBadgeText,
  variant,
  isUnbrought,
  iconSrc,
  stateClasses,
  isFainted,
  hasMonData,
}: MonCardContentProps) {
  const monTooltip = useContext(MonTooltipContext);

  const handleCardClick = () => {
    monTooltip?.close();
    if (isClickable) {
      onClick?.();
    }
  };

  const infoButton = hasMonData ? (
    <span
      role="button"
      tabIndex={0}
      className="info-btn"
      aria-label={`View ${name} details`}
      title={`View ${name} details`}
      onClick={(e) => {
        e.stopPropagation();
        monTooltip?.toggle(e.currentTarget);
      }}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          e.stopPropagation();
          monTooltip?.toggle(e.currentTarget);
        }
      }}
    >
      <InfoIcon />
    </span>
  ) : null;

  if (variant === "row") {
    return (
      <div
        onClick={handleCardClick}
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
          {infoButton}
        </div>
      </div>
    );
  }

  return (
    <div
      onClick={handleCardClick}
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
            {infoButton}
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
    if (variant === "row") {
      return (
        <div className={`${styles.teamSummaryRow} ${styles.unrevealed}`}>
          <div className={styles.rowIdentity}>
            <span className={styles.unrevealedPip} aria-hidden="true">
              ○
            </span>
            <span className={styles.summaryMonName} title="Unrevealed">Unrevealed</span>
          </div>
        </div>
      );
    }

    return (
      <div className={`${styles.teamSummaryCard} ${styles.unrevealed}`}>
        <div className={styles.cardMain}>
          <span className={styles.unrevealedPip} aria-hidden="true">
            ○
          </span>
          <div className={styles.cardDetails}>
            <div className={styles.summaryCardHeader}>
              <span className={styles.summaryMonName} title="Unrevealed">Unrevealed</span>
            </div>
          </div>
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

  const hasMonData = Boolean(monBattleData || appearanceRef || monRef);
  const tooltipPlacement = preferredPlacement ?? (variant === "row" ? "left" : "top");

  const content = (
    <MonCardContent
      name={name}
      hp={hp}
      maxHp={maxHp}
      hpText={hpText}
      status={status}
      isClickable={isClickable}
      onClick={onClick}
      selectionOrder={selectionOrder}
      isActing={isActing}
      actingBadgeText={actingBadgeText}
      variant={variant}
      isUnbrought={isUnbrought}
      iconSrc={iconSrc}
      stateClasses={stateClasses}
      isFainted={isFainted}
      hasMonData={hasMonData}
    />
  );

  if (hasMonData) {
    return (
      <MonTooltipTrigger
        mon={monBattleData}
        monRef={monRef}
        appearanceRef={appearanceRef}
        battleState={battleState}
        rules={rules}
        preferredPlacement={tooltipPlacement}
        as="div"
        className={variant === "row" ? "w-full" : "flex-col w-full h-full"}
        disableClick={true}
      >
        {content}
      </MonTooltipTrigger>
    );
  }

  return content;
}
