import { useState, useEffect } from "react";

interface CountdownTimerProps {
  deadlineSecs: number;
  onExpiry?: () => void;
  prefix?: string;
  className?: string;
  badgeMode?: boolean;
}

export default function CountdownTimer({
  deadlineSecs,
  onExpiry,
  prefix = "",
  className = "",
  badgeMode = false,
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

  const formatTimeLeft = (secs: number) => {
    if (secs < 60) {
      return `${secs}s`;
    }
    const minutes = Math.floor(secs / 60);
    const seconds = secs % 60;
    const paddedSeconds = seconds.toString().padStart(2, "0");
    return `${minutes}:${paddedSeconds}`;
  };

  const formattedTime = formatTimeLeft(timeLeft);

  if (badgeMode) {
    const badgeClass = timeLeft < 15 ? "badge-danger" : "badge-warning";
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
