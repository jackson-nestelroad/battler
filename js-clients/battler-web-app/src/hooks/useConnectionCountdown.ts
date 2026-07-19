import { useState, useEffect } from "react";
import { useAppSelector } from "../store/store";

export function useConnectionCountdown() {
  const connection = useAppSelector((state) => state.connection);
  const { retryDelay, retryCount } = connection;
  const [secondsRemaining, setSecondsRemaining] = useState<number | null>(null);

  useEffect(() => {
    if (retryDelay !== null) {
      setSecondsRemaining(Math.round(retryDelay));
    } else {
      setSecondsRemaining(null);
    }
  }, [retryDelay, retryCount]);

  useEffect(() => {
    if (secondsRemaining === null || secondsRemaining <= 0) return;
    const interval = setInterval(() => {
      setSecondsRemaining((prev) => (prev !== null && prev > 0 ? prev - 1 : 0));
    }, 1000);
    return () => clearInterval(interval);
  }, [secondsRemaining]);

  let connectionMessage = "Connecting...";
  if (connection.status === "connecting" && secondsRemaining !== null) {
    if (secondsRemaining > 0) {
      connectionMessage = `Retrying in ${secondsRemaining}s...${
        retryCount !== null ? ` (Attempt ${retryCount})` : ""
      }`;
    } else {
      connectionMessage = "Retrying now...";
    }
  }

  return {
    status: connection.status,
    secondsRemaining,
    retryCount,
    connectionMessage,
  };
}
