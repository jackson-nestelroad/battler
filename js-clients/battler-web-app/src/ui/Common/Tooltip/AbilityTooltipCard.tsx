import type { AbilityData } from "battler-types";
import SimpleDataTooltipCard from "./SimpleDataTooltipCard";

export interface AbilityTooltipCardProps {
  data: AbilityData;
}

export default function AbilityTooltipCard({ data }: AbilityTooltipCardProps) {
  return (
    <SimpleDataTooltipCard
      name={data.name}
      subtitle="Ability"
      flags={data.flags}
    />
  );
}
