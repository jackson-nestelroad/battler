import type {
  BattleState,
  Side,
  Player,
  Mon,
  MonPhysicalAppearance,
  MonBattleAppearance,
  MonBattleAppearanceReference,
  MonBattleAppearanceWithRecovery,
  DiscoveryRequired,
  DiscoveryRequiredSet,
} from "battler-state";

// Helper to extract known value from DiscoveryRequired
function knownValue<T>(discovery: DiscoveryRequired<T> | null | undefined): T | null {
  if (discovery && "known" in discovery) {
    return discovery.known;
  }
  return null;
}

// Helper to extract known values from DiscoveryRequiredSet
function knownValues<T>(set: DiscoveryRequiredSet<T> | null | undefined): T[] {
  return set ? set.known : [];
}

// Helper to extract all possible values from DiscoveryRequiredSet
function allPossibleValues<T>(set: DiscoveryRequiredSet<T> | null | undefined): T[] {
  if (!set) return [];
  return [...set.known, ...set.possibly_includes];
}

// Helper to get the primary appearance from recovery appearance
function monBattleAppearancePrimary(
  recovery: MonBattleAppearanceWithRecovery,
): MonBattleAppearance {
  if ("inactive" in recovery) {
    return recovery.inactive;
  }
  return recovery.active.primary_battle_appearance;
}

export function fieldWeather(state: BattleState): string | null {
  return state.field.weather || null;
}

export function fieldTerrain(state: BattleState): string | null {
  const keys = Object.keys(state.field.conditions || {});
  return keys.find((name) => name.endsWith("Terrain")) || null;
}

export function fieldConditions(state: BattleState): string[] {
  const keys = Object.keys(state.field.conditions || {});
  return keys.filter((name) => !name.endsWith("Terrain"));
}

export function sideOrElse(state: BattleState, sideIndex: number): Side {
  const s = state.field.sides[sideIndex];
  if (!s) {
    throw new Error("side not found");
  }
  return s;
}

export function side(state: BattleState, sideIndex: number): Side | null {
  try {
    return sideOrElse(state, sideIndex);
  } catch {
    return null;
  }
}

export function sideConditions(state: BattleState, sideIndex: number): string[] {
  const s = sideOrElse(state, sideIndex);
  return Object.keys(s.conditions || {});
}

export function sideForMon(state: BattleState, monRef: MonBattleAppearanceReference): number {
  const sideAndPlayer = sideAndPlayerOrElse(state, monRef.player);
  return sideAndPlayer.sideIndex;
}

export function sideForPlayer(state: BattleState, player: string): number {
  const sideAndPlayer = sideAndPlayerOrElse(state, player);
  return sideAndPlayer.sideIndex;
}

export function playerOrElse(state: BattleState, playerName: string): Player {
  const sideAndPlayer = sideAndPlayerOrElse(state, playerName);
  return sideAndPlayer.playerObj;
}

export function player(state: BattleState, playerName: string): Player | null {
  try {
    return playerOrElse(state, playerName);
  } catch {
    return null;
  }
}

export function monOrElse(state: BattleState, monRef: MonBattleAppearanceReference): Mon {
  const p = playerOrElse(state, monRef.player);
  const m = p.mons[monRef.mon_index];
  if (!m) {
    throw new Error("mon not found");
  }
  return m;
}

export function mon(state: BattleState, monRef: MonBattleAppearanceReference): Mon | null {
  try {
    return monOrElse(state, monRef);
  } catch {
    return null;
  }
}

export function monBattleAppearanceOrElse(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
): MonBattleAppearance {
  const m = monOrElse(state, monRef);
  const recovery = m.battle_appearances[monRef.battle_appearance_index];
  if (!recovery) {
    throw new Error("mon battle appearance not found");
  }
  return monBattleAppearancePrimary(recovery);
}

export function monBattleAppearance(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
): MonBattleAppearance | null {
  try {
    return monBattleAppearanceOrElse(state, monRef);
  } catch {
    return null;
  }
}

export function monLevel(state: BattleState, monRef: MonBattleAppearanceReference): number | null {
  const app = monBattleAppearanceOrElse(state, monRef);
  const val = knownValue(app.level);
  return val !== null ? Number(val) : null;
}

export function monHealth(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
): [number, number] | null {
  const app = monBattleAppearanceOrElse(state, monRef);
  const val = knownValue(app.health);
  return val ? [Number(val[0]), Number(val[1])] : null;
}

export function monStatus(state: BattleState, monRef: MonBattleAppearanceReference): string | null {
  const app = monBattleAppearanceOrElse(state, monRef);
  const val = knownValue(app.status);
  return val && val !== "" ? val : null;
}

