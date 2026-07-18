import React, { useState } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { connectWamp } from "../../core/wamp";
import { setConnectionError } from "../../store/connectionSlice";
import ErrorBanner from "./ErrorBanner";

import styles from "./ConnectForm.module.scss";

export default function ConnectForm() {
  const dispatch = useAppDispatch();
  const connection = useAppSelector((state) => state.connection);

  const [playerName, setPlayerName] = useState(connection.savedPlayerId || "");
  const [serverUrl, setServerUrl] = useState(connection.savedServerUrl || "ws://localhost:8080/ws");
  const [autoconnect, setAutoconnect] = useState(connection.autoconnect);

  const handleConnect = (e: React.FormEvent) => {
    e.preventDefault();
    if (!playerName.trim()) return;
    dispatch(
      connectWamp({
        url: serverUrl,
        playerId: playerName.trim().toLowerCase(),
        autoconnect,
      }),
    );
  };

  const isConnecting = connection.status === "connecting";

  return (
    <div className={styles.connectContainer}>
      <div className={`card ${styles.connectCard}`}>
        <h2>Connect</h2>

        <form onSubmit={handleConnect} className={styles.connectForm}>
          <div className="form-group">
            <label htmlFor="playerName">Player</label>
            <input
              id="playerName"
              type="text"
              value={playerName}
              onChange={(e) => setPlayerName(e.target.value)}
              placeholder="e.g., Red, Ash, Cynthia"
              disabled={isConnecting}
              required
            />
          </div>
          <div className="form-group">
            <label htmlFor="serverUrl">Server</label>
            <input
              id="serverUrl"
              type="text"
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              placeholder="ws://localhost:8080/ws"
              disabled={isConnecting}
              required
            />
          </div>
          <div className="checkbox-group">
            <input
              id="autoconnect"
              type="checkbox"
              checked={autoconnect}
              onChange={(e) => setAutoconnect(e.target.checked)}
              disabled={isConnecting}
            />
            <label htmlFor="autoconnect">Auto-connect</label>
          </div>
          <ErrorBanner
            message={connection.error}
            onClear={() => dispatch(setConnectionError(null))}
          />
          <button type="submit" className="btn btn-primary" disabled={isConnecting}>
            {isConnecting ? "Connecting..." : "Connect"}
          </button>
        </form>
      </div>
    </div>
  );
}
