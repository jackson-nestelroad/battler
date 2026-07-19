import React from "react";
import { useAppSelector } from "../../store/store";
import ConnectForm from "./ConnectForm";
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

  if (bypass) {
    return <>{children}</>;
  }

  const isDisconnected =
    connection.status === "disconnected" ||
    (connection.status === "connecting" && !connection.playerId);

  if (isDisconnected) {
    return <ConnectForm />;
  }

  const isReconnecting = connection.status === "connecting";

  return (
    <div className={styles.wrapper}>
      {children}
      {isReconnecting && (
        <div className={styles.modalOverlay}>
          <div className={styles.modalCard}>
            <div className="spinner" />
            <h3>Offline</h3>
            <p>Reconnecting...</p>
          </div>
        </div>
      )}
    </div>
  );
}

