import type { DescriptionData } from "battler-data-service-client";
import type { AbilityData } from "battler-types";
import SimpleDataTooltipCard from "./SimpleDataTooltipCard";

export interface AbilityTooltipCardProps {
  data: AbilityData;
  description?: DescriptionData | null;
}

export default function AbilityTooltipCard({ data, description }: AbilityTooltipCardProps) {
  return (
    <SimpleDataTooltipCard
      name={data.name}
      subtitle="Ability"
      resourceType="ability"
      flags={data.flags}
      description={description}
    />
  );
}

