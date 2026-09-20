import type { CSSProperties, KeyboardEvent, ReactNode } from "react";
import InfoIcon from "../Common/InfoIcon";
import DataTooltipTrigger, { type DataResourceType } from "../Common/Tooltip/DataTooltipTrigger";
import styles from "./ActionButton.module.scss";

export interface ActionButtonProps {
  title: string;
  subtitle?: string;
  onClick?: () => void;
  disabled?: boolean;
  style?: CSSProperties;
  className?: string;
  typeColor?: string;
  badgeText?: string | null;
  badgeClassName?: string;
  badgeVariant?: "zmove" | "maxMove";
  htmlTitle?: string;
  infoResourceType?: DataResourceType;
  infoResourceName?: string;
  effectivenessBadge?: ReactNode;
}

export default function ActionButton({
  title,
  subtitle,
  onClick,
  disabled,
  style,
  className,
  typeColor,
  badgeText,
  badgeClassName,
  badgeVariant,
  htmlTitle,
  infoResourceType,
  infoResourceName,
  effectivenessBadge,
}: ActionButtonProps) {
  const combinedClassName = `${styles.moveBtn} type-border ${className || ""}`.trim();
  const combinedStyle: CSSProperties = {
    ...style,
    ...(typeColor ? ({ "--type-color": typeColor } as CSSProperties) : {}),
  };

  const infoButton =
    infoResourceType && infoResourceName ? (
      <DataTooltipTrigger
        resourceType={infoResourceType}
        name={infoResourceName}
        className="info-btn"
        showUnderline={false}
        ariaLabel={`View ${infoResourceName} details`}
        title={`View ${infoResourceName} details`}
      >
        <InfoIcon />
      </DataTooltipTrigger>
    ) : null;

  const hasInfoButton = Boolean(infoButton);

  const resolvedBadgeVariant =
    badgeVariant ??
    (badgeText === "Z-Move"
      ? "zmove"
      : badgeText === "Max Move"
        ? "maxMove"
        : undefined);
  const badgeVariantClass =
    resolvedBadgeVariant === "zmove"
      ? styles.zmoveBadge
      : resolvedBadgeVariant === "maxMove"
        ? styles.maxMoveBadge
        : "";
  const combinedBadgeClass = [styles.moveBadge, badgeVariantClass, badgeClassName]
    .filter(Boolean)
    .join(" ");

  const content = (
    <>
      <div className={styles.moveHeaderRow}>
        <span className={styles.moveName}>{title}</span>
        {(effectivenessBadge || infoButton) && (
          <div className={styles.moveHeaderRight}>
            {effectivenessBadge}
            {infoButton}
          </div>
        )}
      </div>
      {(subtitle || badgeText) && (
        <div className={styles.moveMetaRow}>
          {subtitle && <span className={styles.moveMeta}>{subtitle}</span>}
          {badgeText && (
            <span className={combinedBadgeClass}>{badgeText}</span>
          )}
        </div>
      )}
    </>
  );

  const handleDivKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    if (disabled) return;
    if (e.target !== e.currentTarget) return;
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      onClick?.();
    }
  };

  if (hasInfoButton) {
    return (
      <div
        className={combinedClassName}
        style={combinedStyle}
        title={htmlTitle}
        role="button"
        tabIndex={disabled ? -1 : 0}
        aria-disabled={disabled}
        onClick={disabled ? undefined : onClick}
        onKeyDown={handleDivKeyDown}
      >
        {content}
      </div>
    );
  }

  return (
    <button
      type="button"
      className={combinedClassName}
      style={combinedStyle}
      title={htmlTitle}
      disabled={disabled}
      onClick={disabled ? undefined : onClick}
    >
      {content}
    </button>
  );
}
