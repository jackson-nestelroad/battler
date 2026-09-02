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
  FormattedLogDisplayItem,
  FormattedLogEvent,
  LogContext,
  MappedLogParticipantMetadata,
  MapperOptions,
  NoticeRule,
  NoticeRuleNotice,
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
    key: templateKey.replace(/^(logs|hints\.(logs|extensions\.[^.]+))\./, ""),
    tokens: finalTokens,
    category,
    context: cleanContext,
  };
}

function resolveNoticeMon(
  ruleNotice: NoticeRuleNotice,
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
    const targetVar: ContextVar | string | undefined =
      participant.possessive || participant.raw_possessive;
    if (targetVar) {
      const capCtx = capitalizeContextValue(targetVar);
      let monStr: string | undefined = undefined;
      if (typeof capCtx === "string") {
        monStr = capCtx;
      } else if (capCtx && typeof capCtx === "object" && "text" in capCtx) {
        monStr = capCtx.text;
      }
      return { monStr, monRef: participant.ref };
    }
  }

  return {};
}

interface ResolvedTemplate {
  key: string;
  category: LogCategory;
}

function findTemplateKey(
  patterns: string[],
  templateArgs: Record<string, unknown>,
  category: LogCategory,
  extensionSource?: string | null,
): ResolvedTemplate | undefined {
  if (!patterns || patterns.length === 0) return undefined;
  for (const pattern of patterns) {
    const parsed = parsePattern(pattern);
    const serialized = serializePattern(parsed);
    const safePattern = patternToKey(serialized);

    if (extensionSource) {
      const extKey = `hints.extensions.${extensionSource}.${safePattern}`;
      if (
        i18next.exists(extKey, templateArgs) &&
        i18next.t(extKey, { ...templateArgs, returnObjects: true }) !== null
      ) {
        return { key: extKey, category: LogCategory.Hint };
      }
    }

    const fullKey = `logs.${safePattern}`;
    if (
      i18next.exists(fullKey, templateArgs) &&
      i18next.t(fullKey, { ...templateArgs, returnObjects: true }) !== null
    ) {
      return { key: fullKey, category };
    }

    const hintKey = `hints.logs.${safePattern}`;
    if (
      i18next.exists(hintKey, templateArgs) &&
      i18next.t(hintKey, { ...templateArgs, returnObjects: true }) !== null
    ) {
      return { key: hintKey, category: LogCategory.Hint };
    }
  }
  return undefined;
}

