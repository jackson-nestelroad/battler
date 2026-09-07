import type { CSSProperties } from "react";
import styles from "./TypeBadge.module.scss";

export interface TypeBadgeProps {
  type: string;
  size?: "sm" | "md";
  showIcon?: boolean;
  fixedWidth?: boolean;
  className?: string;
}

export default function TypeBadge({
  type,
  size = "md",
  showIcon = true,
  fixedWidth = false,
  className,
}: TypeBadgeProps) {
  const normalizedType = type.trim().toLowerCase();
  const typeKey = normalizedType === "???" ? "unknown" : normalizedType;
  const iconName = typeKey;
  const sizeClass = size === "sm" ? styles.typeBadgeSm : styles.typeBadgeMd;
  const fixedWidthClass = fixedWidth ? styles.fixedWidth : "";
  const badgeClasses = `${styles.typeBadge} ${sizeClass}${fixedWidthClass ? ` ${fixedWidthClass}` : ""}${className ? ` ${className}` : ""}`;
  const baseUrl = import.meta.env?.BASE_URL ?? "/";

  return (
    <span
      className={badgeClasses}
      style={
        {
          background: `var(--background-type-${typeKey}, var(--color-type-${typeKey}, var(--border-color)))`,
        } as CSSProperties
      }
      data-type={typeKey}
    >
      {showIcon && (
        <img
          src={`${baseUrl}assets/types/${iconName}.png`}
          alt=""
          className={styles.typeIcon}
          aria-hidden="true"
          draggable={false}
        />
      )}
      <span className={styles.typeText}>{type}</span>
    </span>
  );
}
