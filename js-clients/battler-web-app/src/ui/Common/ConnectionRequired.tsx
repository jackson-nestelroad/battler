import React from "react";
import { useAppSelector } from "../../store/store";
import ConnectForm from "./ConnectForm";
import { useConnectionCountdown } from "../../hooks/useConnectionCountdown";
import styles from "./ConnectionRequired.module.scss";

interface ConnectionRequiredProps {
  children: React.ReactNode;
  bypass?: boolean;
}

export default function ConnectionRequired({
  children,
  bypass = false,
}: ConnectionRequiredProps) {
  const connection = useAppSelector((state) => state.connection);
  const { status, connectionMessage } = useConnectionCountdown();

  if (bypass) {
    return <>{children}</>;
  }

  const isDisconnected =
    connection.status === "disconnected" ||
    (connection.status === "connecting" && !connection.playerId);

  if (isDisconnected) {
    return <ConnectForm />;
  }

  const isReconnecting = status === "connecting";

  return (
    <div className={styles.wrapper}>
      {children}
      {isReconnecting && (
        <div className={styles.modalOverlay}>
          <div className={styles.modalCard}>
            <div className="spinner" />
            <h3>Offline</h3>
            <p>{connectionMessage}</p>
          </div>
        </div>
      )}
    </div>
  );
}

