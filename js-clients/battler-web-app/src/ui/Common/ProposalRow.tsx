import type { ProposedBattleWithDetails } from "../../store/proposalsSlice";
import { getBattleTitle } from "../../utils/battle";
import styles from "./ProposalRow.module.scss";

interface ProposalRowProps {
  proposal: ProposedBattleWithDetails;
  playerId: string;
  onAccept: (uuid: string) => void;
  onDecline: (uuid: string) => void;
  onDismiss: (uuid: string) => void;
  onView: (uuid: string) => void;
}

export default function ProposalRow({
  proposal,
  playerId,
  onAccept,
  onDecline,
  onDismiss,
  onView,
}: ProposalRowProps) {
  const isPlayer2 = proposal.sides[1]?.players.some((pl) => pl.id === playerId);
  const isPlayer1 = proposal.sides[0]?.players.some((pl) => pl.id === playerId);
  const isDeclined = !!proposal.rejection || !!proposal.deletionReason;
  const title = getBattleTitle(null, null, proposal);

  if (isPlayer2) {
    // Incoming Proposal
    return (
      <div className={styles.proposalItem}>
        <div className={styles.proposalInfo}>
          <span className={styles.proposerName}>{title}</span>
          {isDeclined ? (
            <span className={`${styles.proposalMeta} ${styles.declinedText}`}>
              Failed: {proposal.deletionReason || "declined"}
            </span>
          ) : (
            <span className={styles.proposalMeta}>Incoming • Waiting...</span>
          )}
        </div>
        <div className={styles.proposalActions}>
          {isDeclined ? (
            <button onClick={() => onDismiss(proposal.uuid)} className="btn btn-secondary">
              Dismiss
            </button>
          ) : (
            <>
              <button onClick={() => onView(proposal.uuid)} className="btn btn-primary">
                View
              </button>
              <button onClick={() => onAccept(proposal.uuid)} className="btn btn-success">
                Accept
              </button>
              <button onClick={() => onDecline(proposal.uuid)} className="btn btn-danger">
                Reject
              </button>
            </>
          )}
        </div>
      </div>
    );
  }

  if (isPlayer1) {
    // Sent Proposal
    return (
      <div className={styles.proposalItem}>
        <div className={styles.proposalInfo}>
          <span className={styles.proposerName}>{title}</span>
          {isDeclined ? (
            <span className={`${styles.proposalMeta} ${styles.declinedText}`}>
              Failed: {proposal.deletionReason || "declined"}
            </span>
          ) : (
            <span className={styles.proposalMeta}>Waiting...</span>
          )}
        </div>
        <div className={styles.proposalActions}>
          {isDeclined ? (
            <button onClick={() => onDismiss(proposal.uuid)} className="btn btn-secondary">
              Dismiss
            </button>
          ) : (
            <>
              <button onClick={() => onView(proposal.uuid)} className="btn btn-primary">
                View
              </button>
              <button onClick={() => onDecline(proposal.uuid)} className="btn btn-danger">
                Cancel
              </button>
            </>
          )}
        </div>
      </div>
    );
  }

  return null;
}
