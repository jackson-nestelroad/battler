import type { BattleState, ConditionData } from "battler-state";
import { stateSelectors } from "battler-state";
import { toId } from "./dataTooltipFormatting";

export interface FormattedCondition {
  id: string;
  name: string;
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
  playerSideSubtitle?: string;
  foeSideSubtitle?: string;
  isSpectatorOrReplay: boolean;
}

export interface BattleConditionsViewModel extends SideLabelsInfo {
  fieldData: FormattedFieldConditions;
  playerData: FormattedSideConditions;
  foeData: FormattedSideConditions;
}

export function formatCondition(name: string, _data?: ConditionData | null): FormattedCondition {
  return {
    id: toId(name),
    name,
    displayText: name,
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
  for (const name of Object.keys(rawConditions)) {
    if (name.endsWith("Terrain")) {
      terrain = name;
    } else {
      otherConditions.push(formatCondition(name));
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
    Object.entries(slotMap || {}).map(([name]) => ({
      ...formatCondition(name),
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

  const conditions = Object.keys(side.conditions || {}).map((name) =>
    formatCondition(name),
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
        const foeSide = battleState.field.sides[otherIdx];
        const foeSideName = foeSide?.name;
        return {
          playerSideIndex: sideIdx,
          foeSideIndex: otherIdx,
          playerSideLabel: "Your Side",
          foeSideLabel: "Foe Side",
          foeSideSubtitle:
            foeSideName && foeSideName !== "Foe Side" && foeSideName !== "Side 2"
              ? foeSideName
              : undefined,
          isSpectatorOrReplay: false,
        };
      }
    } catch {
      // Not a player on any side (spectator or replay)
    }
  }

  const side0 = battleState.field.sides[0];
  const side1 = battleState.field.sides[1];
  const side0Name = side0?.name;
  const side1Name = side1?.name;

  return {
    playerSideIndex: 0,
    foeSideIndex: 1,
    playerSideLabel: "Side 1",
    foeSideLabel: "Side 2",
    playerSideSubtitle: side0Name && side0Name !== "Side 1" ? side0Name : undefined,
    foeSideSubtitle: side1Name && side1Name !== "Side 2" ? side1Name : undefined,
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
