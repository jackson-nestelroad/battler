import type { DescriptionData } from "battler-data-service-client";
import { formatMoveEffects } from "battler-log-formatter";
import type { MoveData } from "battler-types";
import {
  formatAccuracy,
  formatBasePower,
  formatPp,
  formatPriority,
} from "../../../utils/dataTooltipFormatting";
import CategoryBadge from "../CategoryBadge";
import TypeBadge from "../TypeBadge";
import cardStyles from "./DataTooltipCard.module.scss";
import TooltipEffectButton from "./TooltipEffectButton";
import TooltipFlagsSection from "./TooltipFlagsSection";

export interface MoveTooltipCardProps {
  data: MoveData;
  description?: DescriptionData | null;
}

export default function MoveTooltipCard({ data, description }: MoveTooltipCardProps) {
  const basePowerStr = formatBasePower(data.base_power);
  const accuracyStr = formatAccuracy(data.accuracy);
  const ppStr = formatPp(data.pp, data.no_pp_boosts);
  const priorityStr = formatPriority(data.priority);
  const effects = formatMoveEffects(data);

  return (
    <article className={`${cardStyles.card} ${cardStyles.cardFixed}`}>
      <header className={cardStyles.header}>
        <div className="flex-row justify-between align-center">
          <span className={cardStyles.name}>{data.name}</span>
          <TooltipEffectButton type="move" name={data.name} />
        </div>
        <span className={cardStyles.subtitle}>Move</span>
        <div className="flex-row align-center gap-xs">
          <CategoryBadge category={data.category} size="sm" />
          <TypeBadge type={data.primary_type} size="sm" interactive />
        </div>
      </header>

      {description?.description && (
        <p className={cardStyles.description}>{description.description}</p>
      )}

      {effects.length > 0 && (
        <section className={cardStyles.effectsSection}>
          <span className={cardStyles.sectionTitle}>Effects</span>
          <div className={cardStyles.effectsList}>
            {effects.map((eff, i) => (
              <p key={i} className={cardStyles.effectItem}>
                {eff.text}
              </p>
            ))}
          </div>
        </section>
      )}

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
