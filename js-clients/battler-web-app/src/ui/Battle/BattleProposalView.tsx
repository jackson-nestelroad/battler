import { useState, useEffect } from "react";
import { useAppDispatch } from "../../store/store";
import { respondToProposal, refreshProposalSession } from "../../core/wamp";
import { selectBattle } from "../../store/battlesSlice";
import { removeProposal } from "../../store/proposalsSlice";
import type { ProposedBattleWithDetails } from "../../store/proposalsSlice";
import { setConnectionError } from "../../store/connectionSlice";
import type { ConnectionState } from "../../store/connectionSlice";
import ErrorBanner from "../Common/ErrorBanner";
import BattleSidesList from "../Common/BattleSidesList";
import { getBattleTitle, formatDeletionReason } from "../../utils/battle";
import CopyableId from "../Common/CopyableId";
import RefreshButton from "../Common/RefreshButton";
import CountdownTimer from "../Common/CountdownTimer";

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

  useEffect(() => {
    if (title) {
      document.title = `${title} | Proposal`;
    }
    return () => {
      document.title = "Battler";
    };
  }, [title]);

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

  return (
    <div className="page-container">
      <header className={`${styles.screenHeader} flex-row justify-between align-center gap-m`}>
        <div className={`${styles.titleInfo} flex-col gap-xs`}>
          <h2>{title}</h2>
          <span className={styles.battleId}>
            <span className={styles.battleFormat}>Battle Proposal</span> •{" "}
            <CopyableId id={battleId} type="proposal" />
          </span>
        </div>
        <RefreshButton
          onClick={handleRefresh}
          isRefreshing={isRefreshing}
          title="Refresh proposal details"
        />
      </header>

      <ErrorBanner message={connection.error} onClear={() => dispatch(setConnectionError(null))} />

      <div className={styles.proposalCardWrapper}>
        <div className="card">
          <BattleSidesList sides={activeProposal.sides} isProposal={true} />

          {!isDeclined && (
            <div className="flex-row justify-center">
              <CountdownTimer
                deadlineSecs={activeProposal.deadline.secs_since_epoch}
                prefix="Expires: "
                badgeMode={true}
              />
            </div>
          )}

          {isDeclined && (
            <ErrorBanner
              message={`Failed: ${formatDeletionReason(activeProposal.deletionReason)}`}
            />
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
    </div>
  );
}
