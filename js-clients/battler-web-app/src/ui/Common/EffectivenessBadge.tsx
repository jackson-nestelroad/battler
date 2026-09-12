import type { MonMoveSlotData, TypeChartData } from "battler-types";
import { formatMultiplier } from "../../hooks/useTypeChart";
import {
  calculateTargetEffectiveness,
  canCalculateEffectiveness,
  getEffectivenessTitle,
  getMultiplierClass,
} from "../../utils/typeEffectiveness";
import styles from "./EffectivenessBadge.module.scss";

export interface EffectivenessBadgeProps {
  mult: number;
  targetTypes?: string[];
  className?: string;
}

export default function EffectivenessBadge({
  mult,
  targetTypes,
  className,
}: EffectivenessBadgeProps) {
  const multClass = getMultiplierClass(mult);
  const multText = formatMultiplier(mult);
  const title =
    targetTypes && targetTypes.length > 0
      ? `${getEffectivenessTitle(mult)} against ${targetTypes.join("/")}`
      : getEffectivenessTitle(mult);

  const combinedClassName = [
    styles.effectivenessBadge,
    multClass,
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <span
      className={combinedClassName}
      title={title}
      aria-label={title}
    >
      {multText}
    </span>
  );
}

export interface MoveEffectivenessBadgeProps {
  typeChart: TypeChartData | null;
  move?: MonMoveSlotData | null;
  targetTypes?: string[] | null;
  className?: string;
}

export function MoveEffectivenessBadge({
  typeChart,
  move,
  targetTypes,
  className,
}: MoveEffectivenessBadgeProps) {
  if (
    !move ||
    !targetTypes ||
    targetTypes.length === 0 ||
    !canCalculateEffectiveness(move)
  ) {
    return null;
  }

  const mult = calculateTargetEffectiveness(typeChart, move.type, targetTypes);
  return (
    <EffectivenessBadge
      mult={mult}
      targetTypes={targetTypes}
      className={className}
    />
  );
}