function pushFormattedMessages(
  key: string,
  category: LogCategory,
  context: LogContext,
  templateArgs: Record<string, unknown>,
  messages: FormattedUiLog[],
): void {
  if (!i18next.exists(key, templateArgs)) return;
  const rawTemplate = i18next.t(key, { ...templateArgs, returnObjects: true });
  const templates =
    typeof rawTemplate === "string"
      ? [rawTemplate]
      : Array.isArray(rawTemplate)
        ? rawTemplate
        : [];
  for (const item of templates) {
    if (typeof item === "string") {
      const msg = createFormattedUiLog(key, item, category, context);
      if (msg) messages.push(msg);
    }
  }
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

    for (const rule of (noticeRules as NoticeRule[])) {
      let match = true;
      if (rule.condition.hasSourceEffectType) {
        const reqType = rule.condition.hasSourceEffectType.toLowerCase();
        const actualType = mapped.source_effect?.effect_type?.toLowerCase();
        if (actualType !== reqType) {
          match = false;
        }
      }
      if (rule.condition.hasEffectType) {
        const reqType = rule.condition.hasEffectType.toLowerCase();
        const actualType = mapped.effect?.effect_type?.toLowerCase();
        if (actualType !== reqType) {
          match = false;
        }
      }
      if (rule.condition.titleIn) {
        const title = mapped.patterns[0]?.split("|")[0] || entry.title.toLowerCase();
        if (!title || !rule.condition.titleIn.includes(title)) {
          match = false;
        }
      }
      if (rule.condition.hasContext) {
        if (!mapped.context[rule.condition.hasContext]) {
          match = false;
        }
      }

      if (match) {
        let name = "";
        if (rule.notice.nameFromPath) {
          if (rule.notice.nameFromPath === "source_effect.name" && mapped.source_effect?.name) {
            name = mapped.source_effect.name;
          } else if (rule.notice.nameFromPath === "effect.name" && mapped.effect?.name) {
            name = mapped.effect.name;
          }
        }
        if (!name && rule.notice.nameFromContext) {
          if (mapped.context[rule.notice.nameFromContext]) {
            name = String(mapped.context[rule.notice.nameFromContext]);
          }
        }

        if (name) {
          const { monStr, monRef } = resolveNoticeMon(rule.notice, mapped);
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
      const switchoutContext: LogContext = {
        ...mapped.context,
        MON: mapped.context.PREV_MON,
        MON_POSSESSIVE: mapped.context.PREV_MON_POSSESSIVE,
        MON_NAME: mapped.context.PREV_MON_NAME,
        MON_NAME_POSSESSIVE: mapped.context.PREV_MON_NAME_POSSESSIVE,
        MON_PLAYER: mapped.context.PREV_MON_PLAYER,
        MON_PLAYER_POSSESSIVE: mapped.context.PREV_MON_PLAYER_POSSESSIVE,
        PLAYER: mapped.context.PREV_MON_PLAYER,
        PLAYER_POSSESSIVE: mapped.context.PREV_MON_PLAYER_POSSESSIVE,
      };
      pushFormattedMessages(
        switchoutKey,
        LogCategory.Secondary,
        switchoutContext,
        templateArgs,
        messages,
      );
    }

    let resolved: ResolvedTemplate | undefined = undefined;
    if (this.options.forceTemplateKey) {
      resolved = {
        key: `logs.${this.options.forceTemplateKey}`,
        category: mapped.category,
      };
    } else {
      resolved = findTemplateKey(
        mapped.patterns,
        templateArgs,
        mapped.category,
        mapped.extension,
      );
    }

    if (resolved) {
      pushFormattedMessages(
        resolved.key,
        resolved.category,
        mapped.context,
        templateArgs,
        messages,
      );
    }

    if (messages.length === 0 && notices.length === 0) return null;

    return { messages, notices };
  }

  public formatEntry(entry: UiLogEntry, state?: BattleState): FormattedLogDisplayItem[] {
    const titleLower = entry.title.toLowerCase();
    if (titleLower === "turn") {
      const turnVal = entry.values?.turn;
      const turnStr = turnVal !== undefined ? String(turnVal) : "";
      return [{ kind: "turn", turn: turnStr }];
    }

    if (titleLower === "continue" || titleLower === "time") {
      return [{ kind: "divider", subtype: "continue" }];
    }

    if (titleLower === "residual") {
      return [{ kind: "divider", subtype: "residual" }];
    }

    const event = this.format(entry, state);
    if (!event) return [];

    const preNotices: FormattedLogDisplayItem[] = [];
    const postNotices: FormattedLogDisplayItem[] = [];

    for (const notice of event.notices) {
      const typeLower = notice.type.toLowerCase();
      if (typeLower === "damage" || typeLower === "heal") {
        postNotices.push({ kind: "notice", notice });
      } else {
        preNotices.push({ kind: "notice", notice });
      }
    }

    const messages: FormattedLogDisplayItem[] = event.messages.map((msg) => ({
      kind: "message",
      category: msg.category,
      message: msg,
    }));

    return [...preNotices, ...messages, ...postNotices];
  }
}

export function formatNoticeText(notice: UiNotice): string {
  const subject = notice.mon ? `${notice.mon} ` : "";
  switch (notice.type.toLowerCase()) {
    case "ability":
    case "item":
      return `[${subject}${notice.name}]`;
    case "damage":
      return `(${subject}lost ${notice.name} HP)`;
    case "heal":
      return `(${subject}restored ${notice.name} HP)`;
    default:
      return `[${notice.type}: ${subject}${notice.name}]`;
  }
}

export function formatUiLogEntry(
  entry: UiLogEntry,
  state?: BattleState,
  options?: MapperOptions | string,
): FormattedLogDisplayItem[] {
  const mapperOptions: MapperOptions =
    typeof options === "string"
      ? { localPlayerId: options, healthFormat: "percentage" }
      : { healthFormat: "percentage", ...options };
  const formatter = new LogFormatter(mapperOptions);
  return formatter.formatEntry(entry, state);
}

export function formatContextValue(val: ContextValue | undefined): string {
  if (val === undefined || val === null) return "";
  if (typeof val === "string") return val;
  if (typeof val === "number" || typeof val === "bigint" || typeof val === "boolean") {
    return String(val);
  }
  if (Array.isArray(val)) {
    return val.map((v) => formatContextValue(v)).join(", ");
  }
  if (typeof val === "object" && "text" in val) {
    return val.text;
  }
  return String(val);
}

export function stringifyLog(log: FormattedUiLog): string {
  return log.tokens
    .map((token) => {
      if (token.type === "text") return token.value;
      const ctxVal = log.context[token.value];
      if (ctxVal === undefined) return `{{${token.value}}}`;
      return formatContextValue(ctxVal);
    })
    .join("");
}
