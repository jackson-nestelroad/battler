import type { UiLogEntry, BattleState } from "battler-state";
import i18next from "./i18n.js";
import { LogCategory } from "./types.js";
import type { LogContext, MapperOptions, ContextValue, ContextVar, FormattedLogEvent, UiNotice } from "./types.js";
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
    const rawData = typeof entry === 'string' ? null : Object.values(entry)[0] as any;
    if (rawData && typeof rawData === 'object') {
        for (const [k, v] of Object.entries(rawData)) {
            // Skip large domain objects that are explicitly mapped
            if (k === 'effect' || k === 'mon' || k === 'target' || k === 'source' || k === 'title') continue;
            
            if (typeof v === 'string' || typeof v === 'number' || typeof v === 'boolean') {
                const upperK = k.toUpperCase();
                if (ctx[upperK] === undefined) {
                    ctx[upperK] = String(v);
                }
            } else if (v && typeof v === 'object' && !Array.isArray(v)) {
                // If it's a map of primitive values (like stats: { atk: 10 })
                // ensure it's not a Mon object by checking for known Mon keys
                if (!('Active' in v) && !('Bench' in v) && !('Party' in v)) {
                    for (const [subK, subV] of Object.entries(v)) {
                        if (typeof subV === 'string' || typeof subV === 'number' || typeof subV === 'boolean') {
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
    const templateArgs = count !== undefined ? { count } : undefined;

    let templateKey = `logs.${mapped.patterns[0]}`;
    const patterns = mapped.patterns;
    
    if (this.options.forceTemplateKey) {
        templateKey = `logs.${this.options.forceTemplateKey}`;
    } else if (patterns && patterns.length > 0) {
        let found = false;
        for (const pattern of patterns) {
            let p = pattern;
            const parts = p.split('|');
            const title = parts.shift()!;
            const tags = parts.filter(x => !x.startsWith('[') && !x.endsWith(']'));
            const flags = parts.filter(x => x.startsWith('[') && x.endsWith(']'));
            tags.sort();
            flags.sort();
            p = [title, ...tags, ...flags].join('|');
            
            const safePattern = p
              .replace(/\|/g, '__')
              .replace(/:/g, '_')
              .replace(/\*/g, 'any')
              .toLowerCase()
              .replace(/[^a-z0-9_]/g, '');
            if (i18next.exists(`logs.${safePattern}`)) {
                templateKey = `logs.${safePattern}`;
                found = true;
                break;
            }
        }
    }

    const notices: UiNotice[] = [];
    
    // Inject synthetic Ability notice if this log was triggered by an ability
    if (mapped.effect?.effect?.effect_type === "Ability" && mapped.effect.effect.name) {
      let monStr: string | undefined = undefined;
      let monRef: any = undefined;
      if (mapped.metadata?.mon?.raw_possessive) {
          monStr = String(mapped.metadata.mon.raw_possessive);
      }
      if (mapped.metadata?.mon?.ref) {
          monRef = mapped.metadata.mon.ref;
      }
      notices.push({
          type: "Ability",
          name: mapped.effect.effect.name,
          mon: monStr,
          monRef
      });
    }

    let template = i18next.t(templateKey, templateArgs);
    
    // Check if the primary event itself is just an ability announcement (e.g. ability|mon:X|ability:Intimidate)
    if (mapped.patterns[0]?.startsWith('ability') && mapped.context.ABILITY) {
      let monStr: string | undefined = undefined;
      let monRef: any = undefined;
      if (mapped.metadata?.mon?.raw_possessive) {
          monStr = String(mapped.metadata.mon.raw_possessive);
      }
      if (mapped.metadata?.mon?.ref) {
          monRef = mapped.metadata.mon.ref;
      }
      notices.push({
          type: "Ability",
          name: String(mapped.context.ABILITY),
          mon: monStr,
          monRef
      });
      // We don't want to emit an [UNHANDLED] text message for pure ability announcements
      // but we DO want to emit legitimate ability messages!
      if (template === "[UNHANDLED]") {
        template = ""; 
      }
    }

    let message: FormattedUiLog | undefined = undefined;

    // Check for explicit empty/silent structural primary logs
    if (mapped.patterns.includes('residual')) {
      message = {
          key: "residual",
          tokens: [],
          category: mapped.category,
          context: mapped.context
      };
    } else if (template && template !== templateKey) {
        message = {
          key: templateKey.replace('logs.', ''),
          tokens: parseTemplateToTokens(template),
          category: mapped.category,
          context: mapped.context
        };

        if (message.tokens.length > 0) {
          const firstToken = message.tokens[0];
          if (firstToken.type === "text" && firstToken.value.length > 0) {
            message.tokens = [...message.tokens];
            message.tokens[0] = { 
              ...firstToken, 
              value: firstToken.value.charAt(0).toUpperCase() + firstToken.value.slice(1) 
            };
          } else if (firstToken.type === "variable") {
            const val = message.context[firstToken.value];
            const capitalizedVal = capitalizeContextValue(val);
            if (capitalizedVal !== val) {
              message.tokens = [...message.tokens];
              message.context = { ...message.context };
              const newKey = `__CAPITALIZED_${firstToken.value}`;
              message.context[newKey] = capitalizedVal;
              message.tokens[0] = { ...firstToken, value: newKey };
            }
          }
        }
    }

    if (!message && notices.length === 0) return null;

    if (message) {
      const usedKeys = new Set<string>();
      for (const token of message.tokens) {
          if (token.type === "variable") {
              usedKeys.add(token.value);
          }
      }

      const cleanContext: Record<string, any> = {};
      for (const [k, v] of Object.entries(message.context)) {
          if (!usedKeys.has(k)) continue;

          if (typeof v === "object" && v !== null && "text" in v) {
              const cleaned: any = { text: v.text };
              if (v.monRef) cleaned.monRef = v.monRef;
              cleanContext[k] = cleaned;
          } else if (Array.isArray(v)) {
              cleanContext[k] = v.map((item: any) => {
                  if (typeof item === "object" && item !== null && "text" in item) {
                      const cleaned: any = { text: item.text };
                      if (item.monRef) cleaned.monRef = item.monRef;
                      return cleaned;
                  }
                  return item;
              });
          } else {
              cleanContext[k] = v;
          }
      }
      message.context = cleanContext;
    }

    return { message, notices };
  }

}

export function stringifyLog(log: FormattedUiLog): string {
    return log.tokens.map((token) => {
        if (token.type === "text") return token.value;
        const ctxVal = log.context[token.value];
        if (typeof ctxVal === "string") return ctxVal;
        if (typeof ctxVal === "number") return ctxVal.toString();
        if (Array.isArray(ctxVal)) return ctxVal.map(v => typeof v === "string" ? v : (v as any).text).join(", ");
        if (ctxVal && typeof ctxVal === "object" && "text" in ctxVal) return ctxVal.text;
        return `{{${token.value}}}`;
    }).join("");
}
