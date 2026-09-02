import type { MonMoveSlotData, PlayerBattleData, Request, MonMoveRequest } from "battler-types";
import type { BattleState } from "battler-state";
import { getMonByTeamPosition, getMonForSlot, getAvailableMoves, getSlotMonName } from "./monHelpers";
import { resolveTargetLabel } from "./targeting";
import { parseChoiceString, type ParsedChoiceAction } from "./choiceParser";
import { CHOICE_MODIFIER_KEYS, CHOICE_MODIFIER_CONFIGS } from "./choiceBuilder";

export interface FormattedChoice {
  slotIndex: number;
  monName: string;
  actionType: "move" | "switch" | "pass" | "shift" | "unknown";
  actionName: string;
  targetName?: string | null;
  modifiers: string[];
}

export function resolveSelectedMove(
  activeReq:
    | MonMoveRequest
    | {
        moves?: MonMoveSlotData[];
        z_moves?: (MonMoveSlotData | null)[];
        max_moves?: MonMoveSlotData[];
      }
    | null
    | undefined,
  parsed: ParsedChoiceAction,
): MonMoveSlotData | null {
  const availableMoves = getAvailableMoves(activeReq, parsed.modifiers || {});
  return availableMoves[parsed.moveIndex || 0] || null;
}

export function formatTurnChoice(
  choiceStr: string,
  slotIndex: number,
  request: Request | null,
  playerData?: PlayerBattleData | null,
  battleState?: BattleState | null,
): FormattedChoice {
  const activeReqs = request?.type === "turn" ? request.active : [];
  const activeReq = activeReqs[slotIndex];

  const activeMon = getMonForSlot(playerData, request, slotIndex);
  const monName = getSlotMonName(activeMon, slotIndex);

  const parsed = parseChoiceString(choiceStr);

  if (parsed.type === "switch") {
    const targetMon = getMonByTeamPosition(playerData, parsed.switchPosition || 0);
    const switchMonName = getSlotMonName(targetMon, parsed.switchPosition || 0);
    return {
      slotIndex,
      monName,
      actionType: "switch",
      actionName: "Switch",
      targetName: switchMonName,
      modifiers: [],
    };
  }

  if (parsed.type === "shift") {
    return {
      slotIndex,
      monName,
      actionType: "shift",
      actionName: "Shift",
      targetName: "Center",
      modifiers: [],
    };
  }

  if (parsed.type === "move" && activeReq) {
    const selectedMoveData = resolveSelectedMove(activeReq, parsed);

    const moveName = selectedMoveData?.name || `Move ${parsed.moveIndex !== undefined ? parsed.moveIndex + 1 : 1}`;

    const modifiers: string[] = [];
    if (parsed.modifiers) {
      for (const mod of CHOICE_MODIFIER_KEYS) {
        if (parsed.modifiers[mod]) {
          modifiers.push(CHOICE_MODIFIER_CONFIGS[mod].label);
        }
      }
    }

    const targetName = resolveTargetLabel(parsed.targetVal, slotIndex, battleState, playerData);

    return {
      slotIndex,
      monName,
      actionType: "move",
      actionName: moveName,
      targetName,
      modifiers,
    };
  }

  if (parsed.type === "pass") {
    return {
      slotIndex,
      monName,
      actionType: "pass",
      actionName: "Pass",
      modifiers: [],
    };
  }

  return {
    slotIndex,
    monName,
    actionType: parsed.type,
    actionName: choiceStr,
    modifiers: [],
  };
}
