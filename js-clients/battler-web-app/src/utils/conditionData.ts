import type { BattleState, ConditionData } from "battler-state";
import { stateSelectors } from "battler-state";

export interface FormattedCondition {
  id: string;
  name: string;
  layers?: number;
  displayText: string;
}

export interface FormattedSlotCondition extends FormattedCondition {
  slotIndex: number;
  slotLabel: string;
}

export interface FormattedFieldConditions {
  weather: string | null;
  terrain: string | null;
  otherConditions: FormattedCondition[];
  allCount: number;
  summaryText: string;
}

export interface FormattedSideConditions {
  sideIndex: number;
  sideLabel: string;
  conditions: FormattedCondition[];
  slotConditions: FormattedSlotCondition[];
  allCount: number;
}

export interface SideLabelsInfo {
  playerSideIndex: number;
  foeSideIndex: number;
  playerSideLabel: string;
  foeSideLabel: string;
  isSpectatorOrReplay: boolean;
}

export interface BattleConditionsViewModel extends SideLabelsInfo {
  fieldData: FormattedFieldConditions;
  playerData: FormattedSideConditions;
  foeData: FormattedSideConditions;
}

function normalizeKey(str: string): string {
  return str.toLowerCase().replace(/[^a-z0-9]/g, "");
}

export function formatCondition(name: string, data?: ConditionData | null): FormattedCondition {
  let layers: number | undefined;
  let displayText = name;

  const rawLayers = data?.data?.layers;
  if (rawLayers) {
    const parsed = parseInt(rawLayers, 10);
    if (!isNaN(parsed) && parsed > 0) {
      layers = parsed;
      displayText = `${name} (${parsed} layer${parsed === 1 ? "" : "s"})`;
    }
  }

  return {
    id: normalizeKey(name),
    name,
    layers,
    displayText,
  };
}

export function extractFieldConditions(battleState?: BattleState | null): FormattedFieldConditions {
  if (!battleState?.field) {
    return {
      weather: null,
      terrain: null,
      otherConditions: [],
      allCount: 0,
      summaryText: "Clear",
    };
  }

  const rawWeather = battleState.field.weather;
  const weather =
    rawWeather && rawWeather !== "Clear" && rawWeather !== "None" ? rawWeather : null;

  let terrain: string | null = null;
  const otherConditions: FormattedCondition[] = [];

  const rawConditions = battleState.field.conditions || {};
  for (const [name, condData] of Object.entries(rawConditions)) {
    if (name.endsWith("Terrain")) {
      terrain = name;
    } else {
      otherConditions.push(formatCondition(name, condData));
    }
  }

  // Weather and Terrain are the only primary named conditions displayed on the Field chip.
  // Other environmental effects (Trick Room, Gravity, etc.) are counted in (+N).
  const primaryField: string[] = [];
  if (weather) primaryField.push(weather);
  if (terrain) primaryField.push(terrain);

  let summaryText = primaryField.join(" / ") || "Clear";
  if (otherConditions.length > 0) {
    summaryText = `${summaryText} (+${otherConditions.length})`;
  }

  const allCount = primaryField.length + otherConditions.length;

  return {
    weather,
    terrain,
    otherConditions,
    allCount,
    summaryText,
  };
}

export function extractSlotConditions(
  battleState?: BattleState | null,
  sideIndex: number = 0,
): FormattedSlotCondition[] {
  const side = battleState?.field?.sides?.[sideIndex];
  if (!side?.slot_conditions) return [];

  return side.slot_conditions.flatMap((slotMap, slotIdx) =>
    Object.entries(slotMap || {}).map(([name, condData]) => ({
      ...formatCondition(name, condData),
      slotIndex: slotIdx,
      slotLabel: `Slot ${slotIdx + 1}`,
    })),
  );
}

export function extractSideConditions(
  battleState?: BattleState | null,
  sideIndex: number = 0,
  sideLabel: string = "",
): FormattedSideConditions {
  const side = battleState?.field?.sides?.[sideIndex];
  if (!side) {
    return {
      sideIndex,
      sideLabel,
      conditions: [],
      slotConditions: [],
      allCount: 0,
    };
  }

  const conditions = Object.entries(side.conditions || {}).map(([name, condData]) =>
    formatCondition(name, condData),
  );

  const slotConditions = extractSlotConditions(battleState, sideIndex);
  const allCount = conditions.length + slotConditions.length;

  return {
    sideIndex,
    sideLabel,
    conditions,
    slotConditions,
    allCount,
  };
}

export function resolveSideLabels(
  battleState?: BattleState | null,
  playerId?: string | null,
): SideLabelsInfo {
  if (!battleState?.field?.sides?.length) {
    return {
      playerSideIndex: 0,
      foeSideIndex: 1,
      playerSideLabel: "Your Side",
      foeSideLabel: "Foe Side",
      isSpectatorOrReplay: false,
    };
  }

  if (playerId) {
    try {
      const sideIdx = stateSelectors.sideForPlayer(battleState, playerId);
      if (sideIdx !== -1 && sideIdx < battleState.field.sides.length) {
        const otherIdx = sideIdx === 0 ? 1 : 0;
        return {
          playerSideIndex: sideIdx,
          foeSideIndex: otherIdx,
          playerSideLabel: "Your Side",
          foeSideLabel: "Foe Side",
          isSpectatorOrReplay: false,
        };
      }
    } catch {
      // Not a player on any side (spectator or replay)
    }
  }

  const side0 = battleState.field.sides[0];
  const side1 = battleState.field.sides[1];
  const side0Name = side0?.name || "Side 1";
  const side1Name = side1?.name || "Side 2";

  return {
    playerSideIndex: 0,
    foeSideIndex: 1,
    playerSideLabel: side0Name,
    foeSideLabel: side1Name,
    isSpectatorOrReplay: true,
  };
}

export function extractAllBattleConditions(
  battleState?: BattleState | null,
  playerId?: string | null,
): BattleConditionsViewModel {
  const sideLabels = resolveSideLabels(battleState, playerId);
  const fieldData = extractFieldConditions(battleState);
  const playerData = extractSideConditions(
    battleState,
    sideLabels.playerSideIndex,
    sideLabels.playerSideLabel,
  );
  const foeData = extractSideConditions(
    battleState,
    sideLabels.foeSideIndex,
    sideLabels.foeSideLabel,
  );

  return {
    ...sideLabels,
    fieldData,
    playerData,
    foeData,
  };
}
