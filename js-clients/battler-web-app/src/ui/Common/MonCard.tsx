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
}: MonCardProps) {
  return (
    <div
      onClick={isClickable ? onClick : undefined}
      className={`${styles.teamSummaryCard} ${active ? styles.summaryActive : ""} ${
        hp === 0 ? styles.summaryFainted : ""
      } ${isClickable ? styles.clickableSummaryCard : ""}`}
    >
      <div className={styles.summaryCardHeader}>
        <span className={styles.summaryMonName}>{name}</span>
        <span className={styles.summaryMonLevel}>L{level}</span>
      </div>

      <div className={styles.summaryCardMetaRow}>
        <div className={styles.summaryCardMeta}>
          {status ? (
            <span className={`status-badge ${status.toLowerCase()}`}>{status}</span>
          ) : hp === 0 ? (
            <span className="badge badge-danger">Fainted</span>
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
