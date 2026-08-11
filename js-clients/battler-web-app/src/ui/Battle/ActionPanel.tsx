import type { MonMoveSlotData, PlayerBattleData, Request } from "battler-types";
import { useEffect, useRef, useState } from "react";
import { submitChoice } from "../../core/wamp";
import { setChoiceError } from "../../store/battlesSlice";
import { useAppDispatch, useAppSelector } from "../../store/store";
import ErrorBanner from "../Common/ErrorBanner";
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

  // Reset when request or turn changes
  useEffect(() => {
    setCurrentSlotIndex(0);
    setChoices([]);
    setSelectedMove(null);
    setSelectedMoveIndex(null);
    setSelectedTeamIndices([]);
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
            {renderForfeitButton()}
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
      const activeSwitchSlot = needsSwitch[currentSlotIndex];

      if (activeSwitchSlot === undefined) {
        return (
          <div className={`${styles.panelPlaceholder} ${styles.reset}`}>
            <p>Submitting...</p>
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
            Switch: <strong>{replaceMonName}</strong>
          </h3>

          <div className="flex-row gap-s align-center">
            {currentSlotIndex > 0 && (
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
            )}
            {renderForfeitButton()}
          </div>
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

      const handleSelectMove = (move: MonMoveSlotData, index: number) => {
        if (submittingRef.current) return;
        dispatch(setChoiceError({ battleId, error: null }));
        setSelectedMove(move);
        setSelectedMoveIndex(index);
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

      return (
        <div className="flex-col gap-m">
          <div className="card-header">
            <h3>
              {activeMonName} ({currentSlotIndex + 1}/{activeRequests.length})
            </h3>
            <div className={styles.headerActions}>{renderForfeitButton()}</div>
          </div>

          {selectedMove === null ? (
            <MoveSelector
              activeReq={activeReq}
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
              onSelectMove={handleSelectMove}
              onClearError={() => dispatch(setChoiceError({ battleId, error: null }))}
            />
          ) : (
            <TargetSelector
              selectedMoveTarget={selectedMove.target}
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
        </div>
      );
    }

    return null;
  };

  return (
    <div className="flex-col gap-xl">
      <div className="card flex-col gap-m">
        <ErrorBanner message={errorMessage} />
        {renderChoiceBody()}
      </div>
      {renderTeamSummary()}
    </div>
  );
}
