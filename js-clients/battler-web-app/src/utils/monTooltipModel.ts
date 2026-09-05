import type { BattleState, UiMon, MonBattleAppearanceReference } from "battler-state";
import { stateSelectors } from "battler-state";
import type { BoostTable, MonBattleData, Nature, Stat } from "battler-types";
import {
  computeHpPercentage,
  getMonDisplayName,
  normalizeStatusCode,
} from "./monHelpers";

export interface NatureModifier {
  plus?: string;
  minus?: string;
}

export const NATURE_MODIFIERS: Record<string, NatureModifier> = {
  Adamant: { plus: "Atk", minus: "SpA" },
  Bold: { plus: "Def", minus: "Atk" },
  Brave: { plus: "Atk", minus: "Spe" },
  Calm: { plus: "SpD", minus: "Atk" },
  Careful: { plus: "SpD", minus: "SpA" },
  Gentle: { plus: "SpD", minus: "Def" },
  Hasty: { plus: "Spe", minus: "Def" },
  Impish: { plus: "Def", minus: "SpA" },
  Jolly: { plus: "Spe", minus: "SpA" },
  Lax: { plus: "Def", minus: "SpD" },
  Lonely: { plus: "Atk", minus: "Def" },
  Mild: { plus: "SpA", minus: "Def" },
  Modest: { plus: "SpA", minus: "Atk" },
  Naive: { plus: "Spe", minus: "SpD" },
  Naughty: { plus: "Atk", minus: "SpD" },
  Quiet: { plus: "SpA", minus: "Spe" },
  Rash: { plus: "SpA", minus: "SpD" },
  Relaxed: { plus: "Def", minus: "Spe" },
  Sassy: { plus: "SpD", minus: "Spe" },
  Timid: { plus: "Spe", minus: "Atk" },
};

export interface TooltipMoveSlot {
  name: string;
  type?: string;
  pp?: number;
  maxPp?: number;
  disabled?: boolean;
  revealed: boolean;
}

export interface TooltipStatRow {
  stat: string; // e.g. "HP", "Atk", "Def", "SpA", "SpD", "Spe"
  label: string;
  value?: number;
  ev?: number;
  iv?: number;
  boost?: number;
  isPlus?: boolean;
  isMinus?: boolean;
}

export interface ExpMetrics {
  experience?: number | null;
  levelExperience?: number | null;
  nextLevelExperience?: number | null;
  expToNextLevel?: number | null;
  expProgressPercent?: number | null;
}

export function formatWeightKg(hectograms?: number | null): number | null {
  if (hectograms == null) return null;
  return Math.round((hectograms / 10) * 10) / 10;
}

export function computeExpMetrics(
  experience?: number | null,
  levelExperience?: number | null,
  nextLevelExperience?: number | null,
): ExpMetrics {
  if (experience == null) {
    return {};
  }

  // Max level (Level 100) or explicitly no next level
  if (nextLevelExperience === null) {
    return {
      experience,
      levelExperience: levelExperience ?? experience,
      nextLevelExperience: null,
      expToNextLevel: 0,
      expProgressPercent: 100,
    };
  }

  // Level bounds provided
  if (levelExperience != null && nextLevelExperience != null) {
    const span = Math.max(1, nextLevelExperience - levelExperience);
    const progress = Math.max(0, experience - levelExperience);
    const percent = Math.min(100, Math.max(0, Math.round((progress / span) * 100)));
    const toNext = Math.max(0, nextLevelExperience - experience);

    return {
      experience,
      levelExperience,
      nextLevelExperience,
      expToNextLevel: toNext,
      expProgressPercent: percent,
    };
  }

  return {
    experience,
  };
}

export interface MonTooltipViewModel {
  // Identity
  species: string;
  name?: string;
  level?: number | null;
  gender?: string | null;
  shiny?: boolean;
  types?: string[];
  teraType?: string | null;
  isTerastallized?: boolean;
  ball?: string | null;
  ownerLabel?: string | null;
  weightKg?: number | null;

  // Battle Vitals
  hp?: number | null;
  maxHp?: number | null;
  hpPercentage?: number | null;
  status?: string | null;
  isFainted?: boolean;

