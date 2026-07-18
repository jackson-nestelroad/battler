import React, { useState } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { proposeBattle, respondToProposal, refreshLobby } from "../../core/wamp";
import { selectBattle } from "../../store/battlesSlice";
import { removeProposal, updateProposal } from "../../store/proposalsSlice";
import type { CoreBattleOptions } from "battler-types";

import { setConnectionError } from "../../store/connectionSlice";
import ErrorBanner from "../Common/ErrorBanner";
import ProposalList from "./ProposalList";
import ConnectForm from "../Common/ConnectForm";
import RefreshButton from "../Common/RefreshButton";

import styles from "./Lobby.module.scss";

export default function Lobby() {
  const dispatch = useAppDispatch();
  const [isRefreshing, setIsRefreshing] = useState(false);

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      await dispatch(refreshLobby(connection.playerId || "")).unwrap();
    } catch (err) {
      console.error(err);
    } finally {
      setIsRefreshing(false);
    }
  };

  const connection = useAppSelector((state) => state.connection);
  const proposalsMap = useAppSelector((state) => state.proposals.proposals);
  const proposals = Object.values(proposalsMap);

  // Proposal form state
  const [opponentName, setOpponentName] = useState("");
  const [format, setFormat] = useState<"Singles" | "Doubles">("Singles");

  const handleSendProposal = (e: React.FormEvent) => {
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
        name: connection.playerId || "Player",
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
        dispatch(updateProposal(proposal));
        dispatch(selectBattle({ view: "proposal", battleId: proposal.uuid }));
      })
      .catch((err) => {
        dispatch(setConnectionError("Failed to send proposal: " + (err.message || err), err));
      });
  };

  const handleAcceptProposal = (uuid: string) => {
    dispatch(respondToProposal({ proposedBattleId: uuid, accept: true }))
      .unwrap()
      .catch((err) => {
        dispatch(setConnectionError("Failed to accept proposal: " + (err.message || err), err));
      });
    dispatch(selectBattle({ view: "proposal", battleId: uuid }));
  };

  const handleDeclineProposal = (uuid: string) => {
    dispatch(respondToProposal({ proposedBattleId: uuid, accept: false }))
      .unwrap()
      .catch((err) => {
        dispatch(setConnectionError("Failed to decline proposal: " + (err.message || err), err));
      });
  };

  if (connection.status !== "connected") {
    return <ConnectForm />;
  }

  // Split proposals into incoming proposals and outgoing proposals
  const incomingProposals = proposals.filter((p) => {
    const isPlayer2 = p.sides[1]?.players[0]?.id === connection.playerId;
    const isResolved = !!p.battle;
    const isDeclined = !!p.rejection || !!p.deletionReason;
    return isPlayer2 && !isResolved && !isDeclined;
  });

  const outgoingProposals = proposals.filter((p) => {
    const isPlayer1 = p.sides[0]?.players[0]?.id === connection.playerId;
    const isResolved = !!p.battle;
    return isPlayer1 && !isResolved;
  });

  return (
    <div className="page-container scroll-y">
      <div className={`dashboard-header ${styles.lobbyHeader}`}>
        <div className="flex-col gap-xs">
          <h1>Lobby</h1>
        </div>
        <RefreshButton onClick={handleRefresh} isRefreshing={isRefreshing} title="Refresh Lobby" />
      </div>

      <ErrorBanner message={connection.error} onClear={() => dispatch(setConnectionError(null))} />

      {/* Propose Battle Form */}
      <section className="card">
        <div className="card-header">
          <h3>New Battle Proposal</h3>
        </div>
        <form onSubmit={handleSendProposal} className={`${styles.proposalForm} flex-col gap-m`}>
          <div className={`${styles.formFields} flex-row gap-m`}>
            <div className={`form-group ${styles.proposalField}`}>
              <label htmlFor="opponentName">Opponent</label>
              <input
                id="opponentName"
                type="text"
                value={opponentName}
                onChange={(e) => setOpponentName(e.target.value)}
                placeholder="Player name"
                required
              />
            </div>

            <div className={`form-group ${styles.formatField}`}>
              <label htmlFor="format">Format</label>
              <select
                id="format"
                value={format}
                onChange={(e) => setFormat(e.target.value as "Singles" | "Doubles")}
              >
                <option value="Singles">Singles</option>
                <option value="Doubles">Doubles</option>
              </select>
            </div>
          </div>

          <div className={styles.formActions}>
            <button type="submit" className="btn btn-primary" disabled={!opponentName.trim()}>
              Propose
            </button>
          </div>
        </form>
      </section>

      <div className={styles.dashboardGrid}>
        {/* Incoming Proposals */}
        <ProposalList
          title="Incoming"
          proposals={incomingProposals}
          playerId={connection.playerId || ""}
          emptyText="None"
          onAccept={handleAcceptProposal}
          onDecline={handleDeclineProposal}
          onDismiss={(uuid) => dispatch(removeProposal(uuid))}
          onView={(uuid) => dispatch(selectBattle({ view: "proposal", battleId: uuid }))}
        />

        {/* Outgoing Proposals */}
        <ProposalList
          title="Sent"
          proposals={outgoingProposals}
          playerId={connection.playerId || ""}
          emptyText="None"
          onAccept={handleAcceptProposal}
          onDecline={handleDeclineProposal}
          onDismiss={(uuid) => dispatch(removeProposal(uuid))}
          onView={(uuid) => dispatch(selectBattle({ view: "proposal", battleId: uuid }))}
        />
      </div>
    </div>
  );
}
