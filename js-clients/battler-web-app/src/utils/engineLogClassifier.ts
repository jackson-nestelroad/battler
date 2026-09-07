export type EngineLogCategory =
  | "landmark"
  | "faint"
  | "action"
  | "event"
  | "noise";

const LANDMARK_COMMANDS = new Set(["turn", "battlestart", "win", "tie"]);

const ACTION_COMMANDS = new Set([
  "move",
  "animatemove",
  "switch",
  "drag",
  "replace",
  "switchout",
  "appear",
  "specieschange",
  "formechange",
  "transform",
  "mega",
  "primal",
  "ultra",
  "dynamax",
  "gigantamax",
]);

const NOISE_COMMANDS = new Set([
  "info",
  "side",
  "player",
  "teamsize",
  "mon",
  "teampreviewstart",
  "teampreview",
  "time",
  "continue",
  "split",
  "maxsidelength",
  "turnlimit",
  "debug",
  "fxlang_debug",
]);

/**
 * Classifies a raw engine log line into a styling category.
 */
export function classifyEngineLog(line: string): EngineLogCategory {
  if (line.startsWith("-battlerservice:")) {
    return "noise";
  }

  const pipeIdx = line.indexOf("|");
  const command = pipeIdx === -1 ? line.trim() : line.slice(0, pipeIdx).trim();

  if (command === "faint") {
    return "faint";
  }

  if (LANDMARK_COMMANDS.has(command)) {
    return "landmark";
  }

  if (ACTION_COMMANDS.has(command)) {
    return "action";
  }

  if (NOISE_COMMANDS.has(command)) {
    return "noise";
  }

  return "event";
}
