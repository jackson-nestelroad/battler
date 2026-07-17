import type { ProposedBattleWithDetails } from "../../store/proposalsSlice";
import styles from "./ProposalRow.module.scss";

interface ProposalRowProps {
  proposal: ProposedBattleWithDetails;
  playerId: string;
  onAccept: (uuid: string) => void;
  onDecline: (uuid: string) => void;
  onDismiss: (uuid: string) => void;
}

export default function ProposalRow({
  proposal,
  playerId,
  onAccept,
  onDecline,
  onDismiss,
}: ProposalRowProps) {
  const isPlayer2 = proposal.sides[1]?.players[0]?.id === playerId;
  const isPlayer1 = proposal.sides[0]?.players[0]?.id === playerId;

  if (isPlayer2) {
    // Incoming Challenge
    const challenger = proposal.sides[0]?.players[0]?.name || "Unknown Trainer";
    return (
      <div className={styles.proposalItem}>
        <div className={styles.proposalInfo}>
          <span className={styles.challengerName}>@{challenger}</span>
          <span className={styles.challengeMeta}>challenges you to a match</span>
        </div>
        <div className={styles.proposalActions}>
          <button
            onClick={() => onAccept(proposal.uuid)}
            className="btn btn-success"
          >
            Accept
          </button>
          <button
            onClick={() => onDecline(proposal.uuid)}
            className="btn btn-danger"
          >
            Decline
          </button>
        </div>
      </div>
    );
  }

  if (isPlayer1) {
    // Outgoing Challenge
    const opponent = proposal.sides[1]?.players[0]?.name || "Unknown Trainer";
    const isDeclined = !!proposal.rejection || !!proposal.deletionReason;
    return (
      <div className={styles.proposalItem}>
        <div className={styles.proposalInfo}>
          <span className={styles.challengerName}>to @{opponent}</span>
          {isDeclined ? (
            <span className={`${styles.challengeMeta} ${styles.declinedText}`}>
              Declined: {proposal.deletionReason || "Opponent rejected challenge"}
            </span>
          ) : (
            <span className={styles.challengeMeta}>Waiting for acceptance...</span>
          )}
        </div>
        <div className={styles.proposalActions}>
          {isDeclined ? (
            <button
              onClick={() => onDismiss(proposal.uuid)}
              className="btn btn-secondary"
            >
              Dismiss
            </button>
          ) : (
            <button
              onClick={() => onDecline(proposal.uuid)}
              className="btn btn-secondary"
            >
              Cancel
            </button>
          )}
        </div>
      </div>
    );
  }

  return null;
}
