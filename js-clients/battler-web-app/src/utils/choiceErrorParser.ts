export interface ParsedChoiceError {
  failedSlotIndex: number | null;
  errorMessage: string;
}

/**
  * Parses server choice error strings formatted like:
  * "invalid choice 1: cannot switch: the mon in slot 3 can only switch in once"
  * "invalid choice 0: cannot move: invalid target for Draco Meteor"
  */
export function parseChoiceError(errorStr: string | null | undefined): ParsedChoiceError {
  if (!errorStr) {
    return { failedSlotIndex: null, errorMessage: "" };
  }

  const match = errorStr.match(/^invalid choice (\d+):\s*(.*)/i);
  if (match) {
    const slotIdx = parseInt(match[1], 10);
    const cleanMsg = match[2].trim();
    return {
      failedSlotIndex: isNaN(slotIdx) ? null : slotIdx,
      errorMessage: cleanMsg || errorStr,
    };
  }

  return {
    failedSlotIndex: null,
    errorMessage: errorStr,
  };
}
