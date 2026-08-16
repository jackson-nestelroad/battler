import type { CSSProperties } from "react";
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
        {badgeText && (
          <span className={`${styles.moveBadge} ${badgeClassName || ""}`}>{badgeText}</span>
        )}
      </div>
      {subtitle && <span className={styles.moveMeta}>{subtitle}</span>}
    </button>
  );
}
