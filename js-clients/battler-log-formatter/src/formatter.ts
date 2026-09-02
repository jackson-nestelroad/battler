import type { BattleState, UiLogEntry, UiMon } from "battler-state";
import noticeRules from "./config/notice-rules.json" with { type: "json" };
import type { LogToken } from "./engine.js";
import { parseTemplateToTokens } from "./engine.js";
import i18next from "./i18n.js";
import { mapUiLogEntry } from "./mapper.js";
import { parsePattern, patternToKey, serializePattern } from "./pattern.js";
import type {
  AnyMappedLog,
  ContextValue,
  ContextVar,
  FormattedLogEvent,
  LogContext,
  MappedLogParticipantMetadata,
  MapperOptions,
  UiNotice,
} from "./types.js";
import { LogCategory } from "./types.js";

export interface FormattedUiLog {
  key?: string;
  tokens: LogToken[];
  category: LogCategory;
  context: LogContext;
}

function capitalizeContextValue(val: ContextValue): ContextValue {
  if (typeof val === "string") {
    if (!val) return val;
    return val.charAt(0).toUpperCase() + val.slice(1);
  }
  if (val && typeof val === "object" && !Array.isArray(val) && "text" in val) {
    if (val.noAutoCapitalize) return val;
    return { ...val, text: val.text.charAt(0).toUpperCase() + val.text.slice(1) };
  }
  if (Array.isArray(val) && val.length > 0) {
    const first = capitalizeContextValue(val[0]);
    if (first !== val[0]) {
      const newArray = [...val];
      newArray[0] = first as string | ContextVar;
      return newArray as ContextValue;
    }
  }
  return val;
}

function createFormattedUiLog(
  templateKey: string,
  templateString: string,
  category: LogCategory,
  context: LogContext,
): FormattedUiLog | null {
  if (!templateString || typeof templateString !== "string") return null;

  const tokens = parseTemplateToTokens(templateString);
  if (tokens.length === 0) return null;

  // Verify all variable dependencies exist in context
  for (const token of tokens) {
    if (token.type === "variable") {
      if (context[token.value] === undefined) {
        return null;
      }
    }
  }

  let msgContext = { ...context };
  let finalTokens = [...tokens];

  const firstToken = finalTokens[0];
  if (firstToken.type === "text" && firstToken.value.length > 0) {
    finalTokens[0] = {
      ...firstToken,
      value: firstToken.value.charAt(0).toUpperCase() + firstToken.value.slice(1),
    };
  } else if (firstToken.type === "variable") {
    const val = msgContext[firstToken.value];
    const capitalizedVal = capitalizeContextValue(val);
    if (capitalizedVal !== val) {
      const newKey = `__CAPITALIZED_${firstToken.value}`;
      msgContext[newKey] = capitalizedVal;
      finalTokens[0] = { ...firstToken, value: newKey };
    }
  }

  const usedKeys = new Set<string>();
  for (const token of finalTokens) {
    if (token.type === "variable") {
      usedKeys.add(token.value);
    }
  }

  const cleanContext: Record<string, ContextValue> = {};
  for (const [k, v] of Object.entries(msgContext)) {
    if (!usedKeys.has(k)) continue;

    if (typeof v === "object" && v !== null && "text" in v) {
      const cleaned: ContextVar = { text: v.text };
      if (v.monRef) cleaned.monRef = v.monRef;
      cleanContext[k] = cleaned;
    } else if (Array.isArray(v)) {
      cleanContext[k] = (v as ContextValue[]).map((item: ContextValue) => {
        if (typeof item === "object" && item !== null && "text" in item) {
          const cleaned: ContextVar = { text: item.text as string };
          if (item.monRef) cleaned.monRef = item.monRef;
          return cleaned;
        }
        return item as string | ContextVar;
      });
    } else {
      cleanContext[k] = v;
    }
  }

  return {
    key: templateKey.replace(/^logs\./, ""),
    tokens: finalTokens,
    category,
    context: cleanContext,
  };
}