  // Experience
  experience?: number | null;
  levelExperience?: number | null;
  nextLevelExperience?: number | null;
  expToNextLevel?: number | null;
  expProgressPercent?: number | null;

  // Active Modifiers
  boosts: Array<{ stat: string; stage: number; label: string }>;
  conditions: string[];

  ability?: string | null;
  item?: string | null;
  nature?: string | null;
  natureModifiers?: NatureModifier | null;
  hiddenPowerType?: string | null;
  friendship?: number | null;

  // Forme / Transform / Dynamax
  isTransformed?: boolean;
  originalSpecies?: string | null;
  isDynamaxed?: boolean;

  // Moves
  moves: TooltipMoveSlot[];

  // Private build stats (null for public mons)
  stats?: TooltipStatRow[] | null;

  // Dedicated Base Summary (null for public mons)
  baseSummary?: MonTooltipViewModel | null;
}

const STAT_DISPLAY_NAMES: Record<string, string> = {
  hp: "HP",
  atk: "Atk",
  def: "Def",
  spa: "SpA",
  spd: "SpD",
  spe: "Spe",
  acc: "Acc",
  eva: "Eva",
};

export type SummaryStatKey = "hp" | "atk" | "def" | "spa" | "spd" | "spe";

const STAT_COLUMNS: Array<{ key: SummaryStatKey; label: string; statKey: Stat }> = [
  { key: "hp", label: "HP", statKey: "HP" },
  { key: "atk", label: "Atk", statKey: "Atk" },
  { key: "def", label: "Def", statKey: "Def" },
  { key: "spa", label: "SpA", statKey: "SpAtk" },
  { key: "spd", label: "SpD", statKey: "SpDef" },
  { key: "spe", label: "Spe", statKey: "Spe" },
];

function createStatsTable(
  summary?: MonBattleData["summary"],
  mon?: MonBattleData | null,
  natureMods?: NatureModifier | null,
): TooltipStatRow[] {
  return STAT_COLUMNS.map(({ key, label, statKey }) => {
    const value = (mon ? mon.stats?.[statKey] : undefined) ?? summary?.stats?.[key];
    const ev = summary?.evs?.[key];
    const iv = summary?.ivs?.[key];
    const boostVal = mon && key !== "hp" && mon.boosts ? mon.boosts[key as keyof BoostTable] : 0;

    return {
      stat: label,
      label,
      value,
      ev,
      iv,
      boost: boostVal || 0,
      isPlus: natureMods?.plus === label,
      isMinus: natureMods?.minus === label,
    };
  });
}

function formatLostItem(item: string): string {
  if (!item || item === "None") return "None";
  return `None (was ${item})`;
}

interface StateMonData {
  physical_appearance?: {
    name?: string;
    species?: string;
    gender?: string;
    shiny?: boolean;
  };
  volatile_data?: {
    forme_change?: string;
    transformed?: Array<{
      name?: string;
      species?: string;
    }>;
  };
  battle_appearances?: unknown[];
}

interface StatePlayerData {
  id?: string;
  mons?: StateMonData[];
}

function resolveAppearanceRef(
  state: BattleState,
  playerId: string,
  monIndex: number,
  mon?: StateMonData | null,
  sideIndex?: number,
): MonBattleAppearanceReference {
  try {
    const resolvedSideIndex = sideIndex ?? stateSelectors.sideForPlayer(state, playerId);
    const side = stateSelectors.side(state, resolvedSideIndex);
    const activeRef = side?.active?.find(
      (a) =>
        a != null &&
        a.player === playerId &&
        a.mon_index === monIndex,
    );
    if (activeRef) {
      return activeRef;
    }
  } catch {
    // Ignore error
  }

  const appearanceIndex = Math.max(0, (mon?.battle_appearances?.length || 1) - 1);
  return {
    player: playerId,
    mon_index: monIndex,
    battle_appearance_index: appearanceIndex,
  };
}

/**
 * Normalizes stat boost records into clean array of active (non-zero) stages.
 */
