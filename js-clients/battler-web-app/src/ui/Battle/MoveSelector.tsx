import type { CSSProperties } from "react";
import type { MonMoveSlotData } from "battler-types";
import styles from "./ActionPanel.module.scss";

interface MoveSelectorProps {
  activeReq: {
    moves: MonMoveSlotData[];
    trapped?: boolean;
    can_mega_evolve?: boolean;
    can_terastallize?: boolean;
    can_z_move?: boolean;
    can_dynamax?: boolean;
    can_ultra_burst?: boolean;
  };
  isLoading: boolean;
  mega: boolean;
  setMega: (val: boolean) => void;
  tera: boolean;
  setTera: (val: boolean) => void;
  zmove: boolean;
  setZmove: (val: boolean) => void;
  dyna: boolean;
  setDyna: (val: boolean) => void;
  ultra: boolean;
  setUltra: (val: boolean) => void;
  onSelectMove: (move: MonMoveSlotData, index: number) => void;
  onClearError: () => void;
}

export default function MoveSelector({
  activeReq,
  isLoading,
  mega,
  setMega,
  tera,
  setTera,
  zmove,
  setZmove,
  dyna,
  setDyna,
  ultra,
  setUltra,
  onSelectMove,
  onClearError,
}: MoveSelectorProps) {
  const hasModifiers = !!(
    activeReq.can_mega_evolve ||
    activeReq.can_terastallize ||
    activeReq.can_z_move ||
    activeReq.can_dynamax ||
    activeReq.can_ultra_burst
  );

  return (
    <div className="flex-col gap-m">
      <div className={styles.movesColumn}>
        <h4>Select move</h4>

        {hasModifiers && (
          <div className={styles.modifiersRow}>
            {[
              {
                key: "mega",
                label: "Mega",
                flag: activeReq.can_mega_evolve,
                value: mega,
                setter: setMega,
              },
              {
                key: "tera",
                label: "Tera",
                flag: activeReq.can_terastallize,
                value: tera,
                setter: setTera,
              },
              {
                key: "zmove",
                label: "Z-Move",
                flag: activeReq.can_z_move,
                value: zmove,
                setter: setZmove,
              },
              {
                key: "dyna",
                label: "Dynamax",
                flag: activeReq.can_dynamax,
                value: dyna,
                setter: setDyna,
              },
              {
                key: "ultra",
                label: "Ultra",
                flag: activeReq.can_ultra_burst,
                value: ultra,
                setter: setUltra,
              },
            ].map(
              ({ key, label, flag, value, setter }) =>
                flag && (
                  <label key={key} className={styles.checkboxLabel}>
                    <input
                      type="checkbox"
                      checked={value}
                      onChange={(e) => {
                        onClearError();
                        setter(e.target.checked);
                      }}
                    />
                    {label}
                  </label>
                ),
            )}
          </div>
        )}

        <div className={styles.movesGrid}>
          {activeReq.moves.map((move, index) => {
            const isMoveDisabled = move.disabled || move.pp === 0;
            return (
              <button
                key={move.id}
                onClick={() => onSelectMove(move, index)}
                className={`${styles.moveBtn} type-border`}
                style={
                  {
                    "--type-color": `var(--color-type-${move.type.toLowerCase()})`,
                  } as CSSProperties
                }
                disabled={isMoveDisabled || isLoading}
              >
                <span className={styles.moveName}>{move.name}</span>
                <span className={styles.moveMeta}>
                  {move.type} | PP: {move.pp}/{move.max_pp}
                </span>
              </button>
            );
          })}
        </div>
        {activeReq.trapped && (
          <p className={styles.trappedMessage}>Trapped</p>
        )}
      </div>
    </div>
  );
}
