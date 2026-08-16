import type { MonMoveSlotData, PlayerBattleData, Request } from "battler-types";
import type { BattleState } from "battler-state";
import { getMonNameFromState, getPlayerNameFromState } from "./targeting";

export interface ParsedChoiceAction {
  type: "move" | "switch" | "pass" | "shift" | "unknown";
  moveIndex?: number;
  targetVal?: number | null;
  mega?: boolean;
  zmove?: boolean;
  ultra?: boolean;
  dyna?: boolean;
  tera?: boolean;
  switchPosition?: number;
}

export interface FormattedChoice {
  slotIndex: number;
  monName: string;
  actionType: "move" | "switch" | "pass" | "shift" | "unknown";
  actionName: string;
  targetName?: string | null;
  modifiers: string[];
  summaryText: string;
}

export function parseChoiceString(choiceStr: string): ParsedChoiceAction {
  const parts = choiceStr.split(",").map((p) => p.trim());
  const head = parts[0] || "";

  if (head.startsWith("move")) {
    const spaceIndex = head.indexOf(" ");
    const moveIndex = spaceIndex !== -1 ? parseInt(head.substring(spaceIndex + 1), 10) : 0;

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

    return {
      type: "move",
      moveIndex: isNaN(moveIndex) ? 0 : moveIndex,
      targetVal,
      mega: flags.has("mega"),
      zmove: flags.has("zmove"),
      ultra: flags.has("ultra"),
      dyna: flags.has("dyna"),
      tera: flags.has("tera"),
    };
  }

  if (head.startsWith("switch")) {
    const spaceIndex = head.indexOf(" ");
    const switchPosition = spaceIndex !== -1 ? parseInt(head.substring(spaceIndex + 1), 10) : 0;
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

export function resolveTargetName(
  targetVal: number | null | undefined,
  userSlotIndex: number,
  battleState?: BattleState | null,
  playerData?: PlayerBattleData | null,
): string | null {
  if (targetVal === null || targetVal === undefined || targetVal === 0) {
    return null;
  }

  if (targetVal > 0) {
    // Foe side (Side index 1)
    const foePos = targetVal - 1;
    const foeName = getMonNameFromState(battleState, 1, foePos);
    const foePlayer = getPlayerNameFromState(battleState, 1, foePos);
    const labelName = foeName || `Foe ${foePos + 1}`;
    return foePlayer ? `${labelName} (Foe • ${foePlayer})` : `${labelName} (Foe)`;
  } else {
    // Ally side (Side index 0)
    const allyPos = Math.abs(targetVal) - 1;
    if (allyPos === userSlotIndex) {
      const selfMon = playerData?.mons?.find((m) => m.player_active_position === userSlotIndex);
      const selfName =
        selfMon?.summary?.name ||
        selfMon?.species ||
        getMonNameFromState(battleState, 0, userSlotIndex) ||
        "Self";
      return `Self (${selfName})`;
    }

    const allyMon = playerData?.mons?.find((m) => m.player_active_position === allyPos);
    const allyMonName =
      allyMon?.summary?.name ||
      allyMon?.species ||
      getMonNameFromState(battleState, 0, allyPos) ||
      `Ally ${allyPos + 1}`;
    const allyPlayer = getPlayerNameFromState(battleState, 0, allyPos);
    return allyPlayer ? `${allyMonName} (Ally • ${allyPlayer})` : `${allyMonName} (Ally)`;
  }
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

  const activeMon = playerData?.mons?.find((m) => m.player_team_position === activeReq?.team_position);
  const monName = activeMon?.summary?.name || activeMon?.species || `Mon #${slotIndex + 1}`;

  const parsed = parseChoiceString(choiceStr);

  if (parsed.type === "switch") {
    const targetMon = playerData?.mons?.find((m) => m.player_team_position === parsed.switchPosition);
    const switchMonName = targetMon?.summary?.name || targetMon?.species || `Mon #${(parsed.switchPosition || 0) + 1}`;
    return {
      slotIndex,
      monName,
      actionType: "switch",
      actionName: "Switch",
      targetName: switchMonName,
      modifiers: [],
      summaryText: `Switch → ${switchMonName}`,
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
      summaryText: "Shift → Center",
    };
  }

  if (parsed.type === "move" && activeReq) {
    let selectedMoveData: MonMoveSlotData | null = null;

    if (parsed.zmove && activeReq.z_moves) {
      selectedMoveData = activeReq.z_moves[parsed.moveIndex || 0];
    } else if (parsed.dyna && activeReq.max_moves) {
      selectedMoveData = activeReq.max_moves[parsed.moveIndex || 0];
    }

    if (!selectedMoveData && activeReq.moves) {
      selectedMoveData = activeReq.moves[parsed.moveIndex || 0] || null;
    }

    const moveName = selectedMoveData?.name || `Move ${parsed.moveIndex !== undefined ? parsed.moveIndex + 1 : 1}`;

    const modifiers: string[] = [];
    if (parsed.mega) modifiers.push("Mega");
    if (parsed.zmove) modifiers.push("Z-Move");
    if (parsed.ultra) modifiers.push("Ultra");
    if (parsed.dyna) modifiers.push("Dynamax");
    if (parsed.tera) modifiers.push("Tera");

    const targetName = resolveTargetName(parsed.targetVal, slotIndex, battleState, playerData);

    let summaryText = moveName;
    if (modifiers.length > 0) {
      summaryText += ` (${modifiers.join(", ")})`;
    }
    if (targetName) {
      summaryText += ` → ${targetName}`;
    }

    return {
      slotIndex,
      monName,
      actionType: "move",
      actionName: moveName,
      targetName,
      modifiers,
      summaryText,
    };
  }

  if (parsed.type === "pass") {
    return {
      slotIndex,
      monName,
      actionType: "pass",
      actionName: "Pass",
      modifiers: [],
      summaryText: request?.type === "switch" ? "Leave Empty (Pass)" : "Pass",
    };
  }

  return {
    slotIndex,
    monName,
    actionType: parsed.type,
    actionName: choiceStr,
    modifiers: [],
    summaryText: choiceStr,
  };
}
