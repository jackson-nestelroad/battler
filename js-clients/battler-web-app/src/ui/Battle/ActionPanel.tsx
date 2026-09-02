import type { MonMoveSlotData, PlayerBattleData, Request } from "battler-types";
import { useEffect, useMemo, useState } from "react";
import { submitChoice } from "../../core/wamp";
import { useAppDispatch, useAppSelector } from "../../store/store";
import { ChoiceBuilder } from "../../utils/choiceBuilder";
import { parseChoiceError, getChosenSwitchPositions } from "../../utils/choiceParser";
import {
  canSlotShift,
  getMonTeamPosition,
  getMonDisplayName,
  getSlotLabel,
  getMonForSlot,
  getAvailableBenchCount,
  getTeamPreviewTargetSize,
  getSlotMonName,
  getRequestSlotCount,
  getActiveSlotPosition,
} from "../../utils/monHelpers";
import { getMoveTargetInfo, getValidTargets, type TargetOption } from "../../utils/targeting";
import ErrorBanner from "../Common/ErrorBanner";
import ActionButton from "./ActionButton";
import ChoiceStepper from "./ChoiceStepper";
import { useChoiceStepper } from "./hooks/useChoiceStepper";
import MoveSelector from "./MoveSelector";
import TargetSelector from "./TargetSelector";
import TeamSummary from "./TeamSummary";
import ForfeitButton from "./ForfeitButton";
import { isMonDynamaxedInState } from "../../utils/battleState";

import styles from "./ActionPanel.module.scss";

interface RequestHeaderProps {
  title: string;
  onForfeit: () => void;
  isLoading: boolean;
}

