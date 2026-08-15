import { useState } from "react";
import { refreshLobby, respondToProposal } from "../../core/wamp";
import { selectBattle } from "../../store/battlesSlice";
import { removeProposal } from "../../store/proposalsSlice";
import { useAppDispatch, useAppSelector } from "../../store/store";

import { setConnectionError } from "../../store/connectionSlice";
import ErrorBanner from "../Common/ErrorBanner";
import RefreshButton from "../Common/RefreshButton";
import BattlesList from "./BattlesList";
import ProposalForm from "./ProposalForm";
import ProposalList from "./ProposalList";

import styles from "./Lobby.module.scss";

export default function Lobby() {
  const dispatch = useAppDispatch();
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [refreshCounter, setRefreshCounter] = useState(0);

  const handleRefresh = async () => {
    setIsRefreshing(true);
    try {
      await dispatch(refreshLobby(connection.playerId || "")).unwrap();
      setRefreshCounter((c) => c + 1);
    } catch (err) {
      console.error(err);
    } finally {
      setIsRefreshing(false);
    }
  };

  const connection = useAppSelector((state) => state.connection);
  const proposalsMap = useAppSelector((state) => state.proposals.proposals);
  const proposals = Object.values(proposalsMap);

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

  // Split proposals into incoming proposals (user is a participant but hasn't accepted yet)
  // and outgoing proposals (user is a participant and has accepted, or proposal is declined)
  const incomingProposals = proposals.filter((p) => {
    const player = p.sides.flatMap((s) => s.players).find((pl) => pl.id === connection.playerId);
    const isResolved = !!p.battle;
    const isDeclined = !!p.rejection || !!p.deletionReason;
    const hasAccepted = player?.status === "accepted";
    return !!player && !isResolved && !isDeclined && !hasAccepted;
  });

  const outgoingProposals = proposals.filter((p) => {
    const player = p.sides.flatMap((s) => s.players).find((pl) => pl.id === connection.playerId);
    const isResolved = !!p.battle;
    const hasAccepted = player?.status === "accepted";
    const isDeclined = !!p.rejection || !!p.deletionReason;
    return !!player && !isResolved && (hasAccepted || isDeclined);
  });

  return (
    <div className="page-container scroll-y">
      <div className={`dashboard-header ${styles.lobbyHeader}`}>
        <div className="flex-col gap-xs">
          <h1>Lobby</h1>
        </div>
        <RefreshButton onClick={handleRefresh} isRefreshing={isRefreshing} title="Refresh lobby" />
      </div>

      <ErrorBanner message={connection.error} onClear={() => dispatch(setConnectionError(null))} />

      {/* Propose Battle Form */}
      <ProposalForm />

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

      {/* All Battles Section */}
      <BattlesList refreshTrigger={refreshCounter} />
    </div>
  );
}
