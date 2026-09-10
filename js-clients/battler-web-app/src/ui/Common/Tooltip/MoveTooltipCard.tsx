import { useContext } from "react";
import type { MoveData } from "battler-types";
import {
  formatAccuracy,
  formatBasePower,
  formatPp,
  formatPriority,
} from "../../../utils/dataTooltipFormatting";
import TypeBadge from "../TypeBadge";
import { FxLangModalContext } from "../FxLang/FxLangModalContext";
import cardStyles from "./DataTooltipCard.module.scss";
import TooltipFlagsSection from "./TooltipFlagsSection";

export interface MoveTooltipCardProps {
  data: MoveData;
}

export default function MoveTooltipCard({ data }: MoveTooltipCardProps) {
  const fxModal = useContext(FxLangModalContext);
  const basePowerStr = formatBasePower(data.base_power);
  const accuracyStr = formatAccuracy(data.accuracy);
  const ppStr = formatPp(data.pp, data.no_pp_boosts);
  const priorityStr = formatPriority(data.priority);

  const categoryClass = `${cardStyles.categoryBadge} ${cardStyles[`category${data.category}`] || cardStyles.categoryStatus}`;

  return (
    <article className={`${cardStyles.card} ${cardStyles.cardFixed}`}>
      <header className={cardStyles.header}>
        <div className="flex-row justify-between align-center">
          <span className={cardStyles.name}>{data.name}</span>
          <button
            type="button"
            className={cardStyles.effectBtn}
            onClick={(e) => {
              e.stopPropagation();
              fxModal?.openFxLangModal({ type: "move", name: data.name });
            }}
          >
            Effect
          </button>
        </div>
        <span className={cardStyles.subtitle}>Move</span>
        <div className="flex-row align-center gap-xs">
          <span className={categoryClass}>{data.category}</span>
          <TypeBadge type={data.primary_type} size="sm" />
        </div>
      </header>

      <section className={cardStyles.traitsGrid}>
        <div className={cardStyles.traitRow}>
          <span className={cardStyles.traitLabel}>Base Power:</span>
          <span className={cardStyles.traitValue}>{basePowerStr}</span>
        </div>
        <div className={cardStyles.traitRow}>
          <span className={cardStyles.traitLabel}>Accuracy:</span>
          <span className={cardStyles.traitValue}>{accuracyStr}</span>
        </div>
        <div className={cardStyles.traitRow}>
          <span className={cardStyles.traitLabel}>PP:</span>
          <span className={cardStyles.traitValue}>{ppStr}</span>
        </div>
        {priorityStr && (
          <div className={cardStyles.traitRow}>
            <span className={cardStyles.traitLabel}>Priority:</span>
            <span className={cardStyles.traitValue}>{priorityStr}</span>
          </div>
        )}
        <div className={cardStyles.traitRow}>
          <span className={cardStyles.traitLabel}>Target:</span>
          <span className={cardStyles.traitValue}>{data.target}</span>
        </div>
      </section>

      <TooltipFlagsSection flags={data.flags} />
    </article>
  );
}
