import { useEffect, useState } from "react";
import type { BattleState } from "battler-state";
import type { PlayerBattleData } from "battler-types";
import { fetchSpeciesBatch, getCachedResource } from "./useDataStore";
import { getMonFromActiveRef } from "../utils/battleState";

/**
 * Extracts all active species names currently on the field from both sides.
 */
export function extractActiveFieldSpecies(
  battleState?: BattleState | null,
  playerData?: PlayerBattleData | null,
): string[] {
  const speciesSet = new Set<string>();

  // 1. From player's active mons
  if (playerData?.mons) {
    for (const m of playerData.mons) {
      if (m.active && m.species) {
        speciesSet.add(m.species);
      }
    }
  }

  // 2. From battleState field sides
  if (battleState?.field?.sides) {
    for (const side of battleState.field.sides) {
      if (!side.active) continue;
      for (const activeRef of side.active) {
        if (!activeRef) continue;
        const mon = getMonFromActiveRef(side, activeRef);
        const species = mon?.physical_appearance?.species;
        if (species) {
          speciesSet.add(species);
        }
      }
    }
  }

  return Array.from(speciesSet);
}

/**
 * Pre-warms the species cache for all active Mons currently on the field
 * across both sides of the battle.
 */
export function usePreloadFieldSpecies(
  battleState?: BattleState | null,
  playerData?: PlayerBattleData | null,
): boolean {
  const [loaded, setLoaded] = useState(false);

  useEffect(() => {
    const speciesList = extractActiveFieldSpecies(battleState, playerData);

    if (speciesList.length === 0) {
      setLoaded(true);
      return;
    }

    // Determine which species are not yet cached
    const missing = speciesList.filter(
      (s) => !getCachedResource("species", s),
    );

    if (missing.length === 0) {
      setLoaded(true);
      return;
    }

    let isMounted = true;
    fetchSpeciesBatch(missing).then(() => {
      if (isMounted) {
        setLoaded(true);
      }
    });

    return () => {
      isMounted = false;
    };
  }, [battleState, playerData]);

  return loaded;
}
