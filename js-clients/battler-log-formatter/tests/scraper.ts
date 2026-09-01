import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { generateCombinatorics } from "../src/mapper.js";
import { matchesRule, parsePattern, patternToKey, serializePattern } from "../src/pattern.js";
const __dirname = path.dirname(fileURLToPath(import.meta.url));

import mapperRules from "../src/config/mapper-rules.json" with { type: "json" };
import scraperConfig from "./data/scraper-config.json" with { type: "json" };

export function maskLog(line: string): string[] {
  line = line.trim();
  if (!line) return [];

  const parts = line.split("|").map((p) => p.trim());
  let title = parts[0];

  const tags: string[] = [];
  const flags: string[] = [];

  for (let i = 1; i < parts.length; i++) {
    const p = parts[i];
    if (!p) continue;

    if (p.includes(":")) {
      const splitIndex = p.indexOf(":");
      let k = p.substring(0, splitIndex);
      const v = p.substring(splitIndex + 1);

      if (scraperConfig.excludeTags.includes(k)) continue;

      let appliedBucket = false;
      const normalizedTitle = title.replace(/^-/, "").toLowerCase();
      for (const bucket of mapperRules.numericBuckets) {
        if (bucket.tag === k && bucket.titles.includes(normalizedTitle)) {
          const num = Number(v);
          if (!isNaN(num) && num >= bucket.min) {
            tags.push(`${k}:${bucket.min}${bucket.suffix}`);
            appliedBucket = true;
            break;
          }
        }
      }

      if (!appliedBucket) {
        tags.push(`${k}:${v}`);
      }
    } else {
      flags.push(p);
    }
  }

  let finalTags = [...tags];
  let finalFlags = [...flags];

  // Phase 1: Strip
  for (const rule of scraperConfig.rules || []) {
    if (!rule.strip) continue;
    if (matchesRule(tags, title, rule)) {
      finalTags = finalTags.filter((t) => !rule.strip!.includes(t.split(":")[0]));
      finalFlags = finalFlags.filter((f) => !rule.strip!.includes(f));
    }
  }

  // Phase 2: Collapse
  for (const rule of scraperConfig.rules || []) {
    if (!rule.collapse) continue;
    if (matchesRule(tags, title, rule)) {
      for (let i = 0; i < finalTags.length; i++) {
        const [k] = finalTags[i].split(":");
        if (rule.collapse.includes(k)) {
          const parts = finalTags[i].split(":");
          if (parts.length > 2 && (k === "from" || k === "what")) {
            finalTags[i] = `${parts[0]}:${parts[1]}:*`;
          } else {
            finalTags[i] = `${k}:*`;
          }
        }
      }
    }
  }

  const basePattern = [title, ...finalTags, ...finalFlags].join("|");
  const results = [basePattern];

  // Phase 3: Inject
  for (const rule of scraperConfig.rules || []) {
    if (!rule.inject) continue;
    if (matchesRule(tags, title, rule)) {
      for (const injected of rule.inject) {
        results.push([title, ...finalTags, injected, ...finalFlags].join("|"));
      }
    }
  }

  return results;
}

const RUST_TESTS_DIR = path.resolve(__dirname, "../../../battler/tests");

function readAllRsFiles(dir: string): string[] {
  let results: string[] = [];
  const list = fs.readdirSync(dir);

  for (const file of list) {
    const fullPath = path.resolve(dir, file);
    const stat = fs.statSync(fullPath);
    if (stat && stat.isDirectory()) {
      results = results.concat(readAllRsFiles(fullPath));
    } else if (file.endsWith(".rs")) {
      results.push(fullPath);
    }
  }
  return results;
}

function extractLogsFromRs(): { raw: string[]; patterns: Set<string> } {
  const files = readAllRsFiles(RUST_TESTS_DIR);
  const allLogs: Set<string> = new Set();
  const allPatterns: Set<string> = new Set();

  for (const file of files) {
    const content = fs.readFileSync(file, "utf-8");
    const regex = /r#"\[([\s\S]*?)\]"#/g;
    let match;
    while ((match = regex.exec(content)) !== null) {
      try {
        const jsonStr = `[${match[1]}]`;
        const arr = JSON.parse(jsonStr);
        if (Array.isArray(arr)) {
          for (const item of arr) {
            if (typeof item === "string") {
              allLogs.add(item);
              const patterns = maskLog(item);
              for (const pattern of patterns) {
                allPatterns.add(pattern);
              }
            }
          }
        }
      } catch (e) {}
    }
  }

  return { raw: Array.from(allLogs), patterns: allPatterns };
}

