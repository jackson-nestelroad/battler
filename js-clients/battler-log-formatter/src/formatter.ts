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

  public format(entry: UiLogEntry, state?: BattleState): FormattedUiLog | null {
    const mapped = mapUiLogEntry(entry, state, this.options);
    if (!mapped) return null;
    
    const count = mapped.context.count;
    const templateArgs = count !== undefined ? { count } : undefined;

    // Use i18next to get the string, passing any primitive args like count for plurals
    const template = i18next.t(`logs.${mapped.key}`, templateArgs);
    if (!template || template === `logs.${mapped.key}`) return null;
    
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
          // Store it under a new key to avoid mutating shared variables
          const newKey = `__CAPITALIZED_${firstToken.value}`;
          result.context[newKey] = capitalizedVal;
          result.tokens[0] = { ...firstToken, value: newKey };
        }
      }
    }

    return result;
  }
}
