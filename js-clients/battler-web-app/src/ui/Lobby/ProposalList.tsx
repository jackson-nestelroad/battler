import type { ProposedBattleWithDetails } from "../../store/proposalsSlice";
import ProposalRow from "../Common/ProposalRow";
import styles from "./Lobby.module.scss";

interface ProposalListProps {
  title: string;
  proposals: ProposedBattleWithDetails[];
  playerId: string;
  emptyText: string;
  onAccept: (uuid: string) => void;
  onDecline: (uuid: string) => void;
  onDismiss: (uuid: string) => void;
}

export default function ProposalList({
  title,
  proposals,
  playerId,
  emptyText,
  onAccept,
  onDecline,
  onDismiss,
}: ProposalListProps) {
  return (
    <section className="card">
      <div className="card-header">
        <h3>{title}</h3>
      </div>
      {proposals.length === 0 ? (
        <p className={styles.emptyText}>{emptyText}</p>
      ) : (
        <div className={`${styles.proposalsList} flex-col gap-s`}>
          {proposals.map((p) => (
            <ProposalRow
              key={p.uuid}
              proposal={p}
              playerId={playerId}
              onAccept={onAccept}
              onDecline={onDecline}
              onDismiss={onDismiss}
            />
          ))}
        </div>
      )}
    </section>
  );
}