export function formatActiveBoosts(
  boosts?: BoostTable | Record<string, number> | null,
): Array<{ stat: string; stage: number; label: string }> {
  if (!boosts) return [];
  const result: Array<{ stat: string; stage: number; label: string }> = [];

  const entries = Object.entries(boosts);
  for (const [rawKey, val] of entries) {
    const stage = Number(val);
    if (!stage || isNaN(stage)) continue;
    const keyLower = rawKey.toLowerCase();
    const statName = STAT_DISPLAY_NAMES[keyLower] || rawKey;
    const sign = stage > 0 ? `+${stage}` : `${stage}`;
    result.push({
      stat: statName,
      stage,
      label: `${sign} ${statName}`,
    });
  }

  return result;
}

function cleanMonName(name: string): string {
  return name.toLowerCase().replace(/[^a-z0-9]/g, "");
}

function findMonInPlayer(
  player: StatePlayerData,
  targetName: string,
): { monIndex: number; mon: StateMonData } | null {
  const cleanTarget = cleanMonName(targetName);
  const monIndex = (player.mons || []).findIndex((m) => {
    const phys = m.physical_appearance;
    if (!phys) return false;
    const cleanPhysName = cleanMonName(phys.name || "");
    const cleanSpecies = cleanMonName(phys.species || "");
    if (cleanPhysName === cleanTarget || cleanSpecies === cleanTarget) {
      return true;
    }
    if (m.volatile_data?.forme_change) {
      const cleanForme = cleanMonName(m.volatile_data.forme_change);
      if (cleanForme === cleanTarget) return true;
    }
    if (m.volatile_data?.transformed) {
      const transPhys = m.volatile_data.transformed[0];
      const transSpecies = cleanMonName(transPhys?.species || "");
      const transName = cleanMonName(transPhys?.name || "");
      if (transSpecies === cleanTarget || transName === cleanTarget) {
        return true;
      }
    }
    return false;
  });

  if (monIndex !== -1 && player.mons) {
    return { monIndex, mon: player.mons[monIndex] };
  }
  return null;
}

function findMonInAllSides(
  state: BattleState,
  targetName: string,
): { playerId: string; monIndex: number; mon: StateMonData; sideIndex: number } | null {
  if (!state.field?.sides) return null;
  try {
    for (let sIdx = 0; sIdx < state.field.sides.length; sIdx++) {
      const players = stateSelectors.sidePlayers(state, sIdx);
      for (const p of players) {
        const found = findMonInPlayer(p as StatePlayerData, targetName);
        if (found) {
          return {
            playerId: p.id || "",
            monIndex: found.monIndex,
            mon: found.mon,
            sideIndex: sIdx,
          };
        }
      }
    }
  } catch {
    // Ignore error
  }
  return null;
}

/**
 * Resolves a player's private MonBattleData against BattleState to link to its live Mon reference.
 */
export function resolveMonRefForBattleData(
  state: BattleState,
  mon: MonBattleData,
): MonBattleAppearanceReference | null {
  const targetName = getMonDisplayName(mon);
  if (!targetName) return null;

  const match = findMonInAllSides(state, targetName);
  if (!match) return null;

  return resolveAppearanceRef(state, match.playerId, match.monIndex, match.mon, match.sideIndex);
}

/**
 * Converts a player's private MonBattleData into a standardized MonTooltipViewModel.
 * Accepts optional BattleState to resolve live battle-log-tracked conditions, transforms, and terastallization.
 */
