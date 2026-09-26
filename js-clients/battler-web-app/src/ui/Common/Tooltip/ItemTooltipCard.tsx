import type { ReactNode } from "react";
import type { DescriptionData } from "battler-data-service-client";
import type { ItemData } from "battler-types";
import { itemIconUrl } from "../../../utils/assets";
import SimpleDataTooltipCard from "./SimpleDataTooltipCard";

export interface ItemTooltipCardProps {
  data: ItemData;
  description?: DescriptionData | null;
  headerAction?: ReactNode;
}

export default function ItemTooltipCard({
  data,
  description,
  headerAction,
}: ItemTooltipCardProps) {
  return (
    <SimpleDataTooltipCard
      name={data.name}
      subtitle="Item"
      resourceType="item"
      iconSrc={itemIconUrl(data.name)}
      flags={data.flags}
      description={description}
      headerAction={headerAction}
    />
  );
}

