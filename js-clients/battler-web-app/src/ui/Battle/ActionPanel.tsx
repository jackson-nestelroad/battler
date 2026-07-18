import { useState, useEffect } from "react";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { submitChoice } from "../../core/wamp";
import type { Request, MonMoveSlotData, PlayerBattleData } from "battler-types";
import ErrorBanner from "../Common/ErrorBanner";
import MonCard from "../Common/MonCard";

import styles from "./ActionPanel.module.scss";

interface ActionPanelProps {
  battleId: string;
  request: Request | null;
  playerData: PlayerBattleData | null;
  playbackPending: boolean;
  isLoading: boolean;
  errorMessage: string | null;
}

const TARGET_REQUIRING_SELECT = [
  "Normal",
  "AdjacentFoe",
  "AdjacentAlly",
  "Any",
  "AdjacentAllyOrUser",
];

export default function ActionPanel({
  battleId,
  request,
  playerData,
  playbackPending,
  isLoading,
  errorMessage,
}: ActionPanelProps) {
  const dispatch = useAppDispatch();
  const battleSession = useAppSelector((state) => state.battles.battles[battleId]);
  const turn = useAppSelector((state) => state.battles.battles[battleId]?.battleState?.turn || 0);

  // Current state of choice building
  const [currentSlotIndex, setCurrentSlotIndex] = useState(0);
  const [choices, setChoices] = useState<string[]>([]);
  const [selectedMove, setSelectedMove] = useState<MonMoveSlotData | null>(null);
  const [selectedMoveIndex, setSelectedMoveIndex] = useState<number | null>(null);

  // Modifiers
  const [mega, setMega] = useState(false);
  const [zmove, setZmove] = useState(false);
  const [ultra, setUltra] = useState(false);
  const [dyna, setDyna] = useState(false);
  const [tera, setTera] = useState(false);

  // Reset when request or turn changes
  useEffect(() => {
    setCurrentSlotIndex(0);
    setChoices([]);
    setSelectedMove(null);
    setSelectedMoveIndex(null);
    resetModifiers();
  }, [request, turn]);

  const resetModifiers = () => {
    setMega(false);
    setZmove(false);
    setUltra(false);
    setDyna(false);
    setTera(false);
  };

  const handleForfeit = () => {
    if (window.confirm("Are you sure you want to forfeit the match?")) {
      dispatch(submitChoice({ battleId, choice: "forfeit" }));
    }
  };

  // Check if player has already submitted their choice for the current turn
  const isMeReady = !!battleSession?.choiceSubmitted;

  const renderTeamSummary = () => {
    if (!playerData || !playerData.mons) return null;
    return (
      <div className={styles.teamSummarySection}>
        <h4 className={styles.summaryTitle}>Your Team</h4>
        <div className={styles.teamSummaryGrid}>
          {playerData.mons.map((mon, idx) => {
            const name = mon.summary?.name || mon.species;

            // Check if card is clickable for switching
            let isClickable = false;
            let handleClick: (() => void) | undefined = undefined;

            if (request && !isMeReady && !playbackPending && !isLoading) {
              if (request.type === "switch") {
                const needsSwitch = request.needs_switch || [];
                const activeSwitchSlot = needsSwitch[currentSlotIndex];
                if (activeSwitchSlot !== undefined) {
                  isClickable = !mon.active && mon.hp > 0;
                  if (isClickable) {
                    handleClick = () => {
                      const newChoices = [...choices, `switch ${mon.player_team_position}`];
                      if (currentSlotIndex + 1 < needsSwitch.length) {
                        setChoices(newChoices);
                        setCurrentSlotIndex(currentSlotIndex + 1);
                      } else {
                        dispatch(submitChoice({ battleId, choice: newChoices.join("; ") }));
                      }
                    };
                  }
                }
              } else if (request.type === "turn" && selectedMove === null) {
                const activeRequests = request.active || [];
                const activeReq = activeRequests[currentSlotIndex];
                if (activeReq && !activeReq.trapped) {
                  isClickable = !mon.active && mon.hp > 0;
                  if (isClickable) {
                    handleClick = () => {
                      const newChoices = [...choices, `switch ${mon.player_team_position}`];
                      if (currentSlotIndex + 1 < activeRequests.length) {
                        setChoices(newChoices);
                        setCurrentSlotIndex(currentSlotIndex + 1);
                        setSelectedMove(null);
                        setSelectedMoveIndex(null);
                        resetModifiers();
                      } else {
                        dispatch(submitChoice({ battleId, choice: newChoices.join("; ") }));
                      }
                    };
                  }
                }
              }
            }

            return (
              <MonCard
                key={idx}
                name={name}
                level={mon.summary?.level || 50}
                hp={mon.hp}
                maxHp={mon.max_hp}
                status={mon.status}
                active={!!mon.active}
                isClickable={isClickable}
                onClick={handleClick}
              />
            );
          })}
        </div>
      </div>
    );
  };

  const renderChoiceBody = () => {
    if (!request || isMeReady) {
      return (
        <div className={`${styles.panelPlaceholder} ${styles.reset}`}>
          <p>Waiting for opponent's choice or server turn resolution...</p>
        </div>
      );
    }

    if (playbackPending) {
      return (
        <div className={`${styles.panelPlaceholder} ${styles.reset}`}>
          <div className={styles.playbackLoading}>
            <div className={styles.dotPulse} />
            <p>Processing turn logs playback... controls are locked.</p>
          </div>
        </div>
      );
    }

    if (request.type === "team") {
      const handleTeamPreviewSubmit = () => {
        dispatch(submitChoice({ battleId, choice: "team 0 1 2 3 4 5" }));
      };

      return (
        <div className="flex-col gap-m">
          <h3>Team Preview Phase</h3>
          <p>Confirm your team order to begin the match.</p>
          <div className={styles.actionsRow}>
            <button
              className="btn btn-primary"
              onClick={handleTeamPreviewSubmit}
              disabled={isLoading}
            >
              Submit Team Order (Default)
            </button>
            <button className="btn btn-danger" onClick={handleForfeit} disabled={isLoading}>
              Forfeit
            </button>
          </div>
          <ErrorBanner message={errorMessage} />
        </div>
      );
    }

    if (request.type === "switch") {
      const needsSwitch = request.needs_switch || [];
      const activeSwitchSlot = needsSwitch[currentSlotIndex];

      if (activeSwitchSlot === undefined) {
        return (
          <div className={`${styles.panelPlaceholder} ${styles.reset}`}>
            <p>Submitting choices...</p>
          </div>
        );
      }

      const monToReplace = playerData?.mons?.find(
        (m) => m.player_active_position === activeSwitchSlot,
      );
      const replaceMonName =
        monToReplace?.summary?.name || monToReplace?.species || `Slot ${activeSwitchSlot + 1}`;

      return (
        <div className="flex-col gap-m">
          <h3>
            Force Switch Required for <strong>{replaceMonName}</strong> (Slot {activeSwitchSlot + 1}
            )
          </h3>
          <p>One of your Pokémon fainted. Select a replacement from your team below.</p>

          <div className={styles.actionsRow}>
            {currentSlotIndex > 0 && (
              <button
                onClick={() => {
                  setChoices(choices.slice(0, -1));
                  setCurrentSlotIndex(currentSlotIndex - 1);
                }}
                className="btn btn-secondary"
                disabled={isLoading}
              >
                Back
              </button>
            )}
            <button className="btn btn-danger" onClick={handleForfeit} disabled={isLoading}>
              Forfeit
            </button>
          </div>
          <ErrorBanner message={errorMessage} />
        </div>
      );
    }

    if (request.type === "turn") {
      const activeRequests = request.active || [];
      const activeReq = activeRequests[currentSlotIndex];

      if (!activeReq) {
        return (
          <div className={`${styles.panelPlaceholder} ${styles.reset}`}>
            <p>Submitting choices...</p>
          </div>
        );
      }

      const activeMon = playerData?.mons?.find(
        (m) => m.player_team_position === activeReq.team_position,
      );
      const activeMonName =
        activeMon?.summary?.name || activeMon?.species || `Pokémon #${currentSlotIndex + 1}`;

      const handleSelectMove = (move: MonMoveSlotData, index: number) => {
        setSelectedMove(move);
        setSelectedMoveIndex(index);
      };

      const handleConfirmMove = (targetVal: number | null) => {
        if (selectedMoveIndex === null) return;

        let moveStr = `move ${selectedMoveIndex}`;
        if (targetVal !== null) {
          moveStr += `,${targetVal}`;
        }

        if (mega) moveStr += ",mega";
        if (zmove) moveStr += ",zmove";
        if (ultra) moveStr += ",ultra";
        if (dyna) moveStr += ",dyna";
        if (tera) moveStr += ",tera";

        const newChoices = [...choices, moveStr];
        submitOrNextSlot(newChoices);
      };

      const submitOrNextSlot = (nextChoices: string[]) => {
        if (currentSlotIndex + 1 < activeRequests.length) {
          setChoices(nextChoices);
          setCurrentSlotIndex(currentSlotIndex + 1);
          setSelectedMove(null);
          setSelectedMoveIndex(null);
          resetModifiers();
        } else {
          dispatch(submitChoice({ battleId, choice: nextChoices.join("; ") }));
        }
      };

      const handleBack = () => {
        if (selectedMove) {
          setSelectedMove(null);
          setSelectedMoveIndex(null);
        } else if (currentSlotIndex > 0) {
          setChoices(choices.slice(0, -1));
          setCurrentSlotIndex(currentSlotIndex - 1);
          resetModifiers();
        }
      };

      return (
        <div className="flex-col gap-m">
          <div className="card-header">
            <h3>
              Commands for <strong>{activeMonName}</strong> (Slot {currentSlotIndex + 1}/
              {activeRequests.length})
            </h3>
            <div className={styles.headerActions}>
              <button className="btn btn-danger" onClick={handleForfeit} disabled={isLoading}>
                Forfeit
              </button>
            </div>
          </div>

          {selectedMove === null ? (
            <div className="flex-col gap-m">
              <div className={styles.movesColumn}>
                <h4>Select Move (Or click a team member below to Switch)</h4>

                <div className={styles.modifiersRow}>
                  {[
                    {
                      key: "mega",
                      label: "Mega Evolve",
                      flag: activeReq.can_mega_evolve,
                      value: mega,
                      setter: setMega,
                    },
                    {
                      key: "tera",
                      label: "Terastallize",
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
                      label: "Ultra Burst",
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
                            onChange={(e) => setter(e.target.checked)}
                          />
                          {label}
                        </label>
                      ),
                  )}
                </div>

                <div className={styles.movesGrid}>
                  {activeReq.moves.map((move, index) => {
                    const isMoveDisabled = move.disabled || move.pp === 0;
                    return (
                      <button
                        key={move.id}
                        onClick={() => handleSelectMove(move, index)}
                        className={`${styles.moveBtn} type-border`}
                        style={
                          {
                            "--type-color": `var(--color-type-${move.type.toLowerCase()})`,
                          } as React.CSSProperties
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
                  <p className={styles.trappedMessage}>Trapped! Cannot switch out.</p>
                )}
              </div>
            </div>
          ) : (
            <div className={styles.targetCard}>
              <h4>Select Target for {selectedMove.name}</h4>
              <p className={styles.targetDesc}>Target criteria: {selectedMove.target}</p>

              <div className={styles.targetButtons}>
                {!TARGET_REQUIRING_SELECT.includes(selectedMove.target) ? (
                  <button
                    onClick={() => handleConfirmMove(null)}
                    className="btn btn-primary w-full"
                    disabled={isLoading}
                  >
                    Confirm (Auto-aim target)
                  </button>
                ) : (
                  <div className={styles.targetGrid}>
                    <button
                      onClick={() => handleConfirmMove(1)}
                      className="btn btn-secondary"
                      disabled={isLoading}
                    >
                      Opponent Left (Position 0)
                    </button>
                    <button
                      onClick={() => handleConfirmMove(2)}
                      className="btn btn-secondary"
                      disabled={isLoading}
                    >
                      Opponent Right (Position 1)
                    </button>
                    <button
                      onClick={() => handleConfirmMove(-1)}
                      className="btn btn-secondary"
                      disabled={isLoading}
                    >
                      Adjacent Ally (Position 0)
                    </button>
                  </div>
                )}
              </div>
            </div>
          )}

          <div className={styles.navigationRow}>
            {(currentSlotIndex > 0 || selectedMove !== null) && (
              <button onClick={handleBack} className="btn btn-secondary" disabled={isLoading}>
                Back
              </button>
            )}
          </div>
          <ErrorBanner message={errorMessage} />
        </div>
      );
    }

    return null;
  };

  return (
    <div className="flex-col gap-xl">
      <div className="card">{renderChoiceBody()}</div>
      {renderTeamSummary()}
    </div>
  );
}
