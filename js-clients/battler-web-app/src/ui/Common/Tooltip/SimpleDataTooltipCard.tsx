import type { ReactNode } from "react";
import type { DescriptionData, ResourceType } from "../../../hooks/useDataStore";
import cardStyles from "./DataTooltipCard.module.scss";
import TooltipEffectButton from "./TooltipEffectButton";
import TooltipFlagsSection from "./TooltipFlagsSection";

export interface SimpleDataTooltipCardProps {
  name: string;
  subtitle: string;
  resourceType?: ResourceType;
  iconSrc?: string;
  flags?: Iterable<string> | null;
  description?: DescriptionData | null;
  children?: ReactNode;
  headerAction?: ReactNode;
}

export default function SimpleDataTooltipCard({
  name,
  subtitle,
  resourceType,
  iconSrc,
  flags,
  description,
  children,
  headerAction,
}: SimpleDataTooltipCardProps) {
  return (
    <article className={`${cardStyles.card} ${cardStyles.cardCompact}`}>
      <header className={cardStyles.header}>
        <div className="flex-row justify-between align-center">
          <div className="flex-row align-center gap-xs min-w-0">
            {iconSrc && (
              <img
                src={iconSrc}
                alt=""
                aria-hidden="true"
                className={cardStyles.headerIcon}
                draggable={false}
                onError={(e) => {
                  (e.currentTarget as HTMLElement).style.visibility = "hidden";
                }}
              />
            )}
            <span className={cardStyles.name}>{name}</span>
          </div>
          {(resourceType || headerAction) && (
            <div className="flex-row align-center gap-xs">
              {resourceType && <TooltipEffectButton type={resourceType} name={name} />}
              {headerAction}
            </div>
          )}
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