interface EffectEntry {
  type: string;
  name: string;
  program: string;
  delegates: string[];
  condition_type?: string;
}
let effectRegistry: Map<string, EffectEntry> | null = null;

function buildEffectRegistry(): Map<string, EffectEntry> {
  if (effectRegistry) return effectRegistry;
  effectRegistry = new Map<string, EffectEntry>();
  const BATTLE_DATA_DIR = path.resolve(__dirname, "../../../battle-data/data");

  function getTypeFromCondition(condType?: string): string {
    if (!condType) return "condition";
    const lower = condType.toLowerCase();
    if (lower === "status" || lower === "weather" || lower === "volatile" || lower === "built-in")
      return lower;
    return "condition";
  }

  function extractStrings(obj: unknown): string {
    if (typeof obj === "string") return obj;
    if (Array.isArray(obj)) return obj.map(extractStrings).join("\n");
    if (obj && typeof obj === "object") {
      return Object.values(obj).map(extractStrings).join("\n");
    }
    return "";
  }

  function readAllJsonFiles(dir: string): string[] {
    let results: string[] = [];
    const list = fs.readdirSync(dir);
    for (const file of list) {
      const fullPath = path.resolve(dir, file);
      const stat = fs.statSync(fullPath);
      if (stat && stat.isDirectory()) {
        results = results.concat(readAllJsonFiles(fullPath));
      } else if (fullPath.endsWith(".json")) {
        results.push(fullPath);
      }
    }
    return results;
  }

  const jsonFiles = readAllJsonFiles(BATTLE_DATA_DIR);
  for (const file of jsonFiles) {
    const relPath = path.relative(BATTLE_DATA_DIR, file);
    let defaultType = "effect";
    if (relPath.startsWith("abilities")) defaultType = "ability";
    else if (relPath.startsWith("items")) defaultType = "item";
    else if (relPath.startsWith("moves")) defaultType = "move";
    else if (relPath.startsWith("clauses")) defaultType = "clause";
    else if (relPath.startsWith("mons")) defaultType = "species";

    const content = JSON.parse(fs.readFileSync(file, "utf-8"));

    const processObj = (key: string, obj: Record<string, unknown>, typeOverride?: string) => {
      if (!obj || typeof obj !== "object") return;
      const name = (obj.name as string) || key;
      const type =
        typeOverride ||
        (obj.condition_type ? getTypeFromCondition(obj.condition_type as string) : defaultType);

      const programStr = extractStrings((obj.program as Record<string, unknown>) || obj);

      const id = (name as string).toLowerCase().replace(/[^a-z0-9]/g, "");

      const delegates =
        (obj.delegates as string[]) ||
        (obj.effect as Record<string, unknown>)?.delegates ||
        (obj.condition as Record<string, unknown>)?.delegates ||
        [];

      const entry = {
        type,
        condition_type: obj.condition_type as string | undefined,
        name,
        program: programStr,
        delegates: delegates,
      };

      effectRegistry!.set(name, entry);
      effectRegistry!.set(id, entry);
      effectRegistry!.set(`${type}:${id}`, entry);
      // Also sometimes it's referenced as condition:id even if type is ability
      effectRegistry!.set(`condition:${id}`, entry);
    };

    if (Array.isArray(content)) {
      for (const val of content) processObj(val.name || "Unknown", val);
    } else {
      for (const [key, val] of Object.entries(content)) {
        if (val && typeof val === "object") {
          processObj(key, val as Record<string, unknown>);
        }
      }
    }
  }

  return effectRegistry;
}

const LOG_NAME_ALIASES: Record<string, string> = {
  announceitem: "item",
  preparemove: "prepare",
  usemove: "move",
};

function getConditionPrefix(effect: EffectEntry, isFrom: boolean): string {
  if (!effect.condition_type) return effect.type;
  const lower = effect.condition_type.toLowerCase();
  if (lower === "built-in" || lower === "volatile") {
    return isFrom ? "" : "condition";
  }
  if (lower === "status" || lower === "weather" || lower === "zpower") {
    return lower;
  }
  return "condition";
}

