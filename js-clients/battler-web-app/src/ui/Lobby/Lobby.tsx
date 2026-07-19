import { useState } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { respondToProposal, refreshLobby } from "../../core/wamp";
import { selectBattle } from "../../store/battlesSlice";
import { removeProposal } from "../../store/proposalsSlice";

import { setConnectionError } from "../../store/connectionSlice";
import ErrorBanner from "../Common/ErrorBanner";
import ProposalList from "./ProposalList";
import ProposalForm from "./ProposalForm";
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



  // Split proposals into incoming proposals and outgoing proposals supporting multi sides
  const incomingProposals = proposals.filter((p) => {
    const isPlayerOnSide2 = p.sides[1]?.players.some((pl) => pl.id === connection.playerId);
    const isResolved = !!p.battle;
    const isDeclined = !!p.rejection || !!p.deletionReason;
    return isPlayerOnSide2 && !isResolved && !isDeclined;
  });

  const outgoingProposals = proposals.filter((p) => {
    const isPlayerOnSide1 = p.sides[0]?.players.some((pl) => pl.id === connection.playerId);
    const isResolved = !!p.battle;
    return isPlayerOnSide1 && !isResolved;
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
    </div>
  );
}
