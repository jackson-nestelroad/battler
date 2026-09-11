import type { ReactNode } from "react";
import type { DescriptionData, ResourceType } from "../../../hooks/useDataStore";
import cardStyles from "./DataTooltipCard.module.scss";
import TooltipEffectButton from "./TooltipEffectButton";
import TooltipFlagsSection from "./TooltipFlagsSection";

export interface SimpleDataTooltipCardProps {
  name: string;
  subtitle: string;
  resourceType?: ResourceType;
  flags?: Iterable<string> | null;
  description?: DescriptionData | null;
  children?: ReactNode;
}

export default function SimpleDataTooltipCard({
  name,
  subtitle,
  resourceType,
  flags,
  description,
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

      {description?.description && (
        <p className={cardStyles.description}>{description.description}</p>
      )}

      {children}
      <TooltipFlagsSection flags={flags} />
    </article>
  );
}

