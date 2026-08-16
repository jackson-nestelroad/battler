import type { MonMoveSlotData, Request } from "battler-types";
import { useCallback, useEffect, useRef, useState } from "react";
import { submitChoice } from "../../../core/wamp";
import { setChoiceError } from "../../../store/battlesSlice";
import { useAppDispatch } from "../../../store/store";
import type { ParsedChoiceError } from "../../../utils/choiceErrorParser";
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

  const [mega, setMega] = useState(false);
  const [zmove, setZmove] = useState(false);
  const [ultra, setUltra] = useState(false);
  const [dyna, setDyna] = useState(false);
  const [tera, setTera] = useState(false);

  const submittingRef = useRef(false);

  // Reset submitting status when loading finishes or choiceError arrives
  useEffect(() => {
    if (!isLoading || choiceError) {
      submittingRef.current = false;
    }
  }, [isLoading, choiceError]);

  const resetModifiers = useCallback(() => {
    setMega(false);
    setZmove(false);
    setUltra(false);
    setDyna(false);
    setTera(false);
  }, []);

  const resetChoiceState = useCallback(() => {
    submittingRef.current = false;
    setChoices([]);
    setCurrentSlotIndex(0);
    setSelectedMove(null);
    setSelectedMoveIndex(null);
    setSelectedTeamIndices([]);
    resetModifiers();
  }, [resetModifiers]);

  // Reset when request changes
  const prevReqRef = useRef(request);
  useEffect(() => {
    if (prevReqRef.current !== request) {
      prevReqRef.current = request;
      resetChoiceState();
    }
  }, [request, resetChoiceState]);

  const advanceSlotOrSubmit = useCallback(
    (newChoices: string[], totalSlotsRequired: number) => {
      dispatch(setChoiceError({ battleId, error: null }));
      if (newChoices.length >= totalSlotsRequired) {
        submittingRef.current = true;
        dispatch(submitChoice({ battleId, choice: newChoices.join("; ") }));
      } else {
        setChoices(newChoices);
        setCurrentSlotIndex(newChoices.length);
        setSelectedMove(null);
        setSelectedMoveIndex(null);
        resetModifiers();
      }
    },
    [battleId, dispatch, resetModifiers, request],
  );

  const handleJumpToSlot = useCallback(
    (slotIndex: number) => {
      if (submittingRef.current || slotIndex >= currentSlotIndex) return;
      dispatch(setChoiceError({ battleId, error: null }));
      setChoices((prev) => prev.slice(0, slotIndex));
      setCurrentSlotIndex(slotIndex);
      setSelectedMove(null);
      setSelectedMoveIndex(null);
      resetModifiers();
    },
    [currentSlotIndex, dispatch, battleId, resetModifiers],
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
        setSelectedMove(null);
        setSelectedMoveIndex(null);
        resetModifiers();
      }
    }
  }, [parsedChoiceError.failedSlotIndex, request, resetModifiers]);

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
    mega,
    setMega,
    zmove,
    setZmove,
    ultra,
    setUltra,
    dyna,
    setDyna,
    tera,
    setTera,
    submittingRef,
    resetModifiers,
    advanceSlotOrSubmit,
    handleJumpToSlot,
  };
}
