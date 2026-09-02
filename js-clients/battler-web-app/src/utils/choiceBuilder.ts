export interface ChoiceModifiers {
  mega?: boolean;
  zmove?: boolean;
  ultra?: boolean;
  dyna?: boolean;
  tera?: boolean;
}

export type ModifierKey = keyof ChoiceModifiers;

export type MoveModifierFlag =
  | "can_mega_evolve"
  | "can_terastallize"
  | "can_z_move"
  | "can_dynamax"
  | "can_ultra_burst";

export interface ModifierConfig {
  key: ModifierKey;
  label: string;
  requestFlag: MoveModifierFlag;
}

export const CHOICE_MODIFIER_CONFIGS: Record<ModifierKey, ModifierConfig> = {
  mega: { key: "mega", label: "Mega", requestFlag: "can_mega_evolve" },
  tera: { key: "tera", label: "Tera", requestFlag: "can_terastallize" },
  zmove: { key: "zmove", label: "Z-Move", requestFlag: "can_z_move" },
  dyna: { key: "dyna", label: "Dynamax", requestFlag: "can_dynamax" },
  ultra: { key: "ultra", label: "Ultra", requestFlag: "can_ultra_burst" },
};

// Serialization order (used for choice strings)
export const CHOICE_MODIFIER_KEYS: ModifierKey[] = ["mega", "zmove", "ultra", "dyna", "tera"];

// UI rendering order
export const UI_MODIFIER_KEYS: ModifierKey[] = ["mega", "tera", "zmove", "dyna", "ultra"];

export const ChoiceBuilder = {
  /**
   * Constructs a move choice string e.g. "move 0, 1, mega" or "move 0, -1, tera, dyna".
   */
  move(moveIndex: number, targetVal?: number | null, modifiers?: ChoiceModifiers): string {
    const parts: string[] = [`move ${moveIndex}`];
    if (typeof targetVal === "number") {
      parts.push(targetVal.toString());
    }
    for (const mod of CHOICE_MODIFIER_KEYS) {
      if (modifiers?.[mod]) parts.push(mod);
    }

    return parts.join(", ");
  },

  /**
   * Constructs a switch choice string e.g. "switch 2".
   */
  switch(playerTeamPosition: number): string {
    return `switch ${playerTeamPosition}`;
  },

  /**
   * Constructs a shift choice string e.g. "shift".
   */
  shift(): string {
    return "shift";
  },

  /**
   * Constructs a pass choice string e.g. "pass".
   */
  pass(): string {
    return "pass";
  },

  /**
   * Constructs a team order choice string e.g. "team 0 1 2".
   */
  team(positions: number[]): string {
    return `team ${positions.join(" ")}`;
  },
};
