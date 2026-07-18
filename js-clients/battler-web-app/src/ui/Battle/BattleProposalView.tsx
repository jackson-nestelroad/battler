import { useState } from "react";
import { useAppDispatch } from "../../store/store";
import { respondToProposal, refreshProposalSession } from "../../core/wamp";
import { selectBattle } from "../../store/battlesSlice";
import { removeProposal } from "../../store/proposalsSlice";
import type { ProposedBattleWithDetails } from "../../store/proposalsSlice";
import { setConnectionError } from "../../store/connectionSlice";
import type { ConnectionState } from "../../store/connectionSlice";
import ErrorBanner from "../Common/ErrorBanner";
import BattleSidesList from "../Common/BattleSidesList";
import { getBattleTitle } from "../../utils/battle";

import styles from "./BattleScreen.module.scss";

interface BattleProposalViewProps {
  battleId: string;
  activeProposal: ProposedBattleWithDetails;
  connection: ConnectionState;
}

export default function BattleProposalView({
  battleId,
  activeProposal,
  connection,
}: BattleProposalViewProps) {
  const dispatch = useAppDispatch();
  const [isRefreshing, setIsRefreshing] = useState(false);

  const title = getBattleTitle(null, null, activeProposal);

  const handleRefresh = async () => {
    if (!connection.playerId) return;
    setIsRefreshing(true);
    try {
      await dispatch(refreshProposalSession({ battleId, playerId: connection.playerId })).unwrap();
    } catch (err) {
      console.error(err);
    } finally {
      setIsRefreshing(false);
    }
  };

  const isPlayer2 = activeProposal.sides[1]?.players[0]?.id === connection.playerId;
  const player2Status = activeProposal.sides[1]?.players[0]?.status;
  const hasPlayer2Accepted = player2Status === "accepted";
  const isDeclined = !!activeProposal.rejection || !!activeProposal.deletionReason;

  const handleAccept = () => {
    dispatch(respondToProposal({ proposedBattleId: battleId, accept: true }))
      .unwrap()
      .catch((err) => {
        dispatch(setConnectionError("Failed to accept proposal: " + (err.message || err), err));
      });
  };

  const handleDecline = () => {
    dispatch(respondToProposal({ proposedBattleId: battleId, accept: false }))
      .unwrap()
      .catch((err) => {
        dispatch(setConnectionError("Failed to decline proposal: " + (err.message || err), err));
      });
  };

  const handleDismiss = () => {
    dispatch(removeProposal(battleId));
    dispatch(selectBattle({ view: "lobby", battleId: null }));
  };

  const secs = activeProposal.deadline.secs_since_epoch;
  const deadlineDate = new Date(secs * 1000);

  return (
    <div className={styles.proposalStatusContainer}>
      <ErrorBanner message={connection.error} onClear={() => dispatch(setConnectionError(null))} />
      <div className="card">
        <div
          className={`${styles.proposalHeader} flex-row justify-between align-start gap-m w-full`}
        >
          <div className="flex-col">
            <h3>{title}</h3>
            <div>
              <span className={styles.proposalFormat}>Battle Proposal</span>
              <span className={styles.proposalTimer}>
                Expires: {deadlineDate.toLocaleTimeString()}
              </span>
            </div>
          </div>
          <button
            onClick={handleRefresh}
            className="btn btn-secondary flex-row align-center gap-xs btn-sm"
            disabled={isRefreshing}
            title="Refresh Proposal Details"
          >
            <span className={isRefreshing ? "spin-icon" : ""}>↻</span> Refresh
          </button>
        </div>

        <BattleSidesList sides={activeProposal.sides} isProposal={true} />

        {isDeclined && (
          <ErrorBanner message={`Failed: ${activeProposal.deletionReason || "unknown reason"}`} />
        )}

        <div className={`${styles.actionRow} flex-col gap-m`}>
          {isDeclined ? (
            <button onClick={handleDismiss} className="btn btn-primary">
              Dismiss
            </button>
          ) : (
            <>
              {isPlayer2 && !hasPlayer2Accepted && (
                <div className="flex-row gap-s">
                  <button onClick={handleAccept} className="btn btn-success flex-1">
                    Accept
                  </button>
                  <button onClick={handleDecline} className="btn btn-danger">
                    Reject
                  </button>
                </div>
              )}

              {(!isPlayer2 || hasPlayer2Accepted) && (
                <div className={`${styles.waitingState} flex-col align-center gap-m`}>
                  <p>Waiting...</p>
                  <div className={styles.waitingActions}>
                    <button
                      onClick={() => dispatch(selectBattle({ view: "lobby", battleId: null }))}
                      className="btn btn-primary"
                    >
                      ← Lobby
                    </button>
                    <button onClick={handleDecline} className="btn btn-danger">
                      Cancel
                    </button>
                  </div>
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
