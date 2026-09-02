import { en } from "../locales/en.js";
import type { UiMon, Effect } from "battler-state";
import type { FormattedUiLog } from "./formatter.js";
import type { LogToken } from "./engine.js";

export type LogTemplateKey = keyof typeof en.logs;

export enum LogCategory {
  Primary = "primary",     // Main actions (Moves, Switches, Faints, Transformations, Outcomes)
  Secondary = "secondary", // Modifiers, effects, abilities, weather, and sub-messages
  Hint = "hint",           // System hints, timer warnings, and informational notices
}

export type UiToken = LogToken;

export interface UiNotice {
  type: string;
  name: string;
  mon?: string;
  monRef?: UiMon;
}

export interface FormattedLogEvent {
  messages: FormattedUiLog[];
  notices: UiNotice[];
}

export type LogDividerType = "continue" | "residual";

export type FormattedLogDisplayItem =
  | {
      kind: "turn";
      turn: string;
    }
  | {
      kind: "divider";
      subtype: LogDividerType;
    }
  | {
      kind: "message";
      category: LogCategory;
      message: FormattedUiLog;
    }
  | {
      kind: "notice";
      notice: UiNotice;
    };

export interface ContextVar {
  text: string;
  monRef?: UiMon; // used for tying UI elements to specific game entities
  noAutoCapitalize?: boolean;
}

export type ContextValue =
  | string
  | number
  | boolean
  | bigint
  | ContextVar
  | (string | number | boolean | bigint | ContextVar)[];

export type LogContext = Record<string, ContextValue> & { count?: number };

export interface NoticeRuleCondition {
  hasEffectType?: string;
  hasSourceEffectType?: string;
  titleIn?: string[];
  hasContext?: string;
}

export interface NoticeRuleNotice {
  type: string;
  nameFromPath?: string;
  nameFromContext?: string;
  monResolution?: "fromContext" | "sourceFirst" | "targetFirst";
  monFromContext?: string;
}

export interface NoticeRule {
  condition: NoticeRuleCondition;
  notice: NoticeRuleNotice;
}

export type ExtractVariables<T> =
  T extends string
    ? (T extends `${string}{{${infer Var}}}${infer Rest}` ? Var | ExtractVariables<Rest> : never)
    : T extends readonly (infer U)[]
      ? ExtractVariables<U>
      : T extends (infer U)[]
        ? ExtractVariables<U>
        : never;

export type RequiredContext<K extends LogTemplateKey> = 
  ExtractVariables<NonNullable<typeof en.logs[K]>> extends never
    ? { count?: number }
    : Record<ExtractVariables<NonNullable<typeof en.logs[K]>>, ContextValue> & { count?: number };

export interface MappedLogParticipantMetadata {
  raw?: string;
  raw_possessive?: string;
  possessive?: ContextVar;
  ref?: UiMon;
}

export interface MappedLogMetadata {
  mon?: MappedLogParticipantMetadata;
  target?: MappedLogParticipantMetadata;
  source?: MappedLogParticipantMetadata;
  prev_mon?: MappedLogParticipantMetadata;
}

export interface AnyMappedLog {
  patterns: string[];
  category: LogCategory;
  context: LogContext;
  effect?: Effect;
  source_effect?: Effect;
  metadata?: MappedLogMetadata;
  extension?: string;
}

export interface MapperOptions {
  localPlayerId?: string;
  healthFormat?: "fraction" | "percentage";
  forceTemplateKey?: string;
}
