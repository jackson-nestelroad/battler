import type { CSSProperties, ReactNode } from "react";
import styles from "./IconBadge.module.scss";

export interface IconBadgeProps {
  label: string;
  iconSrc?: string;
  iconAlt?: string;
  background?: string;
  size?: "sm" | "md";
  showIcon?: boolean;
  fixedWidth?: boolean;
  className?: string;
  iconClassName?: string;
  textClassName?: string;
  style?: CSSProperties;
  children?: ReactNode;
  dataAttributes?: Record<string, string | undefined>;
}

export default function IconBadge({
  label,
  iconSrc,
  iconAlt = "",
  background,
  size = "md",
  showIcon = true,
  fixedWidth = true,
  className,
  iconClassName,
  textClassName,
  style,
  children,
  dataAttributes,
}: IconBadgeProps) {
  const badgeClasses = [
    styles.iconBadge,
    size === "sm" ? styles.iconBadgeSm : styles.iconBadgeMd,
    fixedWidth && styles.fixedWidth,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  const combinedStyle: CSSProperties = {
    ...(background ? { background } : {}),
    ...style,
  };

  const iconClasses = [styles.icon, iconClassName].filter(Boolean).join(" ");
  const textClasses = [styles.text, textClassName].filter(Boolean).join(" ");

  return (
    <span
      className={badgeClasses}
      style={combinedStyle}
      {...dataAttributes}
    >
      {children}
      {showIcon && iconSrc && (
        <img
          src={iconSrc}
          alt={iconAlt}
          className={iconClasses}
          aria-hidden={!iconAlt}
          draggable={false}
        />
      )}
      <span className={textClasses}>{label}</span>
    </span>
  );
}
