import { getLogPatterns } from './pattern_reconstructor.js';
import type { UiLogEntry, BattleState } from "battler-state";
import i18next from "./i18n.js";
import { LogCategory } from "./types.js";
import type { LogContext, MapperOptions, ContextValue, ContextVar } from "./types.js";
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
  public format(entry: UiLogEntry, state?: BattleState): FormattedUiLog[] {
    const mapped = mapUiLogEntry(entry, state, this.options);
    if (!mapped) return [];
    
    if (mapped.effect?.additional?.silent !== undefined) {
      return [];
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

    let templateKey = `logs.${mapped.key}`;
    const patterns = getLogPatterns(entry);
    
    if (patterns && patterns.length > 0) {
        let found = false;
        for (const pattern of patterns) {
            const permutations = [pattern];
            // If pattern has mon:*, try target:* and of:*
            if (pattern.includes('mon:*')) {
                permutations.push(pattern.replace('mon:*', 'target:*'));
                permutations.push(pattern.replace('mon:*', 'of:*'));
            } else if (pattern.includes('target:*')) {
                permutations.push(pattern.replace('target:*', 'mon:*'));
                permutations.push(pattern.replace('target:*', 'of:*'));
            } else if (pattern.includes('of:*')) {
                permutations.push(pattern.replace('of:*', 'mon:*'));
                permutations.push(pattern.replace('of:*', 'target:*'));
            }
            
            for (let p of permutations) {
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
            if (found) break;
        }
    }


    const template = i18next.t(templateKey, templateArgs);
    // If the template is literally empty string, or it doesn't exist (returns the key)
    if (!template || template === templateKey) return [];
    
    const result: FormattedUiLog = {
      key: mapped.key,
      tokens: parseTemplateToTokens(template),
      category: mapped.category,
      context: mapped.context
    };

    if (result.tokens.length > 0) {
      const firstToken = result.tokens[0];
      if (firstToken.type === "text" && firstToken.value.length > 0) {
        result.tokens = [...result.tokens];
        result.tokens[0] = { 
          ...firstToken, 
          value: firstToken.value.charAt(0).toUpperCase() + firstToken.value.slice(1) 
        };
      } else if (firstToken.type === "variable") {
        const val = result.context[firstToken.value];
        const capitalizedVal = capitalizeContextValue(val);
        if (capitalizedVal !== val) {
          result.tokens = [...result.tokens];
          result.context = { ...result.context };
          const newKey = `__CAPITALIZED_${firstToken.value}`;
          result.context[newKey] = capitalizedVal;
          result.tokens[0] = { ...firstToken, value: newKey };
        }
      }
    }

    const logs: FormattedUiLog[] = [];
    
    // Inject synthetic Ability log if this log was triggered by an ability
    if (mapped.effect?.effect?.effect_type === "Ability" && mapped.effect.effect.name) {
      // Create a synthetic ability log
      const abilityLog: FormattedUiLog = {
        tokens: parseTemplateToTokens(`[${mapped.effect.effect.name}]`),
        category: LogCategory.Ability,
        context: {}
      };
      logs.push(abilityLog);
    }
    
    logs.push(result);
    return logs;
  }

}
