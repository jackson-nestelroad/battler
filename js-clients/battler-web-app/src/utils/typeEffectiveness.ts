import { stateSelectors, type BattleState } from "battler-state";
import type {
  MonMoveSlotData,
  PlayerBattleData,
  SpeciesData,
  TypeChartData,
} from "battler-types";
import { getCachedResource } from "../hooks/useDataStore";
import { formatMultiplier, getEffectiveness } from "../hooks/useTypeChart";
import {
  getActiveRefFromState,
  getMonFromActiveRef,
  isMonFaintedInState,
} from "./battleState";
import { getMonByActivePosition } from "./monHelpers";
import { getActivePerPlayer, getOpposingSide } from "./targeting";

/**
 * Extracts types array from SpeciesData (primary_type and optional secondary_type).
 */
export function getSpeciesTypesFromData(data: SpeciesData | null | undefined): string[] {
  if (!data) return [];
  const types: string[] = [String(data.primary_type)];
  if (data.secondary_type) {
    types.push(String(data.secondary_type));
  }
  return types;
}

/**
 * Looks up cached species data and returns its types.
 */
export function getCachedSpeciesTypes(speciesName: string): string[] {
  if (!speciesName) return [];
  const data = getCachedResource("species", speciesName) as SpeciesData | undefined;
  return getSpeciesTypesFromData(data);
}

/**
 * Checks if a move is eligible for type effectiveness calculation
 * (must be non-Status and have a defined type).
 */
export function canCalculateEffectiveness(
  move?: MonMoveSlotData | null,
): boolean {
  return Boolean(move && move.category !== "Status" && move.type);
}

/**
 * Resolves the active types of a target Mon on the field.
 *
 * Checks in order:
 * 1. Terastallization (app.terastallization.known) -> overrides to single tera type.
 * 2. In-battle volatile types via stateSelectors.monTypes(state, monRef, getSpeciesTypes).
 * 3. Base species typing fallback from state or playerData.
 */
export function resolveTargetTypes(
  battleState: BattleState | null | undefined,
  playerData: PlayerBattleData | null | undefined,
  sideIdx: number,
  pos: number,
  customGetSpeciesTypes?: (species: string) => string[],
): string[] {
  const getTypes = customGetSpeciesTypes || getCachedSpeciesTypes;

  if (battleState) {
    const { side, activeRef } = getActiveRefFromState(battleState, sideIdx, pos);

    if (activeRef) {
      try {
        // 1. Terastallization check via selector
        const app = stateSelectors.monBattleAppearance(battleState, activeRef);
        if (app && "known" in app.terastallization && app.terastallization.known) {
          return [String(app.terastallization.known)];
        }
      } catch {
        // Ignore error and fall through
      }

      // 1b. Direct fallback check on appearance object (e.g. for mocked states)
      try {
        const stateMon = getMonFromActiveRef(side, activeRef);
        const rawApp = stateMon?.battle_appearances?.[activeRef.battle_appearance_index ?? 0] as any;
        const appearanceObj =
          rawApp && typeof rawApp === "object"
            ? "active" in rawApp && rawApp.active && typeof rawApp.active === "object"
              ? "primary_battle_appearance" in rawApp.active
                ? rawApp.active.primary_battle_appearance
                : rawApp.active
              : "inactive" in rawApp
                ? rawApp.inactive
                : rawApp
            : null;

        if (appearanceObj?.terastallization?.known) {
          return [String(appearanceObj.terastallization.known)];
        }
      } catch {
        // Ignore error and fall through
      }

      try {
        // 2. Volatile / in-battle types via stateSelectors
        const volatileTypes = stateSelectors.monTypes(battleState, activeRef, (species) =>
          getTypes(species),
        );
        if (volatileTypes && volatileTypes.length > 0) {
          return volatileTypes.map(String);
        }
      } catch {
        // Ignore error and fall through
      }

      // 3. Fallback from player mon in state
      try {
        const stateMon = getMonFromActiveRef(side, activeRef) as any;
        if (stateMon && "types" in stateMon && Array.isArray(stateMon.types) && stateMon.types.length > 0) {
          return stateMon.types.map(String);
        }
        const species = stateMon?.physical_appearance?.species;
        if (species) {
          const fallback = getTypes(species);
          if (fallback.length > 0) {
            return fallback;
          }
        }
      } catch {
        // Ignore error and fall through
      }
    }
  }

  // 4. Fallback from playerData if it's the player's own side
  if (playerData && sideIdx === (playerData.side ?? 0)) {
    const myMon = getMonByActivePosition(playerData, pos) as any;
    if (myMon && "types" in myMon && Array.isArray(myMon.types) && myMon.types.length > 0) {
      return myMon.types.map(String);
    }
    if (myMon?.species) {
      const fallback = getTypes(myMon.species);
      if (fallback.length > 0) {
        return fallback;
      }
    }
  }

  return [];
}

