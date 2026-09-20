import { useEffect, useState } from "react";
import { refreshProposalSession, respondToProposal } from "../../core/wamp";
import { selectBattle } from "../../store/battlesSlice";
import type { ConnectionState } from "../../store/connectionSlice";
import { setConnectionError } from "../../store/connectionSlice";
import type { ProposedBattleWithDetails } from "../../store/proposalsSlice";
import { removeProposal } from "../../store/proposalsSlice";
import { useAppDispatch } from "../../store/store";
import { formatDeletionReason, getBattleTitle } from "../../utils/battle";
import BattleDetailsGrid from "../Common/BattleDetailsGrid";
import BattleSidesList from "../Common/BattleSidesList";
import CopyableId from "../Common/CopyableId";
import CountdownTimer from "../Common/CountdownTimer";
import ErrorBanner from "../Common/ErrorBanner";
import RefreshButton from "../Common/RefreshButton";

import styles from "./BattleProposalView.module.scss";

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

  useEffect(() => {
    dispatch(setConnectionError(null));
  }, [battleId, dispatch]);

  const title = getBattleTitle(null, null, activeProposal);
  const isDeclined = !!activeProposal.rejection || !!activeProposal.deletionReason;

  useEffect(() => {
    if (title) {
      document.title = `${title} | Proposal`;
    }
    return () => {
      document.title = "Battler";
    };
  }, [title]);

  const currentPlayer = activeProposal.sides
    .flatMap((s) => s.players)
    .find((p) => p.id === connection.playerId);

  const isParticipant = !!currentPlayer;
  const hasCurrentPlayerAccepted = currentPlayer?.status === "accepted";

  useEffect(() => {
    if (!isParticipant) {
      dispatch(setConnectionError("Spectators cannot view proposed battles"));
      dispatch(selectBattle({ view: "lobby", battleId: null }));
    }
  }, [isParticipant, dispatch]);

  if (!isParticipant) {
    return null;
  }

  const handleAccept = async () => {
    try {
      await dispatch(respondToProposal({ proposedBattleId: battleId, accept: true })).unwrap();
    } catch {
      // Error handled by wamp thunk
    }
  };

  const handleDecline = async () => {
    try {
      await dispatch(respondToProposal({ proposedBattleId: battleId, accept: false })).unwrap();
    } catch {
      // Error handled by wamp thunk
    }
  };

  const handleDismiss = () => {
    dispatch(removeProposal(battleId));
    dispatch(selectBattle({ view: "lobby", battleId: null }));
  };

  const handleRefresh = async () => {
    if (!connection.playerId) return;
    setIsRefreshing(true);
    try {
      await dispatch(
        refreshProposalSession({ battleId, playerId: connection.playerId }),
      ).unwrap();
    } finally {
      setIsRefreshing(false);
    }
  };

  return (
    <div className="page-container">
      <header className="screen-header flex-row justify-between align-center gap-m">
        <div className="screen-header-title flex-col gap-xs">
          <h2>{title}</h2>
          <span className="screen-header-subtitle">
            <span className="screen-header-format">
              {activeProposal.special ? `${activeProposal.special} Proposal` : "Battle Proposal"}
            </span>{" "}
            • <CopyableId id={battleId} type="proposal" />
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

          <div className={styles.detailsSection}>
            <h4 className="details-header">Battle Details</h4>
            <BattleDetailsGrid
              battleType={activeProposal.battle_type}
              rules={activeProposal.rules}
              timers={activeProposal.timers}
              special={activeProposal.special}
            />
          </div>

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
                {isParticipant && !hasCurrentPlayerAccepted && (
                  <div className="flex-row gap-s">
                    <button onClick={handleAccept} className="btn btn-success flex-1">
                      Accept
                    </button>
                    <button onClick={handleDecline} className="btn btn-danger">
                      Reject
                    </button>
                  </div>
                )}

                {(!isParticipant || hasCurrentPlayerAccepted) && (
                  <div className="flex-col align-center gap-m text-center">
                    <p>Waiting...</p>
                    <div className="flex-row flex-mobile-col justify-center gap-s w-full">
                      <button
                        onClick={() => dispatch(selectBattle({ view: "lobby", battleId: null }))}
                        className="btn btn-primary"
                      >
                        ← Lobby
                      </button>
                      {isParticipant && (
                        <button onClick={handleDecline} className="btn btn-danger">
                          Cancel
                        </button>
                      )}
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