export function monBattleDataToTooltip(
  mon: MonBattleData,
  state?: BattleState | null,
): MonTooltipViewModel {
  const summary = mon.summary;
  const species = mon.species || summary?.species || "Unknown";
  const name = getMonDisplayName(mon) || species;
  const level = summary?.level ?? 50;
  const gender = summary?.gender ?? null;
  const shiny = !!summary?.shiny;
  const types = (mon.types || []).map((t) => String(t));

  const hp = mon.hp;
  const maxHp = mon.max_hp;
  const hpPercentage = computeHpPercentage(hp, maxHp);
  const status = normalizeStatusCode(mon.status);
  const isFainted = hp <= 0 || status === "fnt";

  const nature = summary?.nature as Nature | undefined;
  const natureMods = nature ? NATURE_MODIFIERS[nature] || null : null;

  const weightKg = formatWeightKg(mon.weight ?? summary?.weight);
  const expMetrics = computeExpMetrics(
    summary?.experience,
    summary?.level_experience,
    summary?.next_level_experience,
  );

  // Inferred live battle state
  let isTransformed = false;
  let originalSpecies: string | undefined = undefined;
  let conditions: string[] = [];
  let activeTera: string | null = null;
  let isDynamaxed = false;
  let previousItem: string | null = null;

  if (state) {
    const resolvedRef = resolveMonRefForBattleData(state, mon);
    if (resolvedRef) {
      try {
        conditions = stateSelectors.monConditions(state, resolvedRef) || [];
      } catch {
        // Ignore error
      }

      try {
        previousItem = stateSelectors.monPreviousItem(state, resolvedRef);
      } catch {
        // Ignore error
      }

      try {
        isDynamaxed = Boolean(stateSelectors.monIsDynamaxed(state, resolvedRef));
      } catch {
        // Ignore error
      }

      try {
        const app = stateSelectors.monBattleAppearance(state, resolvedRef);
        if (app && "known" in app.terastallization && app.terastallization.known) {
          activeTera = String(app.terastallization.known);
        }
      } catch {
        // Ignore error
      }

      try {
        const m = stateSelectors.mon(state, resolvedRef);
        if (m?.volatile_data?.transformed) {
          isTransformed = true;
          originalSpecies = m.physical_appearance?.species;
        }
      } catch {
        // Ignore error
      }
    }
  } else {
    isTransformed = Boolean(summary?.species && mon.species && mon.species !== summary.species);
    originalSpecies = isTransformed ? summary?.species : undefined;
  }

  // Moves: all 4 moves with current PP / max PP
  const moves: TooltipMoveSlot[] = (mon.moves || []).map((m) => ({
    name: m.name,
    type: m.type,
    pp: m.pp,
    maxPp: m.max_pp,
    disabled: m.disabled,
    revealed: true,
  }));

  // Build stats table (HP, Atk, Def, SpA, SpD, Spe)
  const statsTable = createStatsTable(summary, mon, natureMods);

  const currentAbility = mon.ability || summary?.ability || null;

  let currentItem = "None";
  const wasItem = previousItem || summary?.item;
  if (mon.item !== undefined) {
    if (mon.item && mon.item !== "") {
      currentItem = mon.item;
    } else if (wasItem) {
      currentItem = formatLostItem(wasItem);
    }
  } else if (previousItem) {
    currentItem = formatLostItem(previousItem);
  } else if (summary?.item) {
    currentItem = summary.item;
  }

  let baseSummary: MonTooltipViewModel | null = null;
  if (summary) {
    const baseMoves: TooltipMoveSlot[] = (summary.moves || []).map((m) => {
      const match = mon.moves?.find(
        (lm) =>
          lm.name.toLowerCase() === m.name.toLowerCase() ||
          lm.id?.toLowerCase() === m.name.toLowerCase(),
      );
      const moveType = m.typ || match?.type;
      const moveMaxPp = m.max_pp || match?.max_pp || m.pp;
      return {
        name: m.name,
        type: moveType ? String(moveType) : undefined,
        pp: m.pp,
        maxPp: moveMaxPp,
        revealed: true,
      };
    });

    const baseStatsTable = createStatsTable(summary, null, natureMods);

    const summaryMaxHp = summary.stats?.hp || summary.hp;
    const summaryHp = Math.min(summary.hp, summaryMaxHp);
    const summaryHpPercentage = computeHpPercentage(summaryHp, summaryMaxHp);
    const summaryStatus = normalizeStatusCode(summary.status);
    const summaryIsFainted = summaryHp <= 0 || summaryStatus === "fnt";

    baseSummary = {
      species: summary.species,
      name: summary.name || summary.species,
      level: summary.level,
      gender: summary.gender,
      shiny: summary.shiny,
      types,
      teraType: summary.tera_type ? String(summary.tera_type) : null,
      isTerastallized: false,
      ball: summary.ball,
      ownerLabel: "Your Mon",
      weightKg: formatWeightKg(summary.weight),
      hp: summaryHp,
      maxHp: summaryMaxHp,
      hpPercentage: summaryHpPercentage,
      status: summaryStatus,
      isFainted: summaryIsFainted,
      ...expMetrics,
      boosts: [],
      conditions: [],
      ability: summary.ability,
      item: summary.item || "None",
      nature: nature || null,
      natureModifiers: natureMods,
      hiddenPowerType: summary.hidden_power_type ? String(summary.hidden_power_type) : null,
      friendship: summary.friendship ?? null,
      isTransformed: false,
      originalSpecies: null,
      isDynamaxed: false,
      moves: baseMoves,
      stats: baseStatsTable,
      baseSummary: null,
    };
  }

  const isTerastallized = Boolean(activeTera);
  const resolvedTeraType =
    activeTera || (summary?.tera_type ? String(summary.tera_type) : null);
  const ball = summary?.ball || null;

  return {
    species,
    name,
    level,
    gender,
    shiny,
    types,
    teraType: resolvedTeraType,
    isTerastallized,
    ball,
    ownerLabel: "Your Mon",
    weightKg,
    hp,
    maxHp,
    hpPercentage,
    status,
    isFainted,
    ...expMetrics,
    boosts: formatActiveBoosts(mon.boosts),
    conditions,
    ability: currentAbility,
    item: currentItem,
    nature: nature || null,
    natureModifiers: natureMods,
    hiddenPowerType: summary?.hidden_power_type ? String(summary.hidden_power_type) : null,
    friendship: summary?.friendship ?? null,
    isTransformed,
    originalSpecies,
    isDynamaxed,
    moves,
    stats: statsTable,
    baseSummary,
  };
}