function resolveNoticeMon(
  ruleNotice: {
    type: string;
    nameFromPath?: string;
    nameFromContext?: string;
    monResolution?: string;
    monFromContext?: string;
  },
  mapped: AnyMappedLog,
): { monStr?: string; monRef?: UiMon } {
  if (ruleNotice.monResolution === "fromContext" || ruleNotice.monFromContext) {
    const ctxKey = ruleNotice.monFromContext as string;
    if (mapped.context[ctxKey]) {
      const rawCtx = mapped.context[ctxKey];
      const capCtx = capitalizeContextValue(rawCtx);
      let monStr: string | undefined = undefined;
      let monRef: UiMon | undefined = undefined;
      if (typeof capCtx === "string") {
        monStr = capCtx;
      } else if (capCtx && typeof capCtx === "object" && "text" in capCtx) {
        monStr = (capCtx as ContextVar).text;
        if ((capCtx as ContextVar).monRef) {
          monRef = (capCtx as ContextVar).monRef;
        }
      }
      return { monStr, monRef };
    }
    return {};
  }

  let participant: MappedLogParticipantMetadata | undefined = undefined;
  if (ruleNotice.monResolution === "sourceFirst") {
    // If of / source is specified, use source; otherwise default to target / mon of log
    participant = mapped.metadata?.source || mapped.metadata?.target || mapped.metadata?.mon;
  } else {
    // targetFirst: default to target / mon of log, fallback to source
    participant = mapped.metadata?.mon || mapped.metadata?.target || mapped.metadata?.source;
  }

  if (participant) {
    let monStr: string | undefined = undefined;
    if (participant.possessive) {
      monStr = participant.possessive.charAt(0).toUpperCase() + participant.possessive.slice(1);
    } else if (participant.raw_possessive) {
      monStr =
        participant.raw_possessive.charAt(0).toUpperCase() + participant.raw_possessive.slice(1);
    }
    return { monStr, monRef: participant.ref };
  }

  return {};
}

function findTemplateKey(
  patterns: string[],
  templateArgs: Record<string, unknown>,
): string | undefined {
  if (!patterns || patterns.length === 0) return undefined;
  for (const pattern of patterns) {
    const parsed = parsePattern(pattern);
    const serialized = serializePattern(parsed);
    const safePattern = patternToKey(serialized);

    const fullKey = `logs.${safePattern}`;
    if (
      i18next.exists(fullKey, templateArgs) &&
      i18next.t(fullKey, { ...templateArgs, returnObjects: true }) !== null
    ) {
      return fullKey;
    }
  }
  return undefined;
}

export class LogFormatter {
  private options: MapperOptions;

  constructor(options: MapperOptions = {}) {
    this.options = options;
  }

