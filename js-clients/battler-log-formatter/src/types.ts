import { en } from "../locales/en.js";
import type { UiMon, EffectData } from "battler-state";

export type LogTemplateKey = keyof typeof en.logs;

export enum LogCategory {
  Primary = "primary",     // Main actions (Moves, Switches, Faints)
  Secondary = "secondary", // Modifiers to primary actions (Critical hit, Super effective)
  Hint = "hint",           // Side-effects, abilities, weather, etc.
}

export interface UiNotice {
  type: string;
  name: string;
  mon?: string;
  monRef?: UiMon;
}

export interface FormattedLogEvent {
  message?: FormattedUiLog;
  notices: UiNotice[];
}

export interface ContextVar {
  text: string;
  monRef?: UiMon; // used for tying UI elements to specific game entities
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

export interface MappedLogMetadata {
  mon?: { raw?: string, raw_possessive?: string, ref?: UiMon };
  target?: { raw?: string, raw_possessive?: string, ref?: UiMon };
  source?: { raw?: string, raw_possessive?: string, ref?: UiMon };
}

export interface AnyMappedLog {
  patterns: string[];
  category: LogCategory;
  context: LogContext;
  effect?: EffectData;
  metadata?: MappedLogMetadata;
}

export interface MapperOptions {
  localPlayerId?: string;
  healthFormat?: "fraction" | "percentage";
  forceTemplateKey?: string;
}