function getUiMonInfo(uiMon: UiMon): { name: string; player: string; side?: number } {
  if ("Active" in uiMon) {
    return { name: uiMon.Active.name, player: uiMon.Active.player, side: uiMon.Active.side };
  }
  if ("Inactive" in uiMon) {
    return { name: uiMon.Inactive.name, player: uiMon.Inactive.player, side: undefined };
  }
  return { name: "", player: "", side: undefined };
}

/**
 * Resolves a UiMon into a MonBattleAppearanceReference against a BattleState.
 * Identifies the Pokémon by its player and name, then checks if it is currently
 * active on the field or inactive/fainted.
 */
export function resolveBattleMonRef(
  state: BattleState,
  uiMon: UiMon,
): MonBattleAppearanceReference | null {
  const { name: targetName, player: targetPlayer, side: sideHint } = getUiMonInfo(uiMon);

  if (!targetName) return null;

  let resolvedPlayerId = targetPlayer;
  let found: { monIndex: number; mon: StateMonData } | null = null;
  let resolvedSideIndex: number | undefined = sideHint;

  // 1. Try finding in the specified player's roster
  if (targetPlayer) {
    const p = stateSelectors.player(state, targetPlayer);
    if (p) {
      found = findMonInPlayer(p as StatePlayerData, targetName);
      if (found) {
        resolvedPlayerId = p.id || targetPlayer;
      }
    }
  }

  // 2. If not found and side hint exists, search players on that side
  if (!found && sideHint != null && state.field?.sides?.[sideHint]) {
    try {
      const players = stateSelectors.sidePlayers(state, sideHint);
      for (const p of players) {
        found = findMonInPlayer(p as StatePlayerData, targetName);
        if (found) {
          resolvedPlayerId = p.id || targetPlayer;
          break;
        }
      }
    } catch {
      // Ignore error
    }
  }

  // 3. Fallback: Search all players across all sides in the battle
  if (!found) {
    const match = findMonInAllSides(state, targetName);
    if (match) {
      resolvedPlayerId = match.playerId || targetPlayer;
      found = match;
      resolvedSideIndex = match.sideIndex;
    }
  }

  if (!found) return null;

  return resolveAppearanceRef(
    state,
    resolvedPlayerId,
    found.monIndex,
    found.mon,
    resolvedSideIndex,
  );
}

function makeEmptyPublicTooltip(species: string, player?: string): MonTooltipViewModel {
  return {
    species,
    ownerLabel: player ? `Player: ${player}` : null,
    boosts: [],
    conditions: [],
    moves: [],
    stats: null,
  };
}

