import type { CSSProperties, KeyboardEvent } from "react";
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
        className={styles.infoBtn}
        showUnderline={false}
        ariaLabel={`View ${infoResourceName} details`}
        title={`View ${infoResourceName} details`}
      >
        <svg
          width="12"
          height="12"
          viewBox="0 0 16 16"
          fill="currentColor"
          aria-hidden="true"
        >
          <path d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1Zm0 2.5a1.1 1.1 0 1 1 0 2.2 1.1 1.1 0 0 1 0-2.2ZM6.75 7.5h1.75v4.25h1.25v1H6.25v-1h1.25V8.5H6.75v-1Z" />
        </svg>
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
        {(badgeText || infoButton) && (
          <div className={styles.moveHeaderRight}>
            {badgeText && (
              <span className={combinedBadgeClass}>{badgeText}</span>
            )}
            {infoButton}
          </div>
        )}
      </div>
      {subtitle && <span className={styles.moveMeta}>{subtitle}</span>}
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
