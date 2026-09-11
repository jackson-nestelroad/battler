import type { DescriptionData } from "battler-data-service-client";
import type { ConditionData } from "battler-types";
import cardStyles from "./DataTooltipCard.module.scss";
import SimpleDataTooltipCard from "./SimpleDataTooltipCard";

export interface ConditionTooltipCardProps {
  data: ConditionData;
  description?: DescriptionData | null;
}

export default function ConditionTooltipCard({ data, description }: ConditionTooltipCardProps) {
  return (
    <SimpleDataTooltipCard
      name={data.name}
      subtitle={data.condition_type}
      resourceType="condition"
      description={description}
    >
      {data.no_copy && (
        <section className={cardStyles.traitsGrid}>
          <div className={cardStyles.traitRow}>
            <span className={cardStyles.traitLabel}>Baton Pass:</span>
            <span className={cardStyles.traitValue}>No copy</span>
          </div>
        </section>
      )}
    </SimpleDataTooltipCard>
  );
}

