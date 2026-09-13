import type { MonData } from "battler-types";
import { monIconUrl } from "../../utils/assets";
import styles from "./TeamMonIcons.module.scss";

export interface TeamMonIconsProps {
  members?: Array<Partial<MonData> | null | undefined>;
  className?: string;
  size?: "sm" | "md";
  maxMons?: number;
}

export default function TeamMonIcons({
  members,
  className,
  size = "md",
  maxMons = 6,
}: TeamMonIconsProps) {
  if (!members || members.length === 0) {
    return null;
  }

  const validMembers = members.filter((m): m is Partial<MonData> =>
    Boolean(m && (m.species || m.name)),
  );

  if (validMembers.length === 0) {
    return null;
  }

  const displayedMembers = validMembers.slice(0, maxMons);
  const overflowMembers = validMembers.slice(maxMons);
  const overflowCount = overflowMembers.length;

  const overflowTooltip =
    overflowCount > 0
      ? `+${overflowCount} more: ${overflowMembers.map((m) => m.name || m.species).join(", ")}`
      : undefined;

  return (
    <div
      className={`${styles.container} ${styles[size]} ${className || ""}`.trim()}
      aria-label="Team Pokémon"
    >
      {displayedMembers.map((mon, idx) => {
        const species = mon.species || mon.name || "";
        const displayName = mon.name || mon.species || "Pokémon";
        const iconSrc = monIconUrl(species);

        return (
          <span
            key={`${species}-${idx}`}
            className={styles.iconWrapper}
            title={displayName}
          >
            <img
              src={iconSrc}
              alt={displayName}
              className={styles.monIcon}
              loading="lazy"
            />
          </span>
        );
      })}
      {overflowCount > 0 && (
        <span
          className={styles.overflowBadge}
          title={overflowTooltip}
          aria-label={overflowTooltip}
        >
          +{overflowCount}
        </span>
      )}
    </div>
  );
}
