import type { BattleState } from "battler-state";
import styles from "./Field.module.scss";

interface FieldProps {
  battleState: BattleState | null;
}

export default function Field({ battleState }: FieldProps) {
  if (!battleState) {
    return (
      <div className={styles.fieldPlaceholder}>
        <p>No battle.</p>
      </div>
    );
  }

  const weather = battleState.field?.weather || "Clear";
  const terrainKeys = Object.keys(battleState.field?.conditions || {});
  const terrain = terrainKeys.find((name) => name.endsWith("Terrain")) || "None";

  return (
    <div className={styles.arena}>
      <div className={styles.fieldConditions}>
        <span className="badge badge-warning">Weather: {weather}</span>
        <span className="badge badge-info">Terrain: {terrain}</span>
        <span className={styles.turnLabel}>
          {battleState.turn === 0 ? "Preview" : `Turn ${battleState.turn}`}
        </span>
      </div>

      <div className={styles.battleground}>
        <div className={styles.placeholderText}>
          <h4>Arena</h4>
        </div>
      </div>
    </div>
  );
}
