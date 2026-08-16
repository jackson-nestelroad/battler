import type { MonMoveSlotData, Request } from "battler-types";
import { useCallback, useEffect, useRef, useState } from "react";
import { submitChoice } from "../../../core/wamp";
import { setChoiceError } from "../../../store/battlesSlice";
import { useAppDispatch } from "../../../store/store";
import type { ParsedChoiceError } from "../../../utils/choiceParser";
import type { ChoiceModifiers } from "../../../utils/choiceBuilder";
import { getRequestSlotCount } from "../../../utils/monHelpers";

interface UseChoiceStepperOptions {
  battleId: string;
  request: Request | null;
  parsedChoiceError: ParsedChoiceError;
  isLoading?: boolean;
  choiceError?: string | null;
}

export function useChoiceStepper({
  battleId,
  request,
  parsedChoiceError,
  isLoading,
  choiceError,
}: UseChoiceStepperOptions) {
  const dispatch = useAppDispatch();

  const [choices, setChoices] = useState<string[]>([]);
  const [currentSlotIndex, setCurrentSlotIndex] = useState(0);
  const [selectedMove, setSelectedMove] = useState<MonMoveSlotData | null>(null);
  const [selectedMoveIndex, setSelectedMoveIndex] = useState<number | null>(null);
  const [selectedTeamIndices, setSelectedTeamIndices] = useState<number[]>([]);

  const [modifiers, setModifiers] = useState<ChoiceModifiers>({});

  const toggleModifier = useCallback((key: keyof ChoiceModifiers, value: boolean) => {
    setModifiers((prev) => ({ ...prev, [key]: value }));
  }, []);

  const submittingRef = useRef(false);

  // Reset submitting status when loading finishes or choiceError arrives
  useEffect(() => {
    if (!isLoading || choiceError) {
      submittingRef.current = false;
    }
  }, [isLoading, choiceError]);

  const resetModifiers = useCallback(() => {
    setModifiers({});
  }, []);

  const clearMoveSelection = useCallback(() => {
    setSelectedMove(null);
    setSelectedMoveIndex(null);
    resetModifiers();
  }, [resetModifiers]);

  const resetChoiceState = useCallback(() => {
    submittingRef.current = false;
    setChoices([]);
    setCurrentSlotIndex(0);
    setSelectedTeamIndices([]);
    clearMoveSelection();
  }, [clearMoveSelection]);

  // Reset when request changes
  const prevReqRef = useRef(request);
  useEffect(() => {
    if (prevReqRef.current !== request) {
      prevReqRef.current = request;
      resetChoiceState();
    }
  }, [request, resetChoiceState]);

  const goBackStep = useCallback(() => {
    if (submittingRef.current) return;
    dispatch(setChoiceError({ battleId, error: null }));
    
    if (selectedMove) {
      // 100% No-op: Backing out of target selection shouldn't wipe modifiers
      setSelectedMove(null);
      setSelectedMoveIndex(null);
    } else if (currentSlotIndex > 0) {
      // Backing out of a whole slot resets everything
      setChoices((prev) => prev.slice(0, -1));
      setCurrentSlotIndex((prev) => prev - 1);
      clearMoveSelection();
    }
  }, [dispatch, battleId, selectedMove, currentSlotIndex, clearMoveSelection]);

  const advanceSlotOrSubmit = useCallback(
    (newChoices: string[], totalSlotsRequired: number) => {
      dispatch(setChoiceError({ battleId, error: null }));
      if (newChoices.length >= totalSlotsRequired) {
        submittingRef.current = true;
        dispatch(submitChoice({ battleId, choice: newChoices.join("; ") }));
      } else {
        setChoices(newChoices);
        setCurrentSlotIndex(newChoices.length);
        clearMoveSelection();
      }
    },
    [battleId, dispatch, clearMoveSelection],
  );

  const handleJumpToSlot = useCallback(
    (slotIndex: number) => {
      if (submittingRef.current || slotIndex >= currentSlotIndex) return;
      dispatch(setChoiceError({ battleId, error: null }));
      setChoices((prev) => prev.slice(0, slotIndex));
      setCurrentSlotIndex(slotIndex);
      clearMoveSelection();
    },
    [currentSlotIndex, dispatch, battleId, clearMoveSelection],
  );

  // Auto-jump to failing slot when choiceError points to a specific failed slot index
  useEffect(() => {
    submittingRef.current = false;
    if (parsedChoiceError.failedSlotIndex !== null && request) {
      const totalSlots = getRequestSlotCount(request);

      if (parsedChoiceError.failedSlotIndex < totalSlots) {
        const failedIdx = parsedChoiceError.failedSlotIndex;
        setChoices((prev) => prev.slice(0, failedIdx));
        setCurrentSlotIndex(failedIdx);
        clearMoveSelection();
      }
    }
  }, [parsedChoiceError.failedSlotIndex, request, clearMoveSelection]);

  const clearChoiceError = useCallback(() => {
    dispatch(setChoiceError({ battleId, error: null }));
  }, [dispatch, battleId]);

  return {
    choices,
    setChoices,
    currentSlotIndex,
    setCurrentSlotIndex,
    selectedMove,
    setSelectedMove,
    selectedMoveIndex,
    setSelectedMoveIndex,
    selectedTeamIndices,
    setSelectedTeamIndices,
    modifiers,
    toggleModifier,
    submittingRef,
    resetModifiers,
    clearMoveSelection,
    clearChoiceError,
    advanceSlotOrSubmit,
    handleJumpToSlot,
    goBackStep,
  };
}
