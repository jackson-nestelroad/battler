export interface ChoiceModifiers {
  mega?: boolean;
  zmove?: boolean;
  ultra?: boolean;
  dyna?: boolean;
  tera?: boolean;
}

export const ChoiceBuilder = {
  /**
   * Constructs a move choice string e.g. "move 0, 1, mega" or "move 0, -1, tera, dyna".
   */
  move(moveIndex: number, targetVal?: number | null, modifiers?: ChoiceModifiers): string {
    const parts: string[] = [`move ${moveIndex}`];
    if (typeof targetVal === "number") {
      parts.push(targetVal.toString());
    }
    if (modifiers?.mega) parts.push("mega");
    if (modifiers?.zmove) parts.push("zmove");
    if (modifiers?.ultra) parts.push("ultra");
    if (modifiers?.dyna) parts.push("dyna");
    if (modifiers?.tera) parts.push("tera");

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
   * Constructs a team order choice string e.g. "team 1, 2, 3".
   */
  team(positions: number[]): string {
    return `team ${positions.join(", ")}`;
  },
};
