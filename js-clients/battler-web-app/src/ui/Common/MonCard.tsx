import HpBar from "./HpBar";
import styles from "./MonCard.module.scss";

interface MonCardProps {
  name: string;
  level: number;
  hp: number;
  maxHp: number;
  status: string | null;
  active: boolean;
  isClickable: boolean;
  onClick?: () => void;
  selectionOrder?: number;
  isActing?: boolean;
  actingBadgeText?: string;
}

export default function MonCard({
  name,
  level,
  hp,
  maxHp,
  status,
  active,
  isClickable,
  onClick,
  selectionOrder,
  isActing,
  actingBadgeText = "ACTING",
}: MonCardProps) {
  return (
    <div
      onClick={isClickable ? onClick : undefined}
      className={`${styles.teamSummaryCard} ${active ? styles.summaryActive : ""} ${
        isActing ? styles.summaryActing : ""
      } ${hp === 0 ? styles.summaryFainted : ""} ${
        isClickable ? styles.clickableSummaryCard : ""
      } ${selectionOrder !== undefined ? styles.selectedCard : ""}`}
    >
      {selectionOrder !== undefined && (
        <div className={styles.selectionBadge}>{selectionOrder}</div>
      )}
      {isActing && <div className={styles.actingBadge}>{actingBadgeText}</div>}
      <div className={styles.summaryCardHeader}>
        <span className={styles.summaryMonName}>{name}</span>
        <span className={styles.summaryMonLevel}>L{level}</span>
      </div>

      <div className={styles.summaryCardMetaRow}>
        <div className={styles.summaryCardMeta}>
          {hp === 0 || status?.toLowerCase() === "fnt" ? (
            <span className="badge badge-danger">Fainted</span>
          ) : status ? (
            <span className={`status-badge ${status.toLowerCase()}`}>{status}</span>
          ) : (
            <span className="badge badge-success">OK</span>
          )}
        </div>
        <span className={styles.summaryHpText}>
          {hp}/{maxHp}
        </span>
      </div>

      <HpBar hp={hp} maxHp={maxHp} />
    </div>
  );
}
