import { newBattleState, alterBattleState } from "battler-state";
import type { BattleState } from "battler-state";

export interface ReplayKeyframe {
  turn: number;
  state: BattleState;
}

export interface SavedReplay {
  battleId: string;
  engineLogs: string[];
}

/**
 * Finds the line index in engineLogs where a specific turn starts.
 */
export function findTurnBoundary(engineLogs: string[], turn: number): number {
  const targetHeader = `turn|turn:${turn}`;
  for (let i = 0; i < engineLogs.length; i++) {
    if (engineLogs[i] && engineLogs[i].startsWith(targetHeader)) {
      return i;
    }
  }
  return engineLogs.length;
}

/**
 * Computes the log index boundary for a given step/turn in a replay.
 */
export function getReplayStepBoundary(engineLogs: string[], step: number, maxTurn: number): number {
  if (step === 0) {
    const battlestartIdx = engineLogs.findIndex(
      (log) => log === "battlestart" || log?.startsWith("battlestart|")
    );
    return battlestartIdx !== -1 ? battlestartIdx + 1 : findTurnBoundary(engineLogs, 1);
  }
  if (step > maxTurn) {
    return engineLogs.length;
  }
  return findTurnBoundary(engineLogs, step);
}

/**
 * Parses the engine logs and precomputes keyframe states at turn 0 and every 10 turns.
 * This runs in O(N) linear time because alterBattleState is called incrementally.
 */
export function precomputeReplayKeyframes(engineLogs: string[]): {
  keyframes: ReplayKeyframe[];
  maxTurn: number;
} {
  let maxTurn = 0;
  for (const log of engineLogs) {
    if (log && log.startsWith("turn|turn:")) {
      const turnNum = parseInt(log.substring("turn|turn:".length), 10);
      if (!isNaN(turnNum) && turnNum > maxTurn) {
        maxTurn = turnNum;
      }
    }
  }

  const keyframes: ReplayKeyframe[] = [];

  // Compute keyframe for step 0
  const turn0Boundary = getReplayStepBoundary(engineLogs, 0, maxTurn);
  let currentState = alterBattleState(newBattleState(), engineLogs.slice(0, turn0Boundary));
  keyframes.push({ turn: 0, state: currentState });

  // Compute keyframes every 10 turns (steps 10, 20, ...)
  for (let t = 10; t <= maxTurn; t += 10) {
    const boundary = getReplayStepBoundary(engineLogs, t, maxTurn);
    const slice = engineLogs.slice(0, boundary);
    // alterBattleState is incremental when state has last_log_index set
    currentState = alterBattleState(currentState, slice);
    keyframes.push({ turn: t, state: currentState });
  }

  // Also precompute the final keyframe at step maxTurn + 1 to avoid computing it from scratch
  const finalStep = maxTurn + 1;
  const finalBoundary = getReplayStepBoundary(engineLogs, finalStep, maxTurn);
  const finalSlice = engineLogs.slice(0, finalBoundary);
  currentState = alterBattleState(currentState, finalSlice);
  keyframes.push({ turn: finalStep, state: currentState });

  return { keyframes, maxTurn };
}

/**
 * Resolves the BattleState for turn T, utilizing keyframes to avoid O(N) calculations.
 * Results are cached in the session's replayStates sparse array.
 */
export function resolveReplayTurnState(
  session: {
    replayStates: (BattleState | undefined)[];
    replayEngineLogs: string[];
    replayKeyframes: ReplayKeyframe[];
  },
  turn: number,
): BattleState {
  // 1. Check cache first
  const cached = session.replayStates[turn];
  if (cached) {
    return cached;
  }

  const maxTurn = session.replayStates.length - 2;

  // 2. Find nearest keyframe <= turn
  let nearestKeyframe = session.replayKeyframes[0];
  for (const kf of session.replayKeyframes) {
    if (kf.turn <= turn && kf.turn > nearestKeyframe.turn) {
      nearestKeyframe = kf;
    }
  }

  // 3. Compute target turn state incrementally from keyframe
  const boundaryIdx = getReplayStepBoundary(session.replayEngineLogs, turn, maxTurn);
  const slice = session.replayEngineLogs.slice(0, boundaryIdx);
  const state = alterBattleState(nearestKeyframe.state, slice);

  // 4. Cache and return
  session.replayStates[turn] = state;
  return state;
}

/**
 * Encodes a string containing UTF-8 characters to a Base64 string.
 * Replaces deprecated `btoa(unescape(encodeURIComponent(str)))`.
 */
export function encodeBase64Utf8(str: string): string {
  const bytes = new TextEncoder().encode(str);
  let binString = "";
  for (let i = 0; i < bytes.length; i++) {
    binString += String.fromCharCode(bytes[i]);
  }
  return btoa(binString);
}

/**
 * Decodes a Base64 string to a UTF-8 string.
 * Replaces deprecated `decodeURIComponent(escape(atob(str)))`.
 */
export function decodeBase64Utf8(base64Str: string): string {
  const binaryStr = atob(base64Str);
  const bytes = new Uint8Array(binaryStr.length);
  for (let i = 0; i < binaryStr.length; i++) {
    bytes[i] = binaryStr.charCodeAt(i);
  }
  return new TextDecoder().decode(bytes);
}
