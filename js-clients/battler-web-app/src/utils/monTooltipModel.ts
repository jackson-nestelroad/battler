import type { BattleState, UiMon, MonBattleAppearanceReference } from "battler-state";
import { stateSelectors } from "battler-state";
import type { BoostTable, MonBattleData, Nature, Stat } from "battler-types";
import { formatStatusBadge } from "./monHelpers";

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
  if (hectograms === undefined || hectograms === null) return null;
  return Math.round((hectograms / 10) * 10) / 10;
}

export function computeExpMetrics(
  experience?: number | null,
  levelExperience?: number | null,
  nextLevelExperience?: number | null,
): ExpMetrics {
  if (experience === undefined || experience === null) {
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
  if (levelExperience !== undefined && levelExperience !== null && nextLevelExperience !== undefined) {
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
  player: any,
  targetName: string,
): { monIndex: number; mon: any } | null {
  const cleanTarget = cleanMonName(targetName);
  const monIndex = (player.mons || []).findIndex((m: any) => {
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

  if (monIndex !== -1) {
    return { monIndex, mon: player.mons[monIndex] };
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
  const targetName = mon.summary?.name || mon.species;
  if (!targetName) return null;

  if (state.field?.sides) {
    for (let sIdx = 0; sIdx < state.field.sides.length; sIdx++) {
      const players = stateSelectors.sidePlayers(state, sIdx);
      for (const p of players) {
        const found = findMonInPlayer(p, targetName);
        if (found) {
          const resolvedPlayerId = p.id || "";
          const { monIndex } = found;

          const side = stateSelectors.side(state, sIdx);
          const activeRef = side?.active?.find(
            (a) =>
              a !== null &&
              a !== undefined &&
              a.player === resolvedPlayerId &&
              a.mon_index === monIndex,
          );
          if (activeRef) {
            return activeRef;
          }

          const pMon = p.mons?.[monIndex];
          const appearanceIndex = Math.max(0, (pMon?.battle_appearances?.length || 1) - 1);
          return {
            player: resolvedPlayerId,
            mon_index: monIndex,
            battle_appearance_index: appearanceIndex,
          };
        }
      }
    }
  }

  return null;
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
  const name = summary?.name || mon.species || species;
  const level = summary?.level ?? 50;
  const gender = summary?.gender ?? null;
  const shiny = !!summary?.shiny;
  const types = (mon.types || []).map((t) => String(t));

  const hp = mon.hp;
  const maxHp = mon.max_hp;
  const hpPercentage = maxHp > 0 ? Math.round((Math.max(0, hp) / maxHp) * 100) : 0;
  const isFainted = hp <= 0 || mon.status?.toLowerCase() === "fnt";

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
  const statKeys: Array<{ key: keyof typeof summary.stats; label: string; statKey: Stat }> = [
    { key: "hp", label: "HP", statKey: "HP" },
    { key: "atk", label: "Atk", statKey: "Atk" },
    { key: "def", label: "Def", statKey: "Def" },
    { key: "spa", label: "SpA", statKey: "SpAtk" },
    { key: "spd", label: "SpD", statKey: "SpDef" },
    { key: "spe", label: "Spe", statKey: "Spe" },
  ];

  const statsTable: TooltipStatRow[] = statKeys.map(({ key, label, statKey }) => {
    const value = mon.stats?.[statKey] ?? summary?.stats?.[key];
    const ev = summary?.evs?.[key];
    const iv = summary?.ivs?.[key];
    const boostVal = key !== "hp" && mon.boosts ? mon.boosts[key as keyof BoostTable] : 0;

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

  const currentAbility = mon.ability || summary?.ability || null;

  const currentItem =
    mon.item !== undefined
      ? mon.item && mon.item !== ""
        ? mon.item
        : previousItem
          ? `None (was ${previousItem})`
          : summary?.item
            ? `None (was ${summary.item})`
            : "None"
      : previousItem
        ? `None (was ${previousItem})`
        : summary?.item
          ? summary.item
          : "None";

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

    const baseStatsTable: TooltipStatRow[] = statKeys.map(({ key, label }) => {
      const value = summary.stats?.[key];
      const ev = summary.evs?.[key];
      const iv = summary.ivs?.[key];
      return {
        stat: label,
        label,
        value,
        ev,
        iv,
        boost: 0,
        isPlus: natureMods?.plus === label,
        isMinus: natureMods?.minus === label,
      };
    });

    const summaryMaxHp = summary.stats?.hp || summary.hp;
    const summaryHp = Math.min(summary.hp, summaryMaxHp);
    const summaryHpPercentage =
      summaryMaxHp > 0
        ? Math.min(100, Math.max(0, Math.round((Math.max(0, summaryHp) / summaryMaxHp) * 100)))
        : 0;
    const summaryStatus = summary.status ? formatStatusBadge(summary.status)?.code || summary.status : null;
    const summaryIsFainted =
      summaryHp === 0 || summaryStatus?.toLowerCase() === "fnt";

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
      experience: expMetrics.experience,
      levelExperience: expMetrics.levelExperience,
      nextLevelExperience: expMetrics.nextLevelExperience,
      expToNextLevel: expMetrics.expToNextLevel,
      expProgressPercent: expMetrics.expProgressPercent,
      boosts: [],
      conditions: [],
      ability: summary.ability,
      item: summary.item ? summary.item : "None",
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
    name: name || species,
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
    status: mon.status ? formatStatusBadge(mon.status)?.code || mon.status : null,
    isFainted,
    experience: expMetrics.experience,
    levelExperience: expMetrics.levelExperience,
    nextLevelExperience: expMetrics.nextLevelExperience,
    expToNextLevel: expMetrics.expToNextLevel,
    expProgressPercent: expMetrics.expProgressPercent,
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

/**
 * Resolves a UiMon into a MonBattleAppearanceReference against a BattleState.
 * Identifies the Pokémon by its player and name, then checks if it is currently
 * active on the field or inactive/fainted.
 */
export function resolveBattleMonRef(
  state: BattleState,
  uiMon: UiMon,
): MonBattleAppearanceReference | null {
  const targetName =
    "Active" in uiMon ? uiMon.Active.name : "Inactive" in uiMon ? uiMon.Inactive.name : "";
  const targetPlayer =
    "Active" in uiMon ? uiMon.Active.player : "Inactive" in uiMon ? uiMon.Inactive.player : "";
  const sideHint = "Active" in uiMon ? uiMon.Active.side : undefined;

  if (!targetName) return null;

  let resolvedPlayerId = targetPlayer;
  let found: { monIndex: number; mon: any } | null = null;

  // 1. Try finding in the specified player's roster
  if (targetPlayer) {
    const p = stateSelectors.player(state, targetPlayer);
    if (p) {
      found = findMonInPlayer(p, targetName);
      if (found) {
        resolvedPlayerId = p.id || targetPlayer;
      }
    }
  }

  // 2. If not found and side hint exists, search players on that side
  if (!found && sideHint !== undefined && state.field?.sides?.[sideHint]) {
    try {
      const players = stateSelectors.sidePlayers(state, sideHint);
      for (const p of players) {
        found = findMonInPlayer(p, targetName);
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
  if (!found && state.field?.sides) {
    try {
      for (let sIdx = 0; sIdx < state.field.sides.length; sIdx++) {
        const players = stateSelectors.sidePlayers(state, sIdx);
        for (const p of players) {
          found = findMonInPlayer(p, targetName);
          if (found) {
            resolvedPlayerId = p.id || targetPlayer;
            break;
          }
        }
        if (found) break;
      }
    } catch {
      // Ignore error
    }
  }

  if (!found) return null;

  const { monIndex, mon } = found;

  // Check if this exact Pokémon is currently active on its side
  try {
    const sideIndex = stateSelectors.sideForPlayer(state, resolvedPlayerId);
    const side = stateSelectors.side(state, sideIndex);
    const activeRef = side?.active.find(
      (a) =>
        a !== null &&
        a !== undefined &&
        a.player === resolvedPlayerId &&
        a.mon_index === monIndex,
    );
    if (activeRef) {
      return activeRef;
    }
  } catch {
    // Ignore error
  }

  // Pokémon is currently not active on the field (it is benched or fainted)
  const appearanceIndex = Math.max(0, mon.battle_appearances.length - 1);
  return {
    player: resolvedPlayerId,
    mon_index: monIndex,
    battle_appearance_index: appearanceIndex,
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
  const fallbackPlayer = "Active" in uiMon ? uiMon.Active.player : "Inactive" in uiMon ? uiMon.Inactive.player : "";
  const fallbackName = "Active" in uiMon ? uiMon.Active.name : "Inactive" in uiMon ? uiMon.Inactive.name : "Mon";

  if (!monRef) {
    return {
      species: fallbackName,
      ownerLabel: fallbackPlayer ? `Player: ${fallbackPlayer}` : null,
      boosts: [],
      conditions: [],
      moves: [],
      stats: null,
    };
  }

  try {
    const species = stateSelectors.monSpecies(state, monRef) || fallbackName;
    const phys = stateSelectors.monPhysicalAppearance(state, monRef);
    const level = stateSelectors.monLevel(state, monRef);
    const health = stateSelectors.monHealth(state, monRef);
    const rawStatus = stateSelectors.monStatus(state, monRef);
    const status = rawStatus ? formatStatusBadge(rawStatus)?.code || rawStatus : null;
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
        item = `None (was ${app.previous_item.known})`;
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
      hpPercentage = maxHp > 0 ? Math.round((Math.max(0, hp) / maxHp) * 100) : 0;
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
    return {
      species: fallbackName,
      ownerLabel: fallbackPlayer ? `Player: ${fallbackPlayer}` : null,
      boosts: [],
      conditions: [],
      moves: [],
      stats: null,
    };
  }
}
