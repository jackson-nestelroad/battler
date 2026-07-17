import React, { useState } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { connectWamp, proposeBattle, respondToProposal } from "../../core/wamp";
import { switchActiveBattle } from "../../store/battlesSlice";
import { removeProposal } from "../../store/proposalsSlice";
import type { CoreBattleOptions } from "battler-types";

import { setConnectionError } from "../../store/connectionSlice";
import ErrorBanner from "../Common/ErrorBanner";
import ProposalList from "./ProposalList";

import styles from "./Lobby.module.scss";

export default function Lobby() {
  const dispatch = useAppDispatch();

  const connection = useAppSelector((state) => state.connection);
  const proposalsMap = useAppSelector((state) => state.proposals.proposals);
  const proposals = Object.values(proposalsMap);

  // Connection form state
  const [playerName, setPlayerName] = useState(connection.savedPlayerId || "");
  const [serverUrl, setServerUrl] = useState(connection.savedServerUrl || "ws://localhost:8080/ws");
  const [autoconnect, setAutoconnect] = useState(connection.autoconnect);

  // Challenge form state
  const [opponentName, setOpponentName] = useState("");
  const [format, setFormat] = useState<"Singles" | "Doubles">("Singles");

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

  const handleSendChallenge = (e: React.FormEvent) => {
    e.preventDefault();
    if (!opponentName.trim()) return;

    const opponentLower = opponentName.trim().toLowerCase();

    // Core battle options payload
    const battleOptions = {
      seed: 0n,
      format: {
        battle_type: format,
        rules: [],
      },
      field: {
        weather: null,
        terrain: null,
        environment: "Grass",
        time: "Day",
      },
      side_1: {
        name: connection.playerId || "Challenger",
        players: [
          {
            id: connection.playerId || "",
            name: connection.playerId || "",
            team: { members: [], bag: { items: {} } },
          },
        ],
      },
      side_2: {
        name: opponentLower,
        players: [
          {
            id: opponentLower,
            name: opponentLower,
            team: { members: [], bag: { items: {} } },
          },
        ],
      },
    };

    const proposedOptions = {
      battle_options: battleOptions as unknown as CoreBattleOptions,
      service_options: {
        creator: connection.playerId || "",
        timers: { battle: null, player: null, action: null },
      },
      timeout: { secs: 60, nanos: 0 },
    };

    dispatch(proposeBattle(proposedOptions))
      .unwrap()
      .then((proposal) => {
        setOpponentName("");
        dispatch(switchActiveBattle(proposal.uuid));
      })
      .catch((err) => {
        dispatch(setConnectionError("Failed to send challenge: " + (err.message || err), err));
      });
  };

  const handleAcceptProposal = (uuid: string) => {
    dispatch(respondToProposal({ proposedBattleId: uuid, accept: true }))
      .unwrap()
      .catch((err) => {
        dispatch(setConnectionError("Failed to accept challenge: " + (err.message || err), err));
      });
    dispatch(switchActiveBattle(uuid));
  };

  const handleDeclineProposal = (uuid: string) => {
    dispatch(respondToProposal({ proposedBattleId: uuid, accept: false }))
      .unwrap()
      .catch((err) => {
        dispatch(setConnectionError("Failed to decline challenge: " + (err.message || err), err));
      });
  };

  if (connection.status !== "connected") {
    const isConnecting = connection.status === "connecting";
    return (
      <div className={styles.lobbyContainer}>
        <div className={`card ${styles.connectCard}`}>
          <h2>Connect to Battle Server</h2>
          <p className={styles.subtitle}>
            Enter your profile name and server address to join the matchmaking lobby.
          </p>

          <form onSubmit={handleConnect} className={styles.connectForm}>
            <div className="form-group">
              <label htmlFor="playerName">Player Name</label>
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
              <label htmlFor="serverUrl">Server URL</label>
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
              <label htmlFor="autoconnect">Auto-connect on next visit</label>
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

  // Split proposals into incoming challenges and outgoing challenges
  const incomingChallenges = proposals.filter((p) => {
    const isPlayer2 = p.sides[1]?.players[0]?.id === connection.playerId;
    const isResolved = !!p.battle;
    const isDeclined = !!p.rejection || !!p.deletionReason;
    return isPlayer2 && !isResolved && !isDeclined;
  });

  const outgoingChallenges = proposals.filter((p) => {
    const isPlayer1 = p.sides[0]?.players[0]?.id === connection.playerId;
    const isResolved = !!p.battle;
    return isPlayer1 && !isResolved;
  });

  return (
    <div className="page-container scroll-y">
      <div className="dashboard-header">
        <h1>Matchmaking Lobby</h1>
        <p>Propose direct challenges or respond to pending invitations from other trainers.</p>
      </div>

      <ErrorBanner message={connection.error} onClear={() => dispatch(setConnectionError(null))} />

      {/* Propose Challenge Form */}
      <section className="card">
        <div className="card-header">
          <h3>Send Direct Challenge</h3>
        </div>
        <form onSubmit={handleSendChallenge} className={`${styles.challengeForm} flex-col gap-m`}>
          <div className={`${styles.formFields} flex-row gap-m`}>
            <div className={`form-group ${styles.challengeField}`}>
              <label htmlFor="opponentName">Opponent Username</label>
              <input
                id="opponentName"
                type="text"
                value={opponentName}
                onChange={(e) => setOpponentName(e.target.value)}
                placeholder="Enter exact trainer name"
                required
              />
            </div>

            <div className={`form-group ${styles.formatField}`}>
              <label htmlFor="format">Battle Format</label>
              <select
                id="format"
                value={format}
                onChange={(e) => setFormat(e.target.value as "Singles" | "Doubles")}
              >
                <option value="Singles">Singles (1v1 Active)</option>
                <option value="Doubles">Doubles (2v2 Active)</option>
              </select>
            </div>
          </div>

          <div className={styles.formActions}>
            <button type="submit" className="btn btn-primary" disabled={!opponentName.trim()}>
              Challenge Trainer
            </button>
          </div>
        </form>
      </section>

      <div className={styles.dashboardGrid}>
        {/* Incoming Challenges */}
        <ProposalList
          title="Incoming Challenges"
          proposals={incomingChallenges}
          playerId={connection.playerId || ""}
          emptyText="No pending incoming challenges"
          onAccept={handleAcceptProposal}
          onDecline={handleDeclineProposal}
          onDismiss={(uuid) => dispatch(removeProposal(uuid))}
        />

        {/* Outgoing Challenges */}
        <ProposalList
          title="Your Sent Challenges"
          proposals={outgoingChallenges}
          playerId={connection.playerId || ""}
          emptyText="No active sent challenges"
          onAccept={handleAcceptProposal}
          onDecline={handleDeclineProposal}
          onDismiss={(uuid) => dispatch(removeProposal(uuid))}
        />
      </div>
    </div>
  );
}
