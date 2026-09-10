import { useContext, type ReactNode } from "react";
import type { ResourceType } from "../../../hooks/useDataStore";
import { FxLangModalContext } from "../FxLang/FxLangModalContext";
import cardStyles from "./DataTooltipCard.module.scss";
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
  const fxModal = useContext(FxLangModalContext);

  return (
    <article className={`${cardStyles.card} ${cardStyles.cardCompact}`}>
      <header className={cardStyles.header}>
        <div className="flex-row justify-between align-center">
          <span className={cardStyles.name}>{name}</span>
          {resourceType && (
            <button
              type="button"
              className={cardStyles.effectBtn}
              onClick={(e) => {
                e.stopPropagation();
                fxModal?.openFxLangModal({ type: resourceType, name });
              }}
            >
              Effect
            </button>
          )}
        </div>
        <span className={cardStyles.subtitle}>{subtitle}</span>
      </header>

      {children}
      <TooltipFlagsSection flags={flags} />
    </article>
  );
}
