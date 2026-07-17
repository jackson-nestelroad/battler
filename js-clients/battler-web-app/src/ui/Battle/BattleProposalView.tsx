import { useAppDispatch } from "../../store/store";
import { respondToProposal } from "../../core/wamp";
import { switchActiveBattle } from "../../store/battlesSlice";
import { removeProposal } from "../../store/proposalsSlice";
import type { ProposedBattleWithDetails } from "../../store/proposalsSlice";
import { setConnectionError } from "../../store/connectionSlice";
import type { ConnectionState } from "../../store/connectionSlice";
import ErrorBanner from "../Common/ErrorBanner";
import BattleSidesList from "../Common/BattleSidesList";

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

  const isPlayer2 = activeProposal.sides[1]?.players[0]?.id === connection.playerId;
  const player2Status = activeProposal.sides[1]?.players[0]?.status;
  const hasPlayer2Accepted = player2Status === "accepted";
  const isDeclined = !!activeProposal.rejection || !!activeProposal.deletionReason;

  const handleAccept = () => {
    dispatch(respondToProposal({ proposedBattleId: battleId, accept: true }))
      .unwrap()
      .catch((err) => {
        dispatch(setConnectionError("Failed to accept challenge: " + (err.message || err), err));
      });
  };

  const handleDecline = () => {
    dispatch(respondToProposal({ proposedBattleId: battleId, accept: false }))
      .unwrap()
      .catch((err) => {
        dispatch(setConnectionError("Failed to decline challenge: " + (err.message || err), err));
      });
  };

  const handleDismiss = () => {
    dispatch(removeProposal(battleId));
    dispatch(switchActiveBattle(null));
  };

  const secs = activeProposal.deadline.secs_since_epoch;
  const deadlineDate = new Date(secs * 1000);

  return (
    <div className={styles.proposalStatusContainer}>
      <ErrorBanner 
        message={connection.error} 
        onClear={() => dispatch(setConnectionError(null))} 
      />
      <div className="card">
        <div className={styles.proposalHeader}>
          <h3>Battle Challenge Proposal</h3>
          <span className={styles.proposalFormat}>
            Format: {activeProposal.battle_options?.format?.battle_type}
          </span>
          <span className={styles.proposalTimer}>
            Expires: {deadlineDate.toLocaleTimeString()}
          </span>
        </div>

        <BattleSidesList sides={activeProposal.sides} isProposal={true} />

        {isDeclined && (
          <ErrorBanner 
            message={`Proposal failed. Reason: ${activeProposal.deletionReason || "unknown reason"}`} 
          />
        )}

        <div className={styles.actionRow}>
          {isDeclined ? (
            <button onClick={handleDismiss} className="btn btn-primary">
              Dismiss & Return to Lobby
            </button>
          ) : (
            <>
              {isPlayer2 && !hasPlayer2Accepted && (
                <div className={styles.actionButtons}>
                  <button 
                    onClick={handleAccept} 
                    className="btn btn-success flex-1"
                  >
                    Accept Challenge
                  </button>
                  <button onClick={handleDecline} className="btn btn-danger">
                    Decline
                  </button>
                </div>
              )}
              
              {(!isPlayer2 || hasPlayer2Accepted) && (
                <div className={styles.waitingState}>
                <p>Waiting for opponent...</p>
                  <button 
                    onClick={() => dispatch(switchActiveBattle(null))} 
                    className="btn btn-primary"
                  >
                    Back to Matchmaking Lobby
                  </button>
                </div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
