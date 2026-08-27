import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { generateCombinatorics } from "../src/mapper.js";
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
    let isMatch = true;
    for (const [matchK, matchV] of Object.entries(rule.match)) {
      if (matchK === "title") {
        if (title !== matchV) {
          isMatch = false;
          break;
        }
      } else {
        const hasTag = tags.some((t) => {
          if ((matchV as string) === "*") {
            return t.startsWith(`${matchK}:`);
          } else if ((matchV as string).endsWith("*")) {
            return t.startsWith(`${matchK}:${(matchV as string).slice(0, -1)}`);
          } else {
            return t === `${matchK}:${matchV as string}`;
          }
        });
        if (!hasTag) {
          isMatch = false;
          break;
        }
      }
    }

    if (isMatch) {
      finalTags = finalTags.filter((t) => !rule.strip.includes(t.split(":")[0]));
      finalFlags = finalFlags.filter((f) => !rule.strip.includes(f));
    }
  }

  // Phase 2: Collapse
  for (const rule of scraperConfig.rules || []) {
    if (!rule.collapse) continue;
    let isMatch = true;
    for (const [matchK, matchV] of Object.entries(rule.match)) {
      if (matchK === "title") {
        if (title !== matchV) {
          isMatch = false;
          break;
        }
      } else {
        const hasTag = tags.some((t) => {
          if ((matchV as string) === "*") {
            return t.startsWith(`${matchK}:`);
          } else if ((matchV as string).endsWith("*")) {
            return t.startsWith(`${matchK}:${(matchV as string).slice(0, -1)}`);
          } else {
            return t === `${matchK}:${matchV as string}`;
          }
        });
        if (!hasTag) {
          isMatch = false;
          break;
        }
      }
    }

    if (isMatch) {
      for (let i = 0; i < finalTags.length; i++) {
        const [k] = finalTags[i].split(":");
        if (rule.collapse.includes(k)) {
          finalTags[i] = `${k}:*`;
        }
      }
    }
  }

  const basePattern = [title, ...finalTags, ...finalFlags].join("|");
  const results = [basePattern];

  // Phase 3: Inject
  for (const rule of scraperConfig.rules || []) {
    if (!rule.inject) continue;
    let isMatch = true;
    for (const [matchK, matchV] of Object.entries(rule.match)) {
      if (matchK === "title") {
        if (title !== matchV) {
          isMatch = false;
          break;
        }
      } else {
        const hasTag = tags.some((t) => {
          if ((matchV as string) === "*") {
            return t.startsWith(`${matchK}:`);
          } else if ((matchV as string).endsWith("*")) {
            return t.startsWith(`${matchK}:${(matchV as string).slice(0, -1)}`);
          } else {
            return t === `${matchK}:${matchV as string}`;
          }
        });
        if (!hasTag) {
          isMatch = false;
          break;
        }
      }
    }

    if (isMatch) {
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
      const type = effect.type;
      let logName = rawLog.logType.replace(/_/g, "");
      if (logName === "announceitem") logName = "item";

      if (rawLog.logType === "fail_unboost") {
        logName = "fail_unboost";
      } else if (rawLog.logType === "fail_heal") {
        logName = "fail_heal";
      }

      const ALWAYS_FROM_EFFECT = new Set(scraperConfig.alwaysFromEffectLogs || []);
      const isFrom = rawLog.fromEffect || ALWAYS_FROM_EFFECT.has(logName);

      let prefix = type;
      if (effect.condition_type) {
        const lower = effect.condition_type.toLowerCase();
        if (lower === "built-in" || lower === "volatile") {
          prefix = isFrom ? "" : "condition";
        } else if (lower === "status" || lower === "weather" || lower === "zpower") {
          prefix = lower;
        } else {
          prefix = "condition";
        }
      }
      const typePrefix = prefix ? prefix + ":" : "";

      if (logName === "preparemove") logName = "prepare";
      if (logName === "usemove") logName = "move";

      const ALWAYS_NO_TAG = new Set(scraperConfig.alwaysNoTagLogs || []);
      const NO_TAG_UNLESS_FROM = new Set(scraperConfig.noTagUnlessFromLogs || []);
      const PREPEND_WILDCARDS = new Set(scraperConfig.prependWildcardTagLogs || []);
      const noTag = ALWAYS_NO_TAG.has(logName);

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
      } else if (noTag || (NO_TAG_UNLESS_FROM.has(logName) && !isFrom)) {
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
      for (const dSig of allDelegates) {
        // Ensure we only replace complete tags by surrounding with |
        // But tags can be preceded by | and followed by | or end of line.
        // Also consider they might be prefixed with `from:`
        if (pattern.includes(`|${dSig}|`) || pattern.endsWith(`|${dSig}`)) {
          const clonedPattern = pattern.split(`|${dSig}|`).join(`|${eSig}|`);
          const finalCloned = clonedPattern.endsWith(`|${dSig}`)
            ? clonedPattern.slice(0, -`|${dSig}`.length) + `|${eSig}`
            : clonedPattern;
          newClonedPatterns.add(finalCloned);
        }
        if (pattern.includes(`|from:${dSig}|`) || pattern.endsWith(`|from:${dSig}`)) {
          const clonedPattern = pattern.split(`|from:${dSig}|`).join(`|from:${eSig}|`);
          const finalCloned = clonedPattern.endsWith(`|from:${dSig}`)
            ? clonedPattern.slice(0, -`|from:${dSig}`.length) + `|from:${eSig}`
            : clonedPattern;
          newClonedPatterns.add(finalCloned);
        }
      }
    }
  }

  for (const cloned of newClonedPatterns) {
    const tpl = getTemplate(cloned);
    if (!validTemplates.has(tpl)) {
      // Do nothing
    }
    extracted.patterns.add(cloned);
  }

  const finalMatrix: string[] = [];

  for (const pattern of Array.from(extracted.patterns).sort()) {
    const matching = extracted.raw.filter((log) => maskLog(log).includes(pattern));
    finalMatrix.push(...matching.slice(0, 3));
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
      const allKeys = rawPatterns.map((rp) =>
        rp
          .replace(/\|/g, "__")
          .replace(/:/g, "_")
          .replace(/\*/g, "any")
          .toLowerCase()
          .replace(/[^a-z0-9_]/g, ""),
      );

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
        entries.set(k, `    "${k}": "[UNHANDLED]",`);
      }

      const sortedKeys = Array.from(entries.keys()).sort((a, b) => a.localeCompare(b));

      const newLines = sortedKeys.map((k) => entries.get(k)!);
      const newBlock = `${match[1]}\n${newLines.join("\n")}${match[3]}`;
      enTsContent = enTsContent.replace(logsRegex, newBlock);
      fs.writeFileSync(enTsPath, enTsContent);
      console.log("Updated locales/en.ts with missing [UNHANDLED] logs (sorted).");
    }

    const undefinedKeys = new Set<string>();
    const lines = blockContent.split("\n");
    for (const line of lines) {
      const match = line.match(/^ {4}["']?([a-zA-Z0-9_]+)["']?\s*:\s*undefined,/);
      if (match) {
        undefinedKeys.add(match[1]);
      }
    }

    const staleKeys: string[] = [];
    for (const line of lines) {
      const match = line.match(/^ {4}["']?([a-zA-Z0-9_]+)["']?\s*:/);
      if (match) {
        const k = match[1];
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
