import type { DescriptionData } from "battler-data-service-client";
import type { ItemData } from "battler-types";
import SimpleDataTooltipCard from "./SimpleDataTooltipCard";

export interface ItemTooltipCardProps {
  data: ItemData;
  description?: DescriptionData | null;
}

export default function ItemTooltipCard({ data, description }: ItemTooltipCardProps) {
  return (
    <SimpleDataTooltipCard
      name={data.name}
      subtitle="Item"
      resourceType="item"
      flags={data.flags}
      description={description}
    />
  );
}

