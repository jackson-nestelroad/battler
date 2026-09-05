import { formatStatusBadge } from "../../utils/monHelpers";

interface StatusBadgeProps {
  status?: string | null;
  isFainted?: boolean;
  className?: string;
}

export default function StatusBadge({ status, isFainted, className }: StatusBadgeProps) {
  const badge = formatStatusBadge(status);
  const badgeClass = className ? ` ${className}` : "";

  if (isFainted || badge?.code === "fnt") {
    return <span className={`status-badge fnt${badgeClass}`}>FNT</span>;
  }
  if (badge) {
    return <span className={`status-badge ${badge.code}${badgeClass}`}>{badge.label}</span>;
  }
  return <span className={`status-badge ok${badgeClass}`}>OK</span>;
}