/**
 * Calculates type effectiveness multiplier of an attacking move against a defender's types.
 */
export function calculateTargetEffectiveness(
  typeChart: TypeChartData | null,
  moveType: string,
  targetTypes: string[],
): number {
  if (!typeChart || !moveType || !targetTypes || targetTypes.length === 0) return 1;
  let mult = 1;
  for (const defType of targetTypes) {
    mult *= getEffectiveness(typeChart, moveType, defType);
  }
  return mult;
}

/**
 * Maps a damage multiplier to its centralized CSS class name.
 */
export function getMultiplierClass(mult: number): string {
  if (mult >= 4) return "multQuad";
  if (mult >= 2) return "multSuper";
  if (mult === 0) return "multImmune";
  if (mult <= 0.25) return "multQuadResist";
  if (mult <= 0.5) return "multResist";
  return "multNeutral";
}

/**
 * Formats a consistent type effectiveness comparison string in the format:
 * "$TYPE vs $TYPE: 1×"
 *
 * Examples:
 * - formatEffectivenessComparison("Ice", ["Normal", "Fire", "Water"], 0.25) => "Ice vs Normal/Fire/Water: ¼×"
 * - formatEffectivenessComparison("Grass", "Water", 2) => "Grass vs Water: 2×"
 * - formatEffectivenessComparison(undefined, "Psychic", 1) => "vs Psychic: 1×"
 * - formatEffectivenessComparison(undefined, undefined, 1) => "1×"
 */
export function formatEffectivenessComparison(
  attackerType: string | undefined | null,
  defenderTypes: string | string[] | undefined | null,
  mult: number,
): string {
  const multText = formatMultiplier(mult);
  const defStr = Array.isArray(defenderTypes)
    ? defenderTypes.filter(Boolean).join("/")
    : defenderTypes || "";

  if (attackerType && defStr) {
    return `${attackerType} vs ${defStr}: ${multText}`;
  }
  if (attackerType) {
    return `${attackerType}: ${multText}`;
  }
  if (defStr) {
    return `vs ${defStr}: ${multText}`;
  }
  return multText;
}


/**
 * Resolves the typing of the opposing active Mon if there is exactly one
 * unambiguous active foe on the field (e.g. Singles, or multi-battle where only 1 foe is alive).
 * Returns null if multiple foes or no active foes exist.
 */
export function getSingleOpposingTargetTypes(
  battleState: BattleState | null | undefined,
  playerData: PlayerBattleData | null | undefined,
  battleType?: string | null,
  activeRequestsCount: number = 1,
): string[] | null {
  const activePerPlayer = getActivePerPlayer(battleType, activeRequestsCount);
  const playerSide = playerData?.side ?? 0;
  const foeSide = getOpposingSide(playerSide);

  const activeFoePositions: number[] = [];
  for (let pos = 0; pos < activePerPlayer; pos++) {
    if (!isMonFaintedInState(battleState, foeSide, pos, playerData)) {
      activeFoePositions.push(pos);
    }
  }

  if (activeFoePositions.length === 1) {
    const foePos = activeFoePositions[0];
    const types = resolveTargetTypes(battleState, playerData, foeSide, foePos);
    return types.length > 0 ? types : null;
  }

  return null;
}
