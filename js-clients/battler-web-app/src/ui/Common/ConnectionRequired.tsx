import React, { useEffect, useState } from "react";
import { useConnectionCountdown } from "../../hooks/useConnectionCountdown";
import { useAppSelector } from "../../store/store";
import ConnectForm from "./ConnectForm";
import styles from "./ConnectionRequired.module.scss";

interface ConnectionRequiredProps {
  children: React.ReactNode;
  bypass?: boolean;
}

export default function ConnectionRequired({ children, bypass = false }: ConnectionRequiredProps) {
  const connection = useAppSelector((state) => state.connection);
  const { status, connectionMessage } = useConnectionCountdown();

  const isReconnecting = status === "connecting" && connection.hasConnected;
  const [showModal, setShowModal] = useState(false);

  useEffect(() => {
    if (!isReconnecting) {
      setShowModal(false);
      return;
    }
    const timer = setTimeout(() => {
      setShowModal(true);
    }, 200);
    return () => clearTimeout(timer);
  }, [isReconnecting]);

  if (bypass) {
    return <>{children}</>;
  }

  const isDisconnected =
    connection.status === "disconnected" ||
    (connection.status === "connecting" && !connection.hasConnected);

  if (isDisconnected) {
    return <ConnectForm />;
  }

  return (
    <div className={styles.wrapper}>
      {children}
      {showModal && (
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
