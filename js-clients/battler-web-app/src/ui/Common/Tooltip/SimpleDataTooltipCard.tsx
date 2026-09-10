import type { ReactNode } from "react";
import type { ResourceType } from "../../../hooks/useDataStore";
import cardStyles from "./DataTooltipCard.module.scss";
import TooltipEffectButton from "./TooltipEffectButton";
import TooltipFlagsSection from "./TooltipFlagsSection";

export interface SimpleDataTooltipCardProps {
  name: string;
  subtitle: string;
  resourceType?: ResourceType;
  flags?: Iterable<string> | null;
  children?: ReactNode;
}

export default function SimpleDataTooltipCard({
  name,
  subtitle,
  resourceType,
  flags,
  children,
}: SimpleDataTooltipCardProps) {
  return (
    <article className={`${cardStyles.card} ${cardStyles.cardCompact}`}>
      <header className={cardStyles.header}>
        <div className="flex-row justify-between align-center">
          <span className={cardStyles.name}>{name}</span>
          {resourceType && <TooltipEffectButton type={resourceType} name={name} />}
        </div>
        <span className={cardStyles.subtitle}>{subtitle}</span>
      </header>

      {children}
      <TooltipFlagsSection flags={flags} />
    </article>
  );
}