function synthesizeFxlangLogs(
  logName: string,
  typePrefix: string,
  name: string,
  isFrom: boolean,
  fxlangLogs: Set<string>,
): void {
  const ALWAYS_NO_TAG = new Set(scraperConfig.alwaysNoTagLogs || []);
  const NO_TAG_UNLESS_FROM = new Set(scraperConfig.noTagUnlessFromLogs || []);
  const PREPEND_WILDCARDS = new Set(scraperConfig.prependWildcardTagLogs || []);

  if (logName === "fail") {
    if (isFrom) {
      fxlangLogs.add(`fail|from:${typePrefix}${name}`);
      fxlangLogs.add(`fail|what:*|from:${typePrefix}${name}`);
    } else {
      fxlangLogs.add(`fail|what:${typePrefix}${name}`);
    }
  } else if (logName === "fail_unboost") {
    if (isFrom) {
      fxlangLogs.add(`fail|what:unboost|from:${typePrefix}${name}`);
    } else {
      fxlangLogs.add(`fail|what:unboost|${typePrefix}${name}`);
    }
  } else if (logName === "fail_heal") {
    fxlangLogs.add(`fail|what:heal`);
  } else if (ALWAYS_NO_TAG.has(logName) || (NO_TAG_UNLESS_FROM.has(logName) && !isFrom)) {
    fxlangLogs.add(logName);
  } else if (PREPEND_WILDCARDS.has(logName)) {
    fxlangLogs.add(`${logName}|ability:*|from:${typePrefix}${name}`);
    fxlangLogs.add(`${logName}|move:*|from:${typePrefix}${name}`);
  } else if (isFrom) {
    fxlangLogs.add(`${logName}|from:${typePrefix}${name}`);
  } else {
    fxlangLogs.add(`${logName}|${typePrefix}${name}`);
  }
}

