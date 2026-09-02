import type { Battle } from "battler-service-client";
import type { BattleState } from "battler-state";
import type { ActiveTimerState } from "../../store/battlesSlice";
import { getBattleStateLabel } from "../../utils/battleState";
import BattleTimers from "./BattleTimers";
import styles from "./Field.module.scss";

interface FieldProps {
  battleState: BattleState | null;
  activeTimers?: Record<string, ActiveTimerState>;
  playerId?: string;
  serviceBattle?: Battle | null;
  isReplay?: boolean;
}

export default function Field({
  battleState,
  activeTimers,
  playerId,
  serviceBattle,
  isReplay = false,
}: FieldProps) {
  if (!battleState) {
    return (
      <div className={styles.arena}>
        <div className={styles.battleground}>
          <div className={styles.placeholderText}>
            <p>None</p>
          </div>
        </div>
      </div>
    );
  }

  const weather = battleState.field?.weather || "Clear";
  const terrainKeys = Object.keys(battleState.field?.conditions || {});
  const terrain = terrainKeys.find((name) => name.endsWith("Terrain")) || "None";

  return (
    <div className={styles.arena}>
      <div className={styles.fieldHeader}>
        <div className={styles.fieldConditions}>
          <span className="badge badge-warning">Weather: {weather}</span>
          <span className="badge badge-info">Terrain: {terrain}</span>
          {activeTimers && (
            <BattleTimers
              activeTimers={activeTimers}
              playerId={playerId}
              battleState={battleState}
              serviceBattle={serviceBattle}
              isReplay={isReplay}
            />
          )}
        </div>
        <span className={styles.turnLabel}>
          {getBattleStateLabel({ phase: battleState.phase, turn: battleState.turn })}
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