export function monAbility(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
): string | null {
  const m = monOrElse(state, monRef);
  if (m.volatile_data.ability) {
    return m.volatile_data.ability;
  }
  const app = monBattleAppearanceOrElse(state, monRef);
  const val = knownValue(app.ability);
  return val && val !== "" ? val : null;
}

export function monMoves(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
  includePossible: boolean,
): string[] {
  const m = monOrElse(state, monRef);
  if (m.volatile_data.moves.length > 0) {
    return m.volatile_data.moves;
  }
  return includePossible
    ? monAllPossibleNonVolatileMoves(state, monRef)
    : monKnownNonVolatileMoves(state, monRef);
}

export function monKnownNonVolatileMoves(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
): string[] {
  const app = monBattleAppearanceOrElse(state, monRef);
  return knownValues(app.moves);
}

export function monAllPossibleNonVolatileMoves(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
): string[] {
  const app = monBattleAppearanceOrElse(state, monRef);
  return allPossibleValues(app.moves);
}

export function monItem(state: BattleState, monRef: MonBattleAppearanceReference): string | null {
  const app = monBattleAppearanceOrElse(state, monRef);
  const val = knownValue(app.item);
  return val && val !== "" ? val : null;
}

export function monSpecies(state: BattleState, monRef: MonBattleAppearanceReference): string {
  const m = monOrElse(state, monRef);
  if (m.volatile_data.transformed) {
    return m.volatile_data.transformed[0].species;
  }
  if (m.volatile_data.forme_change) {
    return m.volatile_data.forme_change;
  }
  return m.physical_appearance.species;
}

export function monTypes(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
  getSpeciesTypes: (species: string) => string[],
): string[] {
  const m = monOrElse(state, monRef);
  const volatileTypes = m.volatile_data.types;
  let types = [...volatileTypes];
  if (types.length === 0) {
    types = getSpeciesTypes(monSpecies(state, monRef));
  }
  if (m.volatile_data.added_type) {
    const added = m.volatile_data.added_type;
    if (!types.includes(added)) {
      types.push(added);
    }
  }
  return types;
}

export function monBoosts(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
): Record<string, number> {
  const m = monOrElse(state, monRef);
  const boosts = m.volatile_data.stat_boosts;
  const obj: Record<string, number> = {};
  if (boosts instanceof Map) {
    for (const [k, v] of boosts.entries()) {
      obj[k] = Number(v);
    }
  } else if (boosts) {
    for (const [k, v] of Object.entries(boosts)) {
      if (v !== undefined && v !== null) {
        obj[k] = Number(v);
      }
    }
  }
  return obj;
}

export function monConditions(state: BattleState, monRef: MonBattleAppearanceReference): string[] {
  const m = monOrElse(state, monRef);
  return Object.keys(m.volatile_data.conditions || {});
}

export function monActivePosition(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
): number | null {
  const sideIndex = sideForMon(state, monRef);
  const s = sideOrElse(state, sideIndex);
  const index = s.active.findIndex((active) => {
    return (
      active !== null &&
      active !== undefined &&
      active.player === monRef.player &&
      active.mon_index === monRef.mon_index
    );
  });
  return index !== -1 ? index : null;
}

export function monIsFainted(state: BattleState, monRef: MonBattleAppearanceReference): boolean {
  const m = monOrElse(state, monRef);
  return m.fainted;
}

export function monPhysicalAppearance(
  state: BattleState,
  monRef: MonBattleAppearanceReference,
): MonPhysicalAppearance | null {
  const m = monOrElse(state, monRef);
  return m.physical_appearance;
}

export function monIsActive(state: BattleState, monRef: MonBattleAppearanceReference): boolean {
  return monActivePosition(state, monRef) !== null;
}

export function activeMonByPosition(
  state: BattleState,
  sideIndex: number,
  positionIndex: number,
): MonBattleAppearanceReference | null {
  const s = sideOrElse(state, sideIndex);
  return s.active[positionIndex] || null;
}

export function playerMons(state: BattleState, playerName: string): Mon[] {
  const p = playerOrElse(state, playerName);
  return p.mons;
}

export function playerBroughtMons(state: BattleState, playerName: string): Mon[] {
  const p = playerOrElse(state, playerName);
  return p.mons.filter((m) => m.brought);
}

export function sidePlayers(state: BattleState, sideIndex: number): Player[] {
  const s = sideOrElse(state, sideIndex);
  return Object.values(s.players || {}).filter((p): p is Player => p !== undefined);
}

// Internal helper
function sideAndPlayerOrElse(
  state: BattleState,
  player: string,
): { sideIndex: number; playerObj: Player } {
  for (let i = 0; i < state.field.sides.length; i++) {
    const s = state.field.sides[i];
    const p = s.players ? s.players[player] || null : null;
    if (p) {
      return { sideIndex: i, playerObj: p };
    }
  }
  throw new Error("player not found");
}
