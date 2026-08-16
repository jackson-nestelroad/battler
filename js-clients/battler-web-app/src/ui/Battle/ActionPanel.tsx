import { stateSelectors } from "battler-state";
import type { MonMoveSlotData, PlayerBattleData, Request } from "battler-types";
import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import { submitChoice } from "../../core/wamp";
import { setChoiceError } from "../../store/battlesSlice";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { ChoiceBuilder } from "../../utils/choiceBuilder";
import { parseChoiceError } from "../../utils/choiceErrorParser";
import { parseChoiceString } from "../../utils/choiceFormatter";
import { canSlotShift, getMonTeamPosition } from "../../utils/monHelpers";
import { getValidTargets, type TargetOption } from "../../utils/targeting";
import ErrorBanner from "../Common/ErrorBanner";
import ChoiceStepper from "./ChoiceStepper";
import MoveSelector from "./MoveSelector";
import TargetSelector from "./TargetSelector";
import TeamSummary from "./TeamSummary";

import styles from "./ActionPanel.module.scss";

interface ActionPanelProps {
  battleId: string;
  request: Request | null;
  playerData: PlayerBattleData | null;
  playbackPending: boolean;
  isLoading: boolean;
  errorMessage: string | null;
}

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
  const [selectedTeamIndices, setSelectedTeamIndices] = useState<number[]>([]);
  const [dynamicTargets, setDynamicTargets] = useState<TargetOption[]>([]);

  // Modifiers
  const [mega, setMega] = useState(false);
  const [zmove, setZmove] = useState(false);
  const [ultra, setUltra] = useState(false);
  const [dyna, setDyna] = useState(false);
  const [tera, setTera] = useState(false);

  const [showForfeitConfirm, setShowForfeitConfirm] = useState(false);

  const resetModifiers = () => {
    setMega(false);
    setZmove(false);
    setUltra(false);
    setDyna(false);
    setTera(false);
  };

  const submittingRef = useRef(false);

  const choiceError = battleSession?.choiceError || null;
  const parsedChoiceError = useMemo(() => parseChoiceError(choiceError), [choiceError]);

  useEffect(() => {
    const failedIdx = parsedChoiceError.failedSlotIndex;
    if (failedIdx !== null && failedIdx < currentSlotIndex) {
      setChoices((prev) => prev.slice(0, failedIdx));
      setCurrentSlotIndex(failedIdx);
      setSelectedMove(null);
      setSelectedMoveIndex(null);
      setDynamicTargets([]);
      resetModifiers();
    }
  }, [parsedChoiceError.failedSlotIndex]); // eslint-disable-line react-hooks/exhaustive-deps

  const handleJumpToSlot = (slotIndex: number) => {
    if (submittingRef.current) return;
    dispatch(setChoiceError({ battleId, error: null }));
    setChoices(choices.slice(0, slotIndex));
    setCurrentSlotIndex(slotIndex);
    setSelectedMove(null);
    setSelectedMoveIndex(null);
    setDynamicTargets([]);
    resetModifiers();
  };

  // Reset when request or turn changes
  useEffect(() => {
    setCurrentSlotIndex(0);
    setChoices([]);
    setSelectedMove(null);
    setSelectedMoveIndex(null);
    setSelectedTeamIndices([]);
    setDynamicTargets([]);
    resetModifiers();
    submittingRef.current = false;
    setShowForfeitConfirm(false);
  }, [request, turn]);

  // Reset submitting ref and forfeit confirm when loading finishes
  useEffect(() => {
    if (!isLoading) {
      submittingRef.current = false;
      setShowForfeitConfirm(false);
    }
  }, [isLoading]);

  // Reset forfeit confirmation after 4 seconds of inactivity
  useEffect(() => {
    if (showForfeitConfirm) {
      const timer = setTimeout(() => {
        setShowForfeitConfirm(false);
      }, 4000);
      return () => clearTimeout(timer);
    }
  }, [showForfeitConfirm]);

  const handleForfeitClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowForfeitConfirm(true);
  };

  const handleForfeitCancel = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowForfeitConfirm(false);
  };

  const handleForfeitConfirm = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (submittingRef.current) return;
    submittingRef.current = true;
    dispatch(submitChoice({ battleId, choice: "forfeit" }));
  };

  const renderForfeitButton = () => {
    if (showForfeitConfirm) {
      return (
        <div className="flex-row gap-xs align-center">
          <button className="btn btn-danger" onClick={handleForfeitConfirm} disabled={isLoading}>
            Confirm
          </button>
          <button className="btn btn-secondary" onClick={handleForfeitCancel} disabled={isLoading}>
            Cancel
          </button>
        </div>
      );
    }

    return (
      <button className="btn btn-danger" onClick={handleForfeitClick} disabled={isLoading}>
        Forfeit
      </button>
    );
  };

  // Check if player has already submitted their choice for the current turn
  const isMeReady = !!battleSession?.choiceSubmitted;

  const battleType =
    battleSession?.serviceBattle?.metadata?.battle_type ||
    battleSession?.metadata?.battle_type ||
    "Singles";

  let activeMonTeamPosition: number | null = null;
  let activeSwitchSlot: number | undefined;
  let monToReplace: MonMoveSlotData | undefined;

  if (request?.type === "turn") {
    const activeReq = request.active?.[currentSlotIndex];
    if (activeReq) {
      activeMonTeamPosition = activeReq.team_position;
    }
  } else if (request?.type === "switch") {
    activeSwitchSlot = request.needs_switch?.[currentSlotIndex];
    if (activeSwitchSlot !== undefined && playerData?.mons) {
      monToReplace = playerData.mons.find(
        (m) => m.player_active_position === activeSwitchSlot,
      );
      if (monToReplace) {
        activeMonTeamPosition = getMonTeamPosition(monToReplace, 0);
      }
    }
  }

  const renderChoiceStepper = () => (
    <ChoiceStepper
      request={request}
      playerData={playerData}
      battleState={battleSession?.battleState}
      choices={choices}
      currentSlotIndex={currentSlotIndex}
      parsedChoiceError={parsedChoiceError}
      isLoading={isLoading}
      onJumpToSlot={handleJumpToSlot}
    />
  );

  // Unified choice progression logic (DRY)
  const advanceSlotOrSubmit = (nextChoices: string[], totalSlots: number) => {
    if (currentSlotIndex + 1 < totalSlots) {
      setChoices(nextChoices);
      setCurrentSlotIndex(currentSlotIndex + 1);
      setSelectedMove(null);
      setSelectedMoveIndex(null);
      resetModifiers();
    } else {
      submittingRef.current = true;
      dispatch(submitChoice({ battleId, choice: nextChoices.join("; ") }));
    }
  };

  const handleSwitch = (playerTeamPosition: number, totalSlots: number) => {
    if (submittingRef.current) return;
    dispatch(setChoiceError({ battleId, error: null }));
    const newChoices = [...choices, `switch ${playerTeamPosition}`];
    advanceSlotOrSubmit(newChoices, totalSlots);
  };

  const handleSelectMon = (idx: number) => {
    if (submittingRef.current) return;
    dispatch(setChoiceError({ battleId, error: null }));
    setSelectedTeamIndices((prev) => {
      const exists = prev.indexOf(idx);
      if (exists !== -1) {
        return prev.filter((i) => i !== idx);
      } else {
        const maxTeamSize = request?.type === "team" ? request.max_team_size : null;
        const targetSize = Math.min(
          playerData?.mons?.length || 0,
          maxTeamSize ?? (playerData?.mons?.length || 0),
        );
        if (prev.length < targetSize) {
          return [...prev, idx];
        }
        return prev;
      }
    });
  };

  const renderTeamSummary = () => {

    return (
      <TeamSummary
        playerData={playerData}
        request={request}
        currentSlotIndex={currentSlotIndex}
        selectedMove={selectedMove}
        isMeReady={isMeReady}
        playbackPending={playbackPending}
        isLoading={isLoading}
        onSwitch={handleSwitch}
        selectedTeamIndices={selectedTeamIndices}
        onSelectMon={handleSelectMon}
        activeMonTeamPosition={activeMonTeamPosition}
        actingBadgeText={request?.type === "switch" ? "SWITCHING" : "ACTING"}
      />
    );
  };

  const renderChoiceBody = () => {
    if (!request || isMeReady) {
      return (
        <div className={`${styles.panelPlaceholder} ${styles.reset}`}>
          <p>Waiting...</p>
        </div>
      );
    }

    if (playbackPending) {
      return (
        <div className={`${styles.panelPlaceholder} ${styles.reset}`}>
          <div className="flex-col align-center gap-m">
            <div className={styles.dotPulse} />
            <p>Playing turn...</p>
          </div>
        </div>
      );
    }

    if (request.type === "team") {
      const targetSize = Math.min(
        playerData?.mons?.length || 0,
        request.max_team_size ?? (playerData?.mons?.length || 0),
      );

      const handleTeamPreviewSubmit = () => {
        if (submittingRef.current) return;
        submittingRef.current = true;
        if (selectedTeamIndices.length > 0) {
          dispatch(submitChoice({ battleId, choice: `team ${selectedTeamIndices.join(" ")}` }));
        } else {
          dispatch(submitChoice({ battleId, choice: "team" }));
        }
      };

      const handleClearSelection = () => {
        setSelectedTeamIndices([]);
      };

      return (
        <div className="flex-col gap-m">
          <div className="card-header">
            <h3>Team Preview</h3>
            <div className={styles.headerActions}>{renderForfeitButton()}</div>
          </div>

          <div className="flex-col gap-s">
            <p className={styles.instructionText}>
              Select your team order. Remaining spots will be filled automatically when confirming.
            </p>
            <span className={styles.selectionProgress}>
              Selected: <strong>{selectedTeamIndices.length}</strong> / {targetSize}
            </span>
          </div>

          <div className="flex-row gap-s align-center">
            <button
              className="btn btn-primary"
              onClick={handleTeamPreviewSubmit}
              disabled={isLoading}
            >
              Confirm team
            </button>
            {selectedTeamIndices.length > 0 && (
              <button
                className="btn btn-secondary"
                onClick={handleClearSelection}
                disabled={isLoading}
              >
                Clear
              </button>
            )}
          </div>
        </div>
      );
    }

    if (request.type === "switch") {
      const needsSwitch = request.needs_switch || [];

      if (activeSwitchSlot === undefined) {
        return (
          <div className={`${styles.panelPlaceholder} ${styles.reset}`}>
            <p>Submitting...</p>
          </div>
        );
      }

      const replaceMonName = monToReplace?.summary?.name || monToReplace?.species;

      const chosenSwitchPositions = choices
        .map((c) => {
          const parsed = parseChoiceString(c);
          return parsed.type === "switch" ? parsed.switchPosition : null;
        })
        .filter((pos): pos is number => pos !== null);

      const remainingHealthyBenchCount =
        playerData?.mons?.filter((m, idx) => {
          const pos = getMonTeamPosition(m, idx);
          return !m.active && m.hp > 0 && !chosenSwitchPositions.includes(pos);
        }).length || 0;

      const remainingChoicesToMake = needsSwitch.length - currentSlotIndex;
      const canPassSwitch =
        remainingHealthyBenchCount < remainingChoicesToMake || remainingHealthyBenchCount === 0;

      const handlePassSwitch = () => {
        if (submittingRef.current) return;
        const newChoices = [...choices, "pass"];
        advanceSlotOrSubmit(newChoices, needsSwitch.length);
      };

      const isMultiSlotBattle = needsSwitch.length > 1 || battleType !== "Singles";

      return (
        <div className="flex-col gap-m">
          <div className="card-header">
            <h3>
              {isMultiSlotBattle
                ? replaceMonName
                  ? `Switch slot ${activeSwitchSlot + 1}: ${replaceMonName}`
                  : `Switch slot ${activeSwitchSlot + 1}`
                : replaceMonName
                  ? `Switch: ${replaceMonName}`
                  : `Switch`}
            </h3>
            <div className={styles.headerActions}>{renderForfeitButton()}</div>
          </div>

          {canPassSwitch && (
            <button
              type="button"
              onClick={handlePassSwitch}
              className={`${styles.moveBtn} type-border`}
              style={{ "--type-color": "var(--color-warning)" } as CSSProperties}
              disabled={isLoading}
              title="Leave slot empty"
            >
              <div className={styles.moveHeaderRow}>
                <span className={styles.moveName}>Pass</span>
              </div>
              <span className={styles.moveMeta}>Leave slot empty</span>
            </button>
          )}

          {currentSlotIndex > 0 && (
            <div className="flex-row gap-s align-center">
              <button
                onClick={() => {
                  setChoices(choices.slice(0, -1));
                  setCurrentSlotIndex(currentSlotIndex - 1);
                }}
                className="btn btn-secondary"
                disabled={isLoading}
              >
                ← Back
              </button>
            </div>
          )}

          {renderChoiceStepper()}
        </div>
      );
    }

    if (request.type === "turn") {
      const activeRequests = request.active || [];
      const activeReq = activeRequests[currentSlotIndex];

      if (!activeReq) {
        return (
          <div className={`${styles.panelPlaceholder} ${styles.reset}`}>
            <p>Submitting...</p>
          </div>
        );
      }

      const activeMon = playerData?.mons?.find(
        (m) => m.player_team_position === activeReq.team_position,
      );
      const activeMonName =
        activeMon?.summary?.name || activeMon?.species || `Mon #${currentSlotIndex + 1}`;

      const TARGET_REQUIRING_SELECT = [
        "Normal",
        "AdjacentFoe",
        "AdjacentAlly",
        "Any",
        "AdjacentAllyOrUser",
      ];

      const handleSelectMove = (move: MonMoveSlotData, index: number) => {
        if (submittingRef.current) return;
        dispatch(setChoiceError({ battleId, error: null }));

        const requiresSelect = TARGET_REQUIRING_SELECT.includes(move.target);
        const dynamicTargetsLocal = requiresSelect
          ? getValidTargets({
              moveTarget: move.target,
              currentSlotIndex,
              battleType,
              activeRequestsCount: activeRequests.length,
              playerData,
              battleState: battleSession?.battleState,
            })
          : [];

        if (!requiresSelect || dynamicTargetsLocal.length <= 1) {
          const targetVal = requiresSelect ? (dynamicTargetsLocal[0]?.value ?? 1) : null;
          const choiceStr = ChoiceBuilder.move(index, targetVal, {
            mega,
            zmove,
            ultra,
            dyna,
            tera,
          });
          const newChoices = [...choices, choiceStr];
          advanceSlotOrSubmit(newChoices, activeRequests.length);
        } else {
          setSelectedMove(move);
          setSelectedMoveIndex(index);
          setDynamicTargets(dynamicTargetsLocal);
        }
      };

      const handleConfirmMove = (targetVal: number | null) => {
        if (submittingRef.current || selectedMoveIndex === null) return;

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
        advanceSlotOrSubmit(newChoices, activeRequests.length);
      };

      const handleBack = () => {
        if (submittingRef.current) return;
        dispatch(setChoiceError({ battleId, error: null }));
        if (selectedMove) {
          setSelectedMove(null);
          setSelectedMoveIndex(null);
        } else if (currentSlotIndex > 0) {
          setChoices(choices.slice(0, -1));
          setCurrentSlotIndex(currentSlotIndex - 1);
          resetModifiers();
        }
      };

      let isMonActiveDynamaxed = false;
      if (battleSession?.battleState && playerData) {
        try {
          isMonActiveDynamaxed = stateSelectors.monIsDynamaxed(battleSession.battleState, {
            player: playerData.id || playerData.name,
            mon_index: activeReq.team_position,
            battle_appearance_index: 0,
          });
        } catch {
          isMonActiveDynamaxed = false;
        }
      }

      const canShift = canSlotShift(
        currentSlotIndex,
        activeRequests.length,
        !!activeReq?.trapped,
      );
      const handleShift = () => {
        if (submittingRef.current) return;
        dispatch(setChoiceError({ battleId, error: null }));
        const choiceStr = ChoiceBuilder.shift();
        const newChoices = [...choices, choiceStr];
        advanceSlotOrSubmit(newChoices, activeRequests.length);
      };

      return (
        <div className="flex-col gap-m">
          <div className="card-header">
            <h3>
              {activeRequests.length > 1
                ? `Slot ${currentSlotIndex + 1}: ${activeMonName}`
                : activeMonName}
            </h3>
            <div className={styles.headerActions}>{renderForfeitButton()}</div>
          </div>

          {selectedMove === null ? (
            <MoveSelector
              activeReq={activeReq}
              isDynamaxed={isMonActiveDynamaxed}
              isLoading={isLoading}
              mega={mega}
              setMega={setMega}
              tera={tera}
              setTera={setTera}
              zmove={zmove}
              setZmove={setZmove}
              dyna={dyna}
              setDyna={setDyna}
              ultra={ultra}
              setUltra={setUltra}
              canShift={canShift}
              onShift={handleShift}
              onSelectMove={handleSelectMove}
              onClearError={() => dispatch(setChoiceError({ battleId, error: null }))}
            />
          ) : (
            <TargetSelector
              selectedMoveTarget={selectedMove.target}
              dynamicTargets={dynamicTargets}
              isLoading={isLoading}
              onConfirmMove={handleConfirmMove}
            />
          )}

          <div className="flex-row">
            {(currentSlotIndex > 0 || selectedMove !== null) && (
              <button onClick={handleBack} className="btn btn-secondary" disabled={isLoading}>
                ← Back
              </button>
            )}
          </div>

          {renderChoiceStepper()}
        </div>
      );
    }

    return null;
  };

  const displayErrorMessage = parsedChoiceError.failedSlotIndex !== null
    ? `Slot ${parsedChoiceError.failedSlotIndex + 1}: ${parsedChoiceError.errorMessage}`
    : errorMessage;

  return (
    <div className="flex-col gap-xl">
      <div className="card flex-col gap-m">
        {displayErrorMessage && <ErrorBanner message={displayErrorMessage} />}
        {renderChoiceBody()}
      </div>
      {renderTeamSummary()}
    </div>
  );
}
