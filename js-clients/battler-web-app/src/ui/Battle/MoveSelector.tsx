import type { MonMoveSlotData } from "battler-types";
import type { CSSProperties } from "react";
import styles from "./ActionPanel.module.scss";

interface MoveSelectorProps {
  activeReq: {
    moves: MonMoveSlotData[];
    z_moves?: (MonMoveSlotData | null)[];
    max_moves?: MonMoveSlotData[];
    trapped?: boolean;
    can_mega_evolve?: boolean;
    can_terastallize?: boolean;
    can_z_move?: boolean;
    can_dynamax?: boolean;
    can_ultra_burst?: boolean;
  };
  isDynamaxed?: boolean;
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
  isDynamaxed = false,
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
  const isMaxMoveActive = dyna || isDynamaxed;

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
                flag: activeReq.can_dynamax && !isDynamaxed,
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
          {activeReq.moves.map((baseMove, index) => {
            let moveToRender: MonMoveSlotData | null = baseMove;
            let badgeText: string | null = null;
            let isZMoveDisabled = false;

            if (zmove) {
              const zMoveData = activeReq.z_moves?.[index];
              if (zMoveData) {
                moveToRender = zMoveData;
                badgeText = "Z-Move";
              } else {
                moveToRender = baseMove;
                isZMoveDisabled = true;
              }
            } else if (isMaxMoveActive) {
              const maxMoveData = activeReq.max_moves?.[index];
              if (maxMoveData) {
                moveToRender = maxMoveData;
                badgeText = "Max Move";
              }
            }

            const isMoveDisabled =
              isZMoveDisabled || baseMove.disabled || baseMove.pp === 0 || moveToRender.disabled;

            return (
              <button
                key={baseMove.id || index}
                onClick={() => onSelectMove(moveToRender, index)}
                className={`${styles.moveBtn} type-border`}
                style={
                  {
                    "--type-color": `var(--color-type-${moveToRender.type.toLowerCase()})`,
                  } as CSSProperties
                }
                disabled={isMoveDisabled || isLoading}
              >
                <div className={styles.moveHeaderRow}>
                  <span className={styles.moveName}>{moveToRender.name}</span>
                  {badgeText && (
                    <span
                      className={`${styles.moveBadge} ${
                        badgeText === "Z-Move" ? styles.zmoveBadge : styles.maxMoveBadge
                      }`}
                    >
                      {badgeText}
                    </span>
                  )}
                </div>
                <span className={styles.moveMeta}>
                  {moveToRender.type} | PP: {baseMove.pp}/{baseMove.max_pp}
                </span>
              </button>
            );
          })}
        </div>
        {activeReq.trapped && <p className={styles.trappedMessage}>Trapped</p>}
      </div>
    </div>
  );
}
