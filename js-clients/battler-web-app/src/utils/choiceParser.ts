import { CHOICE_MODIFIER_KEYS, type ChoiceModifiers } from "./choiceBuilder";

export interface ParsedChoiceError {
  failedSlotIndex: number | null;
  errorMessage: string;
}

export interface ParsedChoiceAction {
  type: "move" | "switch" | "pass" | "shift" | "unknown";
  moveIndex?: number;
  targetVal?: number | null;
  modifiers?: ChoiceModifiers;
  switchPosition?: number;
}

function parseCommandIndex(head: string): number {
  const spaceIndex = head.indexOf(" ");
  return spaceIndex !== -1 ? parseInt(head.substring(spaceIndex + 1), 10) : 0;
}

export function parseChoiceString(choiceStr: string): ParsedChoiceAction {
  const parts = choiceStr.split(",").map((p) => p.trim());
  const head = parts[0] || "";

  if (head.startsWith("move")) {
    const moveIndex = parseCommandIndex(head);

    let targetVal: number | null = null;
    let nextPartIdx = 1;

    if (parts.length > 1) {
      const maybeTarget = parseInt(parts[1], 10);
      if (!isNaN(maybeTarget)) {
        targetVal = maybeTarget;
        nextPartIdx = 2;
      }
    }

    const flags = new Set(parts.slice(nextPartIdx));

    const modifiers: ChoiceModifiers = {};
    for (const mod of CHOICE_MODIFIER_KEYS) {
      if (flags.has(mod)) modifiers[mod] = true;
    }

    return {
      type: "move",
      moveIndex: isNaN(moveIndex) ? 0 : moveIndex,
      targetVal,
      modifiers,
    };
  }

  if (head.startsWith("switch")) {
    const switchPosition = parseCommandIndex(head);
    return {
      type: "switch",
      switchPosition: isNaN(switchPosition) ? 0 : switchPosition,
    };
  }

  if (head === "pass") {
    return { type: "pass" };
  }

  if (head === "shift") {
    return { type: "shift" };
  }

  return { type: "unknown" };
}

/**
 * Extracts chosen switch positions from an array of choice strings.
 */
export function getChosenSwitchPositions(choices: string[]): number[] {
  return choices
    .map((c) => {
      const parsed = parseChoiceString(c);
      return parsed.type === "switch" && parsed.switchPosition !== undefined
        ? parsed.switchPosition
        : null;
    })
    .filter((pos): pos is number => pos !== null);
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