  public format(entry: UiLogEntry, state?: BattleState): FormattedLogEvent | null {
    const mapped = mapUiLogEntry(entry, state, this.options);
    if (!mapped) return null;

    if (entry.values?.silent !== undefined) {
      return null;
    }

    const count = mapped.context.count;
    const templateArgs: Record<string, unknown> = {};
    if (count !== undefined) templateArgs.count = count;

    // Inject global text replacements (only Titlecase to protect {{MON}})
    templateArgs.Mon = i18next.t("vocabulary.Mon");
    templateArgs.Mons = i18next.t("vocabulary.Mons");

    // Inject stats
    templateArgs.hp = i18next.t("stats.hp");
    templateArgs.atk = i18next.t("stats.atk");
    templateArgs.def = i18next.t("stats.def");
    templateArgs.spa = i18next.t("stats.spa");
    templateArgs.spd = i18next.t("stats.spd");
    templateArgs.spe = i18next.t("stats.spe");
    templateArgs.eva = i18next.t("stats.eva");
    templateArgs.acc = i18next.t("stats.acc");

    const notices: UiNotice[] = [];

    for (const rule of noticeRules) {
      let match = true;
      if ("hasSourceEffectType" in rule.condition && rule.condition.hasSourceEffectType) {
        const reqType = (rule.condition.hasSourceEffectType as string).toLowerCase();
        const actualType = mapped.source_effect?.effect_type?.toLowerCase();
        if (actualType !== reqType) {
          match = false;
        }
      }
      if ("hasEffectType" in rule.condition && rule.condition.hasEffectType) {
        const reqType = (rule.condition.hasEffectType as string).toLowerCase();
        const actualType = mapped.effect?.effect_type?.toLowerCase();
        if (actualType !== reqType) {
          match = false;
        }
      }
      if ("titleIn" in rule.condition && rule.condition.titleIn) {
        const title = mapped.patterns[0]?.split("|")[0] || entry.title.toLowerCase();
        if (!title || !rule.condition.titleIn.includes(title)) {
          match = false;
        }
      }
      if ("hasContext" in rule.condition && rule.condition.hasContext) {
        if (!mapped.context[rule.condition.hasContext as string]) {
          match = false;
        }
      }

      if (match) {
        let name = "";
        if ("nameFromPath" in rule.notice) {
          if (rule.notice.nameFromPath === "source_effect.name" && mapped.source_effect?.name) {
            name = mapped.source_effect.name;
          } else if (rule.notice.nameFromPath === "effect.name" && mapped.effect?.name) {
            name = mapped.effect.name;
          }
        }
        if (!name && "nameFromContext" in rule.notice && rule.notice.nameFromContext) {
          if (mapped.context[rule.notice.nameFromContext as string]) {
            name = String(mapped.context[rule.notice.nameFromContext as string]);
          }
        }

        if (name) {
          const { monStr, monRef } = resolveNoticeMon(rule.notice as any, mapped);
          const exists = notices.some(
            (n) => n.type === rule.notice.type && n.name === name && n.mon === monStr,
          );
          if (!exists) {
            notices.push({
              type: rule.notice.type,
              name,
              mon: monStr,
              monRef,
            });
          }
        }
      }
    }

    const messages: FormattedUiLog[] = [];

    // If entry is a switch log and prev_mon is present, format switchout message first
    const isSwitch =
      mapped.patterns[0]?.split("|")[0] === "switch" || entry.title.toLowerCase() === "switch";
    if (isSwitch && mapped.metadata?.prev_mon && mapped.context.PREV_MON) {
      const switchoutKey = "logs.switchout";
      if (i18next.exists(switchoutKey)) {
        const switchoutTemplate = i18next.t(switchoutKey, { ...templateArgs, returnObjects: true });
        const switchoutContext: LogContext = {
          ...mapped.context,
          MON: mapped.context.PREV_MON,
          MON_POSSESSIVE: mapped.context.PREV_MON_POSSESSIVE,
          MON_PLAYER: mapped.context.PREV_MON_PLAYER,
          MON_PLAYER_POSSESSIVE: mapped.context.PREV_MON_PLAYER_POSSESSIVE,
          PLAYER: mapped.context.PREV_MON_PLAYER,
          PLAYER_POSSESSIVE: mapped.context.PREV_MON_PLAYER_POSSESSIVE,
        };
        if (typeof switchoutTemplate === "string") {
          const msg = createFormattedUiLog(
            switchoutKey,
            switchoutTemplate,
            LogCategory.Secondary,
            switchoutContext,
          );
          if (msg) messages.push(msg);
        } else if (Array.isArray(switchoutTemplate)) {
          for (const item of switchoutTemplate) {
            if (typeof item === "string") {
              const msg = createFormattedUiLog(
                switchoutKey,
                item,
                LogCategory.Secondary,
                switchoutContext,
              );
              if (msg) messages.push(msg);
            }
          }
        }
      }
    }

    let resolvedKey: string | undefined = undefined;
    if (this.options.forceTemplateKey) {
      resolvedKey = `logs.${this.options.forceTemplateKey}`;
    } else {
      resolvedKey = findTemplateKey(mapped.patterns, templateArgs);
    }

    if (resolvedKey && i18next.exists(resolvedKey, templateArgs)) {
      const rawTemplate = i18next.t(resolvedKey, { ...templateArgs, returnObjects: true });
      if (typeof rawTemplate === "string") {
        const msg = createFormattedUiLog(resolvedKey, rawTemplate, mapped.category, mapped.context);
        if (msg) messages.push(msg);
      } else if (Array.isArray(rawTemplate)) {
        for (const item of rawTemplate) {
          if (typeof item === "string") {
            const msg = createFormattedUiLog(resolvedKey, item, mapped.category, mapped.context);
            if (msg) messages.push(msg);
          }
        }
      }
    }

    if (messages.length === 0 && notices.length === 0) return null;

    return { messages, notices };
  }
}

export function stringifyLog(log: FormattedUiLog): string {
  return log.tokens
    .map((token) => {
      if (token.type === "text") return token.value;
      const ctxVal = log.context[token.value];
      if (typeof ctxVal === "string") return ctxVal;
      if (typeof ctxVal === "number") return ctxVal.toString();
      if (Array.isArray(ctxVal))
        return ctxVal.map((v) => (typeof v === "string" ? v : (v as ContextVar).text)).join(", ");
      if (ctxVal && typeof ctxVal === "object" && "text" in ctxVal) return ctxVal.text;
      return `{{${token.value}}}`;
    })
    .join("");
}
