import type { Effect, UiMon } from "battler-state";
import { en } from "../locales/en.js";
import type { LogToken } from "./engine.js";
import type { FormattedUiLog } from "./formatter.js";

export type LogTemplateKey = keyof typeof en.logs;
export type HintLogTemplateKey = keyof typeof en.hints.logs;

export enum LogCategory {
  Primary = "primary", // Main actions (Moves, Switches, Faints, Transformations, Outcomes)
  Secondary = "secondary", // Modifiers, effects, abilities, weather, and sub-messages
  Hint = "hint", // System hints, timer warnings, and informational notices
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
  titleNotIn?: string[];
  hasContext?: string;
  hasSource?: boolean;
  hasOf?: boolean;
}

export interface NoticeRuleNotice {
  type: string;
  nameFromPath?: string;
  nameFromContext?: string;
  monResolution?: "fromContext" | "sourceFirst" | "targetFirst" | "sourceOnly";
  monFromContext?: string;
}

export interface NoticeRule {
  condition: NoticeRuleCondition;
  notice: NoticeRuleNotice;
}


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
  of?: MappedLogParticipantMetadata;
  prev_mon?: MappedLogParticipantMetadata;
  [key: string]: MappedLogParticipantMetadata | undefined;
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
  isSpectator?: boolean;
  healthFormat?: "fraction" | "percentage";
  forceTemplateKey?: string;
}
