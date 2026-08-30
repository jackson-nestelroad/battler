import type { UiLogEntry, BattleState, UiMon } from "battler-state";
import i18next from "./i18n.js";
import { LogCategory } from "./types.js";
import type { LogContext, MapperOptions, ContextValue, ContextVar, FormattedLogEvent, UiNotice } from "./types.js";
import noticeRules from "./config/notice-rules.json" with { type: "json" };
import { parseTemplateToTokens } from "./engine.js";
import type { LogToken } from "./engine.js";
import { mapUiLogEntry } from "./mapper.js";

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

function findTemplateKey(
  patterns: string[],
  templateArgs: Record<string, unknown>,
): string | undefined {
  if (!patterns || patterns.length === 0) return undefined;
  for (const pattern of patterns) {
    const parts = pattern.split("|");
    const title = parts.shift()!;
    const tags = parts.filter((x) => x.includes(":"));
    const flags = parts.filter((x) => !x.includes(":"));
    tags.sort();
    flags.sort();
    const p = [title, ...tags, ...flags].join("|");

    const safePattern = p
      .replace(/\|/g, "__")
      .replace(/:/g, "_")
      .replace(/\*/g, "any")
      .toLowerCase()
      .replace(/[^a-z0-9_]/g, "");

    const fullKey = `logs.${safePattern}`;
    if (i18next.exists(fullKey) && i18next.t(fullKey, templateArgs) !== null) {
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
    
    if (mapped.effect?.additional?.silent !== undefined) {
      return null;
    }
    
    // Auto-augment context with raw variables so fallback translations always have them
    const ctx = mapped.context as Record<string, ContextValue>;
    if (mapped.effect) {
        if (mapped.effect.effect) {
            const e = mapped.effect.effect;
            if (e.effect_type && e.name) {
                const key = e.effect_type.toUpperCase();
                if (ctx[key] === undefined) {
                    ctx[key] = e.name;
                }
            }
        }
        
        if (mapped.effect.source_effect) {
            const e = mapped.effect.source_effect;
            if (e.effect_type && e.name) {
                const key = `FROM_${e.effect_type.toUpperCase()}`;
                if (ctx[key] === undefined) {
                    ctx[key] = e.name;
                }
            }
        }

        if (mapped.effect.additional) {
            for (const [k, v] of Object.entries(mapped.effect.additional)) {
                const upperK = k.toUpperCase();
                if (ctx[upperK] === undefined && v !== undefined) {
                    ctx[upperK] = String(v);
                }
            }
        }
    }
    
    // Also auto-augment from any other raw top-level fields (e.g. exp in Experience, stats in LevelUp)
    const rawData = typeof entry === "string" ? null : Object.values(entry)[0] as Record<string, unknown>;
    if (rawData && typeof rawData === "object") {
        for (const [k, v] of Object.entries(rawData)) {
            if (k === "effect" || k === "mon" || k === "target" || k === "source" || k === "title") continue;
            
            if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") {
                const upperK = k.toUpperCase();
                if (ctx[upperK] === undefined) {
                    ctx[upperK] = String(v);
                }
            } else if (v && typeof v === "object" && !Array.isArray(v)) {
                if (!("Active" in v) && !("Bench" in v) && !("Party" in v)) {
                    for (const [subK, subV] of Object.entries(v)) {
                        if (typeof subV === "string" || typeof subV === "number" || typeof subV === "boolean") {
                            const upperSubK = subK.toUpperCase();
                            if (ctx[upperSubK] === undefined) {
                                ctx[upperSubK] = String(subV);
                            }
                        }
                    }
                }
            }
        }
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
        if (rule.condition.hasEffectType && mapped.effect?.effect?.effect_type !== rule.condition.hasEffectType) {
            match = false;
        }
        if (rule.condition.titleIn) {
            const title = mapped.patterns[0]?.split("|")[0];
            if (!title || !rule.condition.titleIn.includes(title)) {
                match = false;
            }
        }
        if (rule.condition.hasContext && !mapped.context[rule.condition.hasContext]) {
            match = false;
        }
        
        if (match) {
            let monStr: string | undefined = undefined;
            let monRef: UiMon | undefined = undefined;
            const monMeta = mapped.metadata?.mon || mapped.metadata?.target;
            if (monMeta?.raw_possessive) {
                monStr = String(monMeta.raw_possessive);
            }
            if (monMeta?.ref) {
                monRef = monMeta.ref;
            }
            
            let name = "";
            if (rule.notice.nameFromPath === "effect.effect.name" && mapped.effect?.effect?.name) {
                name = mapped.effect.effect.name;
            } else if (rule.notice.nameFromContext && mapped.context[rule.notice.nameFromContext]) {
                name = String(mapped.context[rule.notice.nameFromContext]);
            }
            
            notices.push({
                type: rule.notice.type as "Ability",
                name,
                mon: monStr,
                monRef
            });
        }
    }

    const messages: FormattedUiLog[] = [];

    // Check for explicit empty/silent structural primary logs
    if (mapped.patterns.includes("residual")) {
      messages.push({
        key: "residual",
        tokens: [],
        category: mapped.category,
        context: {},
      });
    } else {
      let resolvedKey: string | undefined = undefined;
      if (this.options.forceTemplateKey) {
        resolvedKey = `logs.${this.options.forceTemplateKey}`;
      } else {
        resolvedKey = findTemplateKey(mapped.patterns, templateArgs);
      }

      if (resolvedKey && i18next.exists(resolvedKey)) {
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
    }

    if (messages.length === 0 && notices.length === 0) return null;

    return { messages, notices };
  }
}

export function stringifyLog(log: FormattedUiLog): string {
    return log.tokens.map((token) => {
        if (token.type === "text") return token.value;
        const ctxVal = log.context[token.value];
        if (typeof ctxVal === "string") return ctxVal;
        if (typeof ctxVal === "number") return ctxVal.toString();
        if (Array.isArray(ctxVal)) return ctxVal.map(v => typeof v === "string" ? v : (v as ContextVar).text).join(", ");
        if (ctxVal && typeof ctxVal === "object" && "text" in ctxVal) return ctxVal.text;
        return `{{${token.value}}}`;
    }).join("");
}
