import type { CSSProperties } from "react";
import styles from "./TypeBadge.module.scss";

export interface TypeBadgeProps {
  type: string;
  size?: "sm" | "md";
  className?: string;
}

export default function TypeBadge({
  type,
  size = "md",
  className,
}: TypeBadgeProps) {
  const normalizedType = type.trim().toLowerCase();
  const sizeClass = size === "sm" ? styles.typeBadgeSm : styles.typeBadgeMd;
  const badgeClasses = `${styles.typeBadge} ${sizeClass}${className ? ` ${className}` : ""}`;

  return (
    <span
      className={badgeClasses}
      style={
        {
          backgroundColor: `var(--color-type-${normalizedType}, var(--border-color))`,
        } as CSSProperties
      }
      data-type={normalizedType}
    >
      {type}
    </span>
  );
}
