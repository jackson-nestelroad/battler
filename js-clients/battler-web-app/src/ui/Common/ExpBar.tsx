import styles from "./ExpBar.module.scss";

interface ExpBarProps {
  progressPercent: number;
  className?: string;
}

export default function ExpBar({ progressPercent, className }: ExpBarProps) {
  const percent = Math.max(0, Math.min(100, Math.round(progressPercent)));
  const containerClass = `${styles.expBarContainer}${className ? ` ${className}` : ""}`;

  return (
    <div
      className={containerClass}
      aria-label={`EXP progress ${percent}%`}
      role="progressbar"
      aria-valuenow={percent}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div className={styles.expBarFill} style={{ width: `${percent}%` }} />
    </div>
  );
}
