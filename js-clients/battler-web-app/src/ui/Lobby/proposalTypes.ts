export interface FormPlayer {
  id: string;
  controlType: "human" | "ai";
  playerType: "trainer" | "wild" | "protagonist";
  hasAffection: boolean;
  hasStrictBag: boolean;
  cannotMegaEvolve: boolean;
  cannotZMove: boolean;
  cannotDynamax: boolean;
  cannotTerastallize: boolean;
  monsCaught: number;
  // Wild options
  wildCatchable: boolean;
  wildEscapable: boolean;
  wildCanEscape: boolean;
  wildEncounterType: "Normal" | "Fishing";
  selectedTeam?: string;
}

export function createDefaultPlayer(id = ""): FormPlayer {
  return {
    id,
    controlType: "human",
    playerType: "trainer",
    hasAffection: false,
    hasStrictBag: false,
    cannotMegaEvolve: false,
    cannotZMove: false,
    cannotDynamax: false,
    cannotTerastallize: false,
    monsCaught: 0,
    wildCatchable: true,
    wildEscapable: true,
    wildCanEscape: true,
    wildEncounterType: "Normal",
  };
}

export interface TimerSettingsState {
  preset: "none" | "blitz" | "standard" | "custom";
  battleTimer: string;
  playerTimer: string;
  actionTimer: string;
  teamPreviewTimer: string;
  proposalTimeout: number;
}

export const TIMER_PRESETS = {
  blitz: {
    actionTimer: "10",
    actionWarnings: [5n],
    teamPreviewTimer: "15",
    teamPreviewWarnings: [5n],
    playerTimer: "",
    playerWarnings: [] as bigint[],
    battleTimer: "",
    battleWarnings: [] as bigint[],
  },
  standard: {
    actionTimer: "45",
    actionWarnings: [15n, 5n],
    teamPreviewTimer: "60",
    teamPreviewWarnings: [30n, 10n],
    playerTimer: "420",
    playerWarnings: [300n, 180n, 60n, 30n, 10n],
    battleTimer: "1200",
    battleWarnings: [600n, 300n, 60n],
  },
} as const;

