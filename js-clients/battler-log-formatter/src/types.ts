import { en } from "../locales/en.js";

export type LogTemplateKey = keyof typeof en.logs;

export enum LogCategory {
  Primary = "primary",     // Main actions (Moves, Switches, Faints)
  Secondary = "secondary", // Modifiers to primary actions (Critical hit, Super effective)
  Hint = "hint",           // Side-effects, abilities, weather, etc.
  Ability = "ability",     // Standalone ability activations
}

export interface ContextVar {
  text: string;
  id?: string; // used for tying UI elements to specific game entities
  noAutoCapitalize?: boolean;
}

export type ContextValue = string | number | ContextVar | (string | ContextVar)[];

export type LogContext = Record<string, ContextValue> & { count?: number };

export type ExtractVariables<T extends string> =
  T extends `${string}{{${infer Var}}}${infer Rest}`
    ? Var | ExtractVariables<Rest>
    : never;

export type RequiredContext<K extends LogTemplateKey> = 
  ExtractVariables<typeof en.logs[K]> extends never
    ? { count?: number }
    : Record<ExtractVariables<typeof en.logs[K]>, ContextValue> & { count?: number };

import type { EffectData } from "battler-state";

export interface MappedLogBase<K extends LogTemplateKey> {
  key: K;
  category: LogCategory;
  context: RequiredContext<K>;
  effect?: EffectData;
}

export type AnyMappedLog = {
  [K in LogTemplateKey]: MappedLogBase<K>
}[LogTemplateKey];

export interface MapperOptions {
  localPlayerId?: string;
  foeFormat?: "standard" | "withPlayer" | "possessive";
  allyFormat?: "standard" | "possessive";
  healthFormat?: "fraction" | "percentage";
}
