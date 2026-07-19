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
