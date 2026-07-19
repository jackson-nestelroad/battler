import { useState, useEffect } from "react";
import { formatSeconds } from "../../utils/battle";

interface CountdownTimerProps {
  deadlineSecs: number;
  onExpiry?: () => void;
  prefix?: string;
  className?: string;
  badgeMode?: boolean;
  badgeClassOverride?: string;
}

export default function CountdownTimer({
  deadlineSecs,
  onExpiry,
  prefix = "",
  className = "",
  badgeMode = false,
  badgeClassOverride,
}: CountdownTimerProps) {
  const [timeLeft, setTimeLeft] = useState<number | null>(null);

  useEffect(() => {
    let hasNotifiedExpiry = false;

    const updateTimer = () => {
      const now = Math.floor(Date.now() / 1000);
      const diff = deadlineSecs - now;
      const remaining = Math.max(0, diff);
      setTimeLeft(remaining);

      if (remaining === 0) {
        if (!hasNotifiedExpiry && onExpiry) {
          hasNotifiedExpiry = true;
          onExpiry();
        }
        clearInterval(interval);
      }
    };

    const interval = setInterval(updateTimer, 1000);
    updateTimer(); // Initial call

    return () => clearInterval(interval);
  }, [deadlineSecs, onExpiry]);

  if (timeLeft === null) return null;

  const formattedTime = formatSeconds(timeLeft);

  if (badgeMode) {
    const badgeClass = timeLeft < 15 ? "badge-danger" : badgeClassOverride || "badge-warning";
    return (
      <div className={`badge ${badgeClass} badge-timer ${className}`}>
        {prefix}
        {formattedTime}
      </div>
    );
  }

  return (
    <div className={className}>
      {prefix}
      {formattedTime}
    </div>
  );
}
