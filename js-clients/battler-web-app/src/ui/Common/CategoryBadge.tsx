import IconBadge from "./IconBadge";
import styles from "./CategoryBadge.module.scss";

export interface CategoryBadgeProps {
  category: string;
  size?: "sm" | "md";
  showIcon?: boolean;
  fixedWidth?: boolean;
  className?: string;
}

export default function CategoryBadge({
  category,
  size = "md",
  showIcon = true,
  fixedWidth = true,
  className,
}: CategoryBadgeProps) {
  const normalizedCategory = category.trim().toLowerCase();
  const validCategory =
    normalizedCategory === "physical" ||
    normalizedCategory === "special" ||
    normalizedCategory === "status"
      ? normalizedCategory
      : "status";

  const baseUrl = import.meta.env?.BASE_URL ?? "/";

  const categoryClasses = [
    styles.categoryBadge,
    size === "sm" ? styles.categoryBadgeSm : styles.categoryBadgeMd,
    fixedWidth && styles.fixedWidth,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <IconBadge
      label={category}
      iconSrc={`${baseUrl}assets/categories/${validCategory}.png`}
      background={`var(--color-category-${validCategory}, var(--border-color))`}
      size={size}
      showIcon={showIcon}
      fixedWidth={fixedWidth}
      className={categoryClasses}
      dataAttributes={{
        "data-category": validCategory,
      }}
    />
  );
}
