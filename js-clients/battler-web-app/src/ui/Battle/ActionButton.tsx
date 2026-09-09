import type { CSSProperties } from "react";
import DataTooltipTrigger, { type DataResourceType } from "../Common/Tooltip/DataTooltipTrigger";
import styles from "./ActionPanel.module.scss";

interface ActionButtonProps {
  title: string;
  subtitle?: string;
  onClick: () => void;
  disabled?: boolean;
  typeColor?: string; // e.g. "var(--color-primary)" or `var(--color-type-fire)`
  badgeText?: string | null;
  badgeClassName?: string;
  htmlTitle?: string;
  infoResourceType?: DataResourceType;
  infoResourceName?: string;
}

export default function ActionButton({
  title,
  subtitle,
  onClick,
  disabled = false,
  typeColor = "var(--color-primary)",
  badgeText,
  badgeClassName,
  htmlTitle,
  infoResourceType,
  infoResourceName,
}: ActionButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`${styles.moveBtn} type-border`}
      style={{ "--type-color": typeColor } as CSSProperties}
      disabled={disabled}
      title={htmlTitle}
    >
      <div className={styles.moveHeaderRow}>
        <span className={styles.moveName}>{title}</span>
        <div className={styles.moveHeaderRight}>
          {badgeText && (
            <span className={`${styles.moveBadge} ${badgeClassName || ""}`}>{badgeText}</span>
          )}
          {infoResourceType && infoResourceName && (
            <DataTooltipTrigger
              resourceType={infoResourceType}
              name={infoResourceName}
              className={styles.infoBtn}
              showUnderline={false}
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
          )}
        </div>
      </div>
      {subtitle && <span className={styles.moveMeta}>{subtitle}</span>}
    </button>
  );
}
