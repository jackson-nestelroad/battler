import { formatStatusBadge } from "../../utils/monHelpers";
import DataTooltipTrigger from "./Tooltip/DataTooltipTrigger";

interface StatusBadgeProps {
  status?: string | null;
  isFainted?: boolean;
  className?: string;
  interactive?: boolean;
}

export default function StatusBadge({
  status,
  isFainted,
  className,
  interactive = false,
}: StatusBadgeProps) {
  const badge = formatStatusBadge(status);
  const badgeClass = className ? ` ${className}` : "";

  if (isFainted || badge?.code === "fnt") {
    return <span className={`status-badge fnt${badgeClass}`}>FNT</span>;
  }
  if (badge) {
    const badgeEl = (
      <span className={`status-badge ${badge.code}${badgeClass}`}>{badge.label}</span>
    );
    if (interactive) {
      return (
        <DataTooltipTrigger
          resourceType="condition"
          name={status ?? badge.label}
          showUnderline={false}
        >
          {badgeEl}
        </DataTooltipTrigger>
      );
    }
    return badgeEl;
  }
  return <span className={`status-badge ok${badgeClass}`}>OK</span>;
}