function RequestHeader({ title, onForfeit, isLoading }: RequestHeaderProps) {
  return (
    <div className="card-header">
      <h3>{title}</h3>
      <ForfeitButton onForfeit={onForfeit} isLoading={isLoading} />
    </div>
  );
}

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

  const choiceError = battleSession?.choiceError || null;
  const parsedChoiceError = useMemo(() => parseChoiceError(choiceError), [choiceError]);

  const {
    choices,
    currentSlotIndex,
    selectedMove,
    setSelectedMove,
    selectedMoveIndex,
    setSelectedMoveIndex,
    selectedTeamIndices,
    setSelectedTeamIndices,
    modifiers,
    toggleModifier,
    submittingRef,
    clearChoiceError,
    advanceSlotOrSubmit,
    handleJumpToSlot,
    goBackStep,
  } = useChoiceStepper({
    battleId,
    request,
    parsedChoiceError,
    isLoading,
    choiceError,
  });

  const [dynamicTargets, setDynamicTargets] = useState<TargetOption[]>([]);

  // Reset dynamicTargets when request or turn changes
  useEffect(() => {
    setDynamicTargets([]);
  }, [request, turn]);

  const handleForfeit = () => {
    if (submittingRef.current) return;
    submittingRef.current = true;
    dispatch(submitChoice({ battleId, choice: "forfeit" }));
  };

  // Check if player has already submitted their choice for the current turn
  const isMeReady = !!battleSession?.choiceSubmitted;

  const battleType =
    battleSession?.serviceBattle?.metadata?.battle_type ||
    battleSession?.metadata?.battle_type ||
    "Singles";

  const activeMon = getMonForSlot(playerData, request, currentSlotIndex);
  const activeMonTeamPosition = activeMon ? getMonTeamPosition(activeMon, 0) : null;
  const monToReplace = request?.type === "switch" ? activeMon : undefined;
  const activeSwitchSlot = request?.type === "switch" ? getActiveSlotPosition(request, currentSlotIndex) : undefined;

  const isSwitch = request?.type === "switch";
  const isTurn = request?.type === "turn";

  const handleSwitch = (playerTeamPosition: number, totalSlots: number) => {
    if (submittingRef.current) return;
    const newChoices = [...choices, ChoiceBuilder.switch(playerTeamPosition)];
    advanceSlotOrSubmit(newChoices, totalSlots);
  };

  const handleSelectMon = (idx: number) => {
    if (submittingRef.current) return;
    clearChoiceError();
    setSelectedTeamIndices((prev) => {
      const exists = prev.indexOf(idx);
      if (exists !== -1) {
        return prev.filter((i) => i !== idx);
      } else {
        const targetSize = getTeamPreviewTargetSize(request, playerData);
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

  const getHeaderTitle = (): string => {
    if (isMeReady) return "Waiting";
    if (playbackPending) return "Turn Resolution";
    if (request?.type === "team") return "Team Preview";
    if (request?.type === "switch") {
      if (activeSwitchSlot === undefined) return "Switch";
      const replaceMonName = getMonDisplayName(monToReplace);
      const totalSlots = getRequestSlotCount(request);
      const isMultiSlotBattle = totalSlots > 1 || battleType !== "Singles";
      return isMultiSlotBattle
        ? `Switch ${getSlotLabel(activeSwitchSlot + 1, replaceMonName)}`
        : replaceMonName
          ? `Switch: ${replaceMonName}`
          : "Switch";
    }
    if (request?.type === "turn") {
      const activeRequests = request.active || [];
      const activeMonName = getSlotMonName(activeMon, currentSlotIndex);
      return activeRequests.length > 1
        ? getSlotLabel(currentSlotIndex + 1, activeMonName)
        : activeMonName;
    }
    return "Battle";
  };

  const renderPlaceholder = (text: string, showSpinner = false) => (
    <div className={styles.panelPlaceholder}>
      {showSpinner ? (
        <div className="flex-col align-center gap-m">
          <div className={styles.dotPulse} />
          <p>{text}</p>
        </div>
      ) : (
        <p>{text}</p>
      )}
    </div>
  );

  const renderChoiceBody = () => {
    if (!request || isMeReady) return renderPlaceholder("Waiting...", true);
    if (playbackPending) return renderPlaceholder("Playing turn...", true);

    if (request.type === "team") {
      const targetSize = getTeamPreviewTargetSize(request, playerData);

      const handleTeamPreviewSubmit = () => {
        if (submittingRef.current) return;
        submittingRef.current = true;
        if (selectedTeamIndices.length > 0) {
          dispatch(submitChoice({ battleId, choice: ChoiceBuilder.team(selectedTeamIndices) }));
        } else {
          dispatch(submitChoice({ battleId, choice: "team" }));
        }
      };

      const handleClearSelection = () => {
        setSelectedTeamIndices([]);
      };

      return (
        <div className="flex-col gap-s">
          <div className={styles.columnHeaderRow}>
            <h4 className={styles.summaryTitle}>Team Preview</h4>
          </div>
          <p className={styles.instructionText}>
            Select your team order from the right. Remaining spots will be filled automatically.
          </p>
          <span className={styles.selectionProgress}>
            Selected: <strong>{selectedTeamIndices.length}</strong> / {targetSize}
          </span>

          <div className="flex-row gap-s align-center">
            <button
              type="button"
              className="btn btn-primary"
              onClick={handleTeamPreviewSubmit}
              disabled={isLoading}
            >
              Confirm team
            </button>
            {selectedTeamIndices.length > 0 && (
              <button
                type="button"
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
      if (activeSwitchSlot === undefined) return renderPlaceholder("Submitting...");

      const chosenSwitchPositions = getChosenSwitchPositions(choices);
      const remainingHealthyBenchCount = getAvailableBenchCount(playerData, chosenSwitchPositions);
      const totalSlots = getRequestSlotCount(request);
      const remainingChoicesToMake = totalSlots - currentSlotIndex;
      const canPassSwitch =
        remainingHealthyBenchCount < remainingChoicesToMake || remainingHealthyBenchCount === 0;

      const handlePassSwitch = () => {
        if (submittingRef.current) return;
        const newChoices = [...choices, ChoiceBuilder.pass()];
        advanceSlotOrSubmit(newChoices, totalSlots);
      };

      return (
        <div className="flex-col gap-m">
          {showBackButton && (
            <div className="flex-row">
              <button
                type="button"
                onClick={goBackStep}
                className="btn btn-sm btn-secondary"
                disabled={isLoading}
                title="Go back to previous choice"
              >
                ← Back
              </button>
            </div>
          )}
          {renderPlaceholder("Switching...", true)}
          {canPassSwitch && (
            <ActionButton
              title="Pass"
              subtitle="Leave slot empty"
              onClick={handlePassSwitch}
              disabled={isLoading}
              typeColor="var(--color-warning)"
              htmlTitle="Leave slot empty"
            />
          )}
        </div>
      );
    }

    if (request.type === "turn") {
      const activeRequests = request.active || [];
      const activeReq = activeRequests[currentSlotIndex];

      if (!activeReq) return renderPlaceholder("Submitting...");

      const submitMoveChoice = (moveIdx: number, targetVal: number | null) => {
        const choiceStr = ChoiceBuilder.move(moveIdx, targetVal, modifiers);
        const newChoices = [...choices, choiceStr];
        advanceSlotOrSubmit(newChoices, activeRequests.length);
      };

      const handleSelectMove = (move: MonMoveSlotData, index: number) => {
        if (submittingRef.current) return;
        clearChoiceError();

        const requiresSelect = getMoveTargetInfo(move.target).isChoosable;
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
          submitMoveChoice(index, targetVal);
        } else {
          setSelectedMove(move);
          setSelectedMoveIndex(index);
          setDynamicTargets(dynamicTargetsLocal);
        }
      };

      const handleConfirmMove = (targetVal: number | null) => {
        if (submittingRef.current || selectedMoveIndex === null) return;
        submitMoveChoice(selectedMoveIndex, targetVal);
      };

      const isMonActiveDynamaxed = isMonDynamaxedInState(
        battleSession?.battleState,
        playerData?.side ?? 0,
        currentSlotIndex,
      );

      const canShift = canSlotShift(
        currentSlotIndex,
        activeRequests.length,
        !!activeReq?.trapped,
      );
      const handleShift = () => {
        if (submittingRef.current) return;
        const choiceStr = ChoiceBuilder.shift();
        const newChoices = [...choices, choiceStr];
        advanceSlotOrSubmit(newChoices, activeRequests.length);
      };

      return selectedMove === null ? (
        <MoveSelector
          activeReq={activeReq}
          isDynamaxed={isMonActiveDynamaxed}
          isLoading={isLoading}
          modifiers={modifiers}
          toggleModifier={toggleModifier}
          canShift={canShift}
          onShift={handleShift}
          onSelectMove={handleSelectMove}
          onClearError={clearChoiceError}
          onBack={showBackButton ? goBackStep : undefined}
        />
      ) : (
        <TargetSelector
          selectedMoveTarget={selectedMove.target}
          dynamicTargets={dynamicTargets}
          isLoading={isLoading}
          onConfirmMove={handleConfirmMove}
          onBack={showBackButton ? goBackStep : undefined}
        />
      );
    }

    return null;
  };

  let displayErrorMessage = errorMessage;
  if (parsedChoiceError.failedSlotIndex !== null) {
    const failedIdx = parsedChoiceError.failedSlotIndex;
    const actualSlotPos = getActiveSlotPosition(request, failedIdx);
    displayErrorMessage = `Slot ${actualSlotPos + 1}: ${parsedChoiceError.errorMessage}`;
  } else if (choiceError) {
    displayErrorMessage = choiceError;
  }

  // Determine visibility states for hoisted elements
  let showHeader = false;
  let showStepper = false;
  if (request && !isMeReady && !playbackPending) {
    if (request.type === "team") {
      showHeader = true;
    } else if (isSwitch) {
      showHeader = activeSwitchSlot !== undefined;
      showStepper = activeSwitchSlot !== undefined;
    } else if (isTurn && request.type === "turn") {
      showHeader = !!request.active?.[currentSlotIndex];
      showStepper = !!request.active?.[currentSlotIndex];
    }
  }

  const showBackButton = isSwitch
    ? currentSlotIndex > 0
    : isTurn
      ? currentSlotIndex > 0 || selectedMove !== null
      : false;

  return (
    <div className={`card ${styles.actionPanelCard}`}>
      {displayErrorMessage && <ErrorBanner message={displayErrorMessage} />}
      {showHeader && (
        <RequestHeader
          title={getHeaderTitle()}
          onForfeit={handleForfeit}
          isLoading={isLoading}
          key={turn}
        />
      )}

      <div className={styles.commandDeckGrid}>
        <div className={styles.commandActionCol}>
          {renderChoiceBody()}
          {showStepper && (
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
          )}
        </div>
        <div className={styles.commandTeamCol}>
          {renderTeamSummary()}
        </div>
      </div>
    </div>
  );
}
