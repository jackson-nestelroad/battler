interface RefreshButtonProps {
  onClick: () => void | Promise<void>;
  isRefreshing: boolean;
  disabled?: boolean;
  title?: string;
  className?: string;
}

export default function RefreshButton({
  onClick,
  isRefreshing,
  disabled,
  title = "Refresh",
  className = "",
}: RefreshButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`btn btn-secondary flex-row align-center gap-xs btn-sm ${className}`}
      disabled={disabled || isRefreshing}
      title={title}
    >
      <span className={isRefreshing ? "spin-icon" : ""}>↻</span>
      <span className="btn-text-desktop">Refresh</span>
    </button>
  );
}