function extractLogsFromFxlang(): Set<string> {
  const fxlangLogs = new Set<string>();
  const registry = buildEffectRegistry();

  function getLogsForEffect(
    effect: EffectEntry,
    visited: Set<EffectEntry>,
  ): { logType: string; fromEffect: boolean }[] {
    if (visited.has(effect)) return [];
    visited.add(effect);

    const logs: { logType: string; fromEffect: boolean }[] = [];
    const regex = /log_([a-z_]+)(?:\:\s*([^"'\n\]]+))?/g;
    let match;
    while ((match = regex.exec(effect.program)) !== null) {
      const logType = match[1];
      const customArg = match[2];

      if (logType === "custom_effect" && customArg) {
        logs.push({ logType: customArg.split(" ")[0], fromEffect: false });
      } else {
        logs.push({ logType, fromEffect: Boolean(customArg && customArg.includes("from_effect")) });
      }
    }

    const sideAddRegex = /add_side_condition/g;
    if (sideAddRegex.test(effect.program)) {
      logs.push({ logType: "sidestart", fromEffect: false });
    }
    const sideRemoveRegex = /remove_side_condition/g;
    if (sideRemoveRegex.test(effect.program)) {
      logs.push({ logType: "sideend", fromEffect: false });
    }
    const formeChangeRegex = /forme_change\:\s*([^"'\n\]]+)/g;
    if (formeChangeRegex.test(effect.program)) {
      logs.push({ logType: "formechange", fromEffect: true });
    }

    for (const delegateId of effect.delegates || []) {
      const delegateEffect = registry.get(delegateId);
      if (delegateEffect) {
        logs.push(...getLogsForEffect(delegateEffect, visited));
      }
    }

    return logs;
  }

  const IGNORE_KEYS = new Set(scraperConfig.ignoreKeys || []);
  const uniqueEffects = new Set(registry.values());
  for (const effect of uniqueEffects) {
    const name = effect.name;
    const logs = getLogsForEffect(effect, new Set());
    for (const rawLog of logs) {
      let logName = rawLog.logType.replace(/_/g, "");
      if (rawLog.logType === "fail_unboost" || rawLog.logType === "fail_heal") {
        logName = rawLog.logType;
      } else if (LOG_NAME_ALIASES[logName]) {
        logName = LOG_NAME_ALIASES[logName];
      }

      const ALWAYS_FROM_EFFECT = new Set(scraperConfig.alwaysFromEffectLogs || []);
      const isFrom = rawLog.fromEffect || ALWAYS_FROM_EFFECT.has(logName);

      const prefix = getConditionPrefix(effect, isFrom);
      const typePrefix = prefix ? prefix + ":" : "";

      synthesizeFxlangLogs(logName, typePrefix, name, isFrom, fxlangLogs);
    }
  }

  // Apply ignores across all added logs
  for (const log of fxlangLogs) {
    if (IGNORE_KEYS.has(log)) {
      fxlangLogs.delete(log);
    }
  }

  return fxlangLogs;
}

function generateMatrix() {
  const extracted = extractLogsFromRs();
  const validTemplates = new Set<string>();

  function getTemplate(pattern: string): string {
    const parts = pattern.split("|");
    const title = parts[0];
    const tempParts = [title];
    for (let i = 1; i < parts.length; i++) {
      if (!parts[i].includes(":")) {
        tempParts.push(parts[i]);
      } else {
        const firstColon = parts[i].indexOf(":");
        const k = parts[i].substring(0, firstColon);
        const v = parts[i].substring(firstColon + 1);
        if (v.includes(":")) {
          const secondColon = v.indexOf(":");
          const type = v.substring(0, secondColon);
          tempParts.push(`${k}:${type}:*`);
        } else {
          tempParts.push(`${k}:*`);
        }
      }
    }
    return tempParts.join("|");
  }

  for (const pattern of extracted.patterns) {
    validTemplates.add(getTemplate(pattern));
  }

  const fxlangPatterns = extractLogsFromFxlang();
  const ALLOW_KEYS = new Set(scraperConfig.allowKeys || []);

  for (const pattern of fxlangPatterns) {
    const maskedList = maskLog(pattern);
    for (const masked of maskedList) {
      const tpl = getTemplate(masked);
      if (!validTemplates.has(tpl)) {
        if (!ALLOW_KEYS.has(masked)) {
          console.warn(
            `[WARNING] Dropping illegal pattern not seen in tests: ${masked} (template: ${tpl})`,
          );
        } else {
          extracted.patterns.add(masked);
        }
      } else {
        extracted.patterns.add(masked);
      }
    }
  }

  const manualLogs = (scraperConfig as any).manualLogs || [];
  for (const pattern of manualLogs) {
    const maskedList = maskLog(pattern);
    for (const masked of maskedList) {
      extracted.patterns.add(masked);
    }
  }

  const registry = buildEffectRegistry();
  const uniqueEffects = new Set(registry.values());
  const newClonedPatterns = new Set<string>();

  for (const effect of uniqueEffects) {
    if (!effect.delegates || effect.delegates.length === 0) continue;

    const allDelegates = new Set<string>();
    const queue = [...effect.delegates];
    const visited = new Set<string>();

    while (queue.length > 0) {
      const dId = queue.shift()!;
      if (visited.has(dId)) continue;
      visited.add(dId);

      const dEffect = registry.get(dId);
      if (dEffect) {
        allDelegates.add(`${dEffect.type}:${dEffect.name}`);
        if (dEffect.delegates) queue.push(...dEffect.delegates);
      }
    }

    if (allDelegates.size === 0) continue;

    const eSig = `${effect.type}:${effect.name}`;

    for (const pattern of extracted.patterns) {
      const parsed = parsePattern(pattern);
      for (const dSig of allDelegates) {
        let modified = false;
        const newTags = parsed.tags.map((t) => {
          if (t === dSig) {
            modified = true;
            return eSig;
          }
          if (t === `from:${dSig}`) {
            modified = true;
            return `from:${eSig}`;
          }
          return t;
        });

        if (modified) {
          newClonedPatterns.add(serializePattern({ ...parsed, tags: newTags }));
        }
      }
    }
  }

  for (const cloned of newClonedPatterns) {
    extracted.patterns.add(cloned);
  }

  // Dynamic status condition expansion from conditions.json
  const statusConditionNames = new Set<string>();
  for (const entry of uniqueEffects) {
    if (entry.condition_type === "Status") {
      statusConditionNames.add(entry.name);
    }
  }

  for (const pattern of Array.from(extracted.patterns)) {
    const parsed = parsePattern(pattern);
    for (const sName of statusConditionNames) {
      for (const targetName of statusConditionNames) {
        if (targetName === sName) continue;
        let modified = false;
        const newTags = parsed.tags.map((t) => {
          if (t === `status:${sName}`) {
            modified = true;
            return `status:${targetName}`;
          }
          if (t === `from:status:${sName}`) {
            modified = true;
            return `from:status:${targetName}`;
          }
          return t;
        });

        if (modified) {
          extracted.patterns.add(serializePattern({ ...parsed, tags: newTags }));
        }
      }
    }
  }

  // Build inverted index of pattern -> raw logs in O(N) time
  const patternToRawLogs = new Map<string, string[]>();
  for (const rawLog of extracted.raw) {
    const maskedList = maskLog(rawLog);
    for (const p of maskedList) {
      let list = patternToRawLogs.get(p);
      if (!list) {
        list = [];
        patternToRawLogs.set(p, list);
      }
      if (list.length < 3) {
        list.push(rawLog);
      }
    }
  }

  const finalMatrix: string[] = [];
  for (const pattern of Array.from(extracted.patterns).sort()) {
    const matching = patternToRawLogs.get(pattern) || [];
    finalMatrix.push(...matching);
  }

  fs.writeFileSync(
    path.resolve(__dirname, "data/logs-matrix.json"),
    JSON.stringify(finalMatrix, null, 2),
  );
  fs.writeFileSync(
    path.resolve(__dirname, "data/unique-log-patterns.txt"),
    Array.from(extracted.patterns).sort().join("\n"),
  );

  console.log(`Generated ${extracted.patterns.size} unique patterns in unique-log-patterns.txt`);
  console.log(`Generated ${finalMatrix.length} raw examples in logs-matrix.json`);

  const enTsPath = path.resolve(__dirname, "../locales/en.ts");
  let enTsContent = fs.readFileSync(enTsPath, "utf-8");

  const logsRegex = /(logs: \{)([\s\S]*?)(\n  \})/m;
  const match = enTsContent.match(logsRegex);

  if (match) {
    const blockContent = match[2];
    const allGeneratedFallbacks = new Set<string>();
    const newKeys: string[] = [];
    let added = false;

    for (const pattern of Array.from(extracted.patterns)) {
      let p = pattern;
      const pParts = p.split("|");
      const pTitle = pParts.shift()!;
      const pTags = pParts.filter((x) => x.includes(":"));
      const pFlags = pParts.filter((x) => !x.includes(":"));

      const rawPatterns = generateCombinatorics(pTitle, pTags, pFlags);
      const allKeys = rawPatterns.map((rp) => patternToKey(rp));

      for (const k of allKeys) {
        if (k.includes("__silent")) continue;

        allGeneratedFallbacks.add(k);
        if (!blockContent.includes(`"${k}":`) && !blockContent.includes(` ${k}:`)) {
          if (!newKeys.includes(k)) {
            newKeys.push(k);
            added = true;
            console.log(`Auto-added missing log translation: ${k}`);
          }
        }
      }
    }

    if (added) {
      const entries = new Map<string, string>();
      let currentKey = "";
      let currentContent = "";

      const lines = blockContent.split("\n");
      for (const line of lines) {
        if (line.trim().length === 0) continue;
        const match = line.match(/^ {4}["']?([a-zA-Z0-9_]+)["']?\s*:/);
        if (match) {
          if (currentKey) {
            entries.set(currentKey, currentContent);
          }
          currentKey = match[1];
          currentContent = line;
        } else {
          currentContent += "\n" + line;
        }
      }
      if (currentKey) {
        entries.set(currentKey, currentContent);
      }

      for (const k of newKeys) {
        entries.set(k, `    "${k}": null,`);
      }

      const sortedKeys = Array.from(entries.keys()).sort((a, b) => a.localeCompare(b));

      const newLines = sortedKeys.map((k) => entries.get(k)!);
      const newBlock = `${match[1]}\n${newLines.join("\n")}${match[3]}`;
      enTsContent = enTsContent.replace(logsRegex, newBlock);
      fs.writeFileSync(enTsPath, enTsContent);
    }

    const lines = blockContent.split("\n");
    const staleKeys: string[] = [];
    for (const line of lines) {
      const lineMatch = line.match(/^ {4}["']?([a-zA-Z0-9_]+)["']?\s*:/);
      if (lineMatch) {
        const k = lineMatch[1];
        if (!allGeneratedFallbacks.has(k)) {
          staleKeys.push(k);
        }
      }
    }

    if (staleKeys.length > 0) {
      fs.writeFileSync(path.resolve(__dirname, "data/stale-keys.txt"), staleKeys.join("\n"));
      console.log(`Found ${staleKeys.length} stale keys. Wrote to data/stale-keys.txt`);
    } else {
      fs.writeFileSync(path.resolve(__dirname, "data/stale-keys.txt"), "");
      console.log("No stale keys found.");
    }
  } else {
    console.error("Could not parse en.ts logs block");
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  generateMatrix();
}
