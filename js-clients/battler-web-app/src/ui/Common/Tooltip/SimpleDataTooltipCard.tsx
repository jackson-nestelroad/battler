import type { ReactNode } from "react";
import cardStyles from "./DataTooltipCard.module.scss";
import TooltipFlagsSection from "./TooltipFlagsSection";

export interface SimpleDataTooltipCardProps {
  name: string;
  subtitle: string;
  flags?: Iterable<string> | null;
  children?: ReactNode;
}

export default function SimpleDataTooltipCard({
  name,
  subtitle,
  flags,
  children,
}: SimpleDataTooltipCardProps) {
  return (
    <article className={`${cardStyles.card} ${cardStyles.cardCompact}`}>
      <header className={cardStyles.header}>
        <span className={cardStyles.name}>{name}</span>
        <span className={cardStyles.subtitle}>{subtitle}</span>
      </header>

      {children}
      <TooltipFlagsSection flags={flags} />
    </article>
  );
}
