export type MoveEffectType = "status" | "volatile_status" | "boost" | "heal";

export type MoveEffectSubject = "user" | "target" | "allies";

export interface FormattedMoveEffect {
  /**
   * Complete natural localized sentence.
   * e.g., "10% chance to paralyze the target."
   * e.g., "Paralyzes the target."
   * e.g., "Lowers the user's Defense and Sp. Def by 1 stage."
   * e.g., "Restores 50% of the user's HP."
   */
  readonly text: string;

  /**
   * Raw chance percentage string if applicable, e.g. "10%", "30%", "100%".
   * Undefined for guaranteed hit_effect or user_effect without user_effect_chance.
   */
  readonly chance?: string;

  /**
   * Core effect category.
   */
  readonly type: MoveEffectType;

  /**
   * Target subject receiving the effect.
   */
  readonly subject: MoveEffectSubject;
}
