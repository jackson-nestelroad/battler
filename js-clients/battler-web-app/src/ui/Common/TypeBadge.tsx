import type { CSSProperties } from "react";
import styles from "./TypeBadge.module.scss";

export interface TypeBadgeProps {
  type: string;
  size?: "sm" | "md";
  variant?: "standard" | "tera";
  showIcon?: boolean;
  fixedWidth?: boolean;
  className?: string;
}

export default function TypeBadge({
  type,
  size = "md",
  variant = "standard",
  showIcon = true,
  fixedWidth = false,
  className,
}: TypeBadgeProps) {
  const normalizedType = type.trim().toLowerCase();
  const typeKey = normalizedType === "???" ? "unknown" : normalizedType;
  const iconName = typeKey;
  const sizeClass = size === "sm" ? styles.typeBadgeSm : styles.typeBadgeMd;
  const variantClass = variant === "tera" ? styles.typeBadgeTera : "";
  const fixedWidthClass = fixedWidth ? styles.fixedWidth : "";
  const badgeClasses = `${styles.typeBadge} ${sizeClass}${variantClass ? ` ${variantClass}` : ""}${fixedWidthClass ? ` ${fixedWidthClass}` : ""}${className ? ` ${className}` : ""}`;
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
      data-variant={variant}
    >
      {variant === "tera" && (
        <>
          <svg
            className={styles.teraCapLeft}
            viewBox="0 0 10 20"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            <polygon points="0,10 3,0 10,10" className={styles.facetHighlight} />
            <polygon points="0,10 3,20 10,10" className={styles.facetShadow} />
            <line x1="0" y1="10" x2="3" y2="0" className={styles.lineHighlightSpecular} />
            <line x1="3" y1="0" x2="10" y2="10" className={styles.lineHighlight} />
            <line x1="0" y1="10" x2="3" y2="20" className={styles.lineShadow} />
            <line x1="3" y1="20" x2="10" y2="10" className={styles.lineShadow} />
          </svg>
          <svg
            className={styles.teraCapRight}
            viewBox="0 0 10 20"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            <polygon points="10,10 7,0 0,10" className={styles.facetHighlightSpecular} />
            <polygon points="10,10 7,20 0,10" className={styles.facetShadow} />
            <line x1="10" y1="10" x2="7" y2="0" className={styles.lineHighlightSpecular} />
            <line x1="7" y1="0" x2="0" y2="10" className={styles.lineHighlight} />
            <line x1="10" y1="10" x2="7" y2="20" className={styles.lineShadow} />
            <line x1="7" y1="20" x2="0" y2="10" className={styles.lineShadow} />
          </svg>
        </>
      )}
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