/**
 * Converts a BattleState and UiMon reference into a public MonTooltipViewModel.
 * Strictly avoids leaking unrevealed private data (EVs/IVs, unrevealed moves, hidden ability/item).
 */
export function publicMonStateToTooltip(
  state: BattleState | null | undefined,
  uiMon: UiMon,
): MonTooltipViewModel | null {
  if (!state) return null;

  const monRef = resolveBattleMonRef(state, uiMon);
  const { name: targetName, player: targetPlayer } = getUiMonInfo(uiMon);
  const fallbackPlayer = targetPlayer;
  const fallbackName = targetName || "Mon";

  if (!monRef) {
    return makeEmptyPublicTooltip(fallbackName, fallbackPlayer);
  }

  try {
    const species = stateSelectors.monSpecies(state, monRef) || fallbackName;
    const phys = stateSelectors.monPhysicalAppearance(state, monRef);
    const level = stateSelectors.monLevel(state, monRef);
    const health = stateSelectors.monHealth(state, monRef);
    const rawStatus = stateSelectors.monStatus(state, monRef);
    const status = normalizeStatusCode(rawStatus);
    const isFainted = stateSelectors.monIsFainted(state, monRef);
    const ability = stateSelectors.monAbility(state, monRef);
    const rawBoosts = stateSelectors.monBoosts(state, monRef);
    const conditions = stateSelectors.monConditions(state, monRef) || [];
    const types = stateSelectors.monTypes(state, monRef, () => []) || [];

    const app = stateSelectors.monBattleAppearance(state, monRef);

    // Item: distinguished between unrevealed (null), known empty ("None"), or known item name
    let item: string | null = null;
    if (app && "known" in app.item) {
      if (app.item.known !== "") {
        item = app.item.known;
      } else if (
        app.previous_item &&
        "known" in app.previous_item &&
        app.previous_item.known !== ""
      ) {
        item = formatLostItem(app.previous_item.known);
      } else {
        item = "None";
      }
    }

    // Check Terastallization
    let teraType: string | null = null;
    let isTerastallized = false;
    if (app && "known" in app.terastallization && app.terastallization.known) {
      teraType = String(app.terastallization.known);
      isTerastallized = true;
    }

    // Health calculations
    let hp: number | null = null;
    let maxHp: number | null = null;
    let hpPercentage: number | null = null;

    if (health) {
      hp = health[0];
      maxHp = health[1];
      hpPercentage = computeHpPercentage(hp, maxHp);
    }

    // Known / revealed moves (only include moves that have actually been seen in battle)
    const knownMoves = stateSelectors.monMoves(state, monRef, false) || [];
    const moveSlots: TooltipMoveSlot[] = knownMoves.map((name) => ({
      name,
      revealed: true,
    }));

    const monName = phys?.name || species;

    let isTransformed = false;
    let originalSpecies: string | undefined = undefined;
    let isDynamaxed = false;
    try {
      isDynamaxed = Boolean(stateSelectors.monIsDynamaxed(state, monRef));
    } catch {
      // Ignore error
    }

    const m = stateSelectors.mon(state, monRef);
    if (m?.volatile_data?.transformed) {
      isTransformed = true;
      originalSpecies = m.physical_appearance?.species;
    }

    return {
      species,
      name: monName,
      level,
      gender: phys?.gender || null,
      shiny: !!phys?.shiny,
      types,
      teraType,
      isTerastallized,
      ball: null,
      ownerLabel: monRef.player ? `Player: ${monRef.player}` : null,
      hp,
      maxHp,
      hpPercentage,
      status,
      isFainted,
      boosts: formatActiveBoosts(rawBoosts),
      conditions,
      ability: ability || null,
      item: item || null,
      isTransformed,
      originalSpecies,
      isDynamaxed,
      moves: moveSlots,
      stats: null, // Private stats are never exposed publicly
    };
  } catch (err) {
    console.error("Failed to resolve public mon tooltip from state:", err);
    return makeEmptyPublicTooltip(fallbackName, fallbackPlayer);
  }
}
