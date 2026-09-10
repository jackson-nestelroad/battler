import type { ItemData } from "battler-types";
import SimpleDataTooltipCard from "./SimpleDataTooltipCard";

export interface ItemTooltipCardProps {
  data: ItemData;
}

export default function ItemTooltipCard({ data }: ItemTooltipCardProps) {
  return (
    <SimpleDataTooltipCard
      name={data.name}
      subtitle="Item"
      resourceType="item"
      flags={data.flags}
    />
  );
}
