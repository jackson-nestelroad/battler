import type { ReactNode } from "react";
import type { DescriptionData } from "battler-data-service-client";
import type { AbilityData } from "battler-types";
import SimpleDataTooltipCard from "./SimpleDataTooltipCard";

export interface AbilityTooltipCardProps {
  data: AbilityData;
  description?: DescriptionData | null;
  headerAction?: ReactNode;
}

export default function AbilityTooltipCard({
  data,
  description,
  headerAction,
}: AbilityTooltipCardProps) {
  return (
    <SimpleDataTooltipCard
      name={data.name}
      subtitle="Ability"
      resourceType="ability"
      flags={data.flags}
      description={description}
      headerAction={headerAction}
    />
  );
}

