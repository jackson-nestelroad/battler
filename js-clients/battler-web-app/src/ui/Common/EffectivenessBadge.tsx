import type { MonMoveSlotData, TypeChartData } from "battler-types";
import { formatMultiplier } from "../../hooks/useTypeChart";
import {
  calculateTargetEffectiveness,
  canCalculateEffectiveness,
  formatEffectivenessComparison,
  getMultiplierClass,
} from "../../utils/typeEffectiveness";
import styles from "./EffectivenessBadge.module.scss";

export interface EffectivenessBadgeProps {
  mult: number;
  attackerType?: string;
  targetTypes?: string[];
  className?: string;
}

export default function EffectivenessBadge({
  mult,
  attackerType,
  targetTypes,
  className,
}: EffectivenessBadgeProps) {
  const multClass = getMultiplierClass(mult);
  const multText = formatMultiplier(mult);
  const title = formatEffectivenessComparison(attackerType, targetTypes, mult);

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
      attackerType={move.type}
      targetTypes={targetTypes}
      className={className}
    />
  );
}
