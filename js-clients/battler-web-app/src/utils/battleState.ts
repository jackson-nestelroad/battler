export interface BattleStateInput {
  state?: string | null;
  phase?: unknown;
  turn?: number | bigint | null;
}

export function getBattleStateLabel(input: BattleStateInput): string {
  const { state, phase, turn } = input;

  if (state === "preparing") {
    return "Preparing";
  }

  const phaseStr =
    typeof phase === "string"
      ? phase
      : phase && typeof phase === "object"
        ? Object.keys(phase)[0]
        : null;

  if (state === "finished" || phaseStr === "finished") {
    return "Finished";
  }

  if (phaseStr === "pre_battle" || turn === 0 || turn === 0n) {
    return "Preview";
  }

  if (turn !== undefined && turn !== null && turn > 0) {
    return `Turn ${turn}`;
  }

  if (state === "active") {
    return "Preview";
  }

  return "Preview";
}
