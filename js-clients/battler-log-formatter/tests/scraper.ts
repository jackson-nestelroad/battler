import { maskLog } from '../src/utils.js';
import fs from "fs";
import path from "path";


const RUST_TESTS_DIR = path.resolve(import.meta.dirname, "../../../battler/tests");

function readAllRsFiles(dir: string): string[] {
  let results: string[] = [];
  const list = fs.readdirSync(dir);
  for (const file of list) {
    const fullPath = path.resolve(dir, file);
    const stat = fs.statSync(fullPath);
    if (stat && stat.isDirectory()) {
      results = results.concat(readAllRsFiles(fullPath));
    } else if (fullPath.endsWith(".rs")) {
      results.push(fullPath);
    }
  }
  return results;
}


function extractLogsFromRs(): { raw: string[], patterns: Set<string> } {
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
              const pattern = maskLog(item);
              if (pattern) {
                allPatterns.add(pattern);
              }
            }
          }
        }
      } catch (e) {
      }
    }
  }

  return { 
    raw: Array.from(allLogs).sort(), 
    patterns: allPatterns 
  };
}

function extractLogsFromFxlang(): Set<string> {
  const fxlangLogs = new Set<string>();
  const BATTLE_DATA_DIR = path.resolve(import.meta.dirname, "../../../battle-data/data");

  const effectRegistry = new Map<string, { type: string, name: string, program: string, delegates: string[] }>();

  function getTypeFromCondition(condType?: string): string {
    if (!condType) return "condition";
    const lower = condType.toLowerCase();
    if (lower === "status" || lower === "weather" || lower === "volatile") return lower;
    return "condition";
  }

  function extractStrings(obj: any): string {
      if (typeof obj === 'string') return obj;
      if (Array.isArray(obj)) return obj.map(extractStrings).join('\n');
      if (obj && typeof obj === 'object') {
          return Object.values(obj).map(extractStrings).join('\n');
      }
      return '';
  }

  if (!fs.existsSync(BATTLE_DATA_DIR)) {
      console.error("Path does not exist:", BATTLE_DATA_DIR);
      return fxlangLogs;
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
    
    const processObj = (key: string, obj: any, typeOverride?: string) => {
        if (!obj || typeof obj !== 'object') return;
        const name = obj.name || key;
        const type = typeOverride || (obj.condition_type ? getTypeFromCondition(obj.condition_type) : defaultType);
        
        const programStr = extractStrings(obj.program || obj);
        
        const id = name.toLowerCase().replace(/[^a-z0-9]/g, '');

        const entry = {
            type,
            name,
            program: programStr,
            delegates: obj.delegates || []
        };

        effectRegistry.set(name, entry);
        effectRegistry.set(id, entry);
        effectRegistry.set(`${type}:${id}`, entry);
        // Also sometimes it's referenced as condition:id even if type is ability
        effectRegistry.set(`condition:${id}`, entry);
    };

    if (Array.isArray(content)) {
        for (const val of content) processObj(val.name || "Unknown", val);
    } else {
        for (const [key, val] of Object.entries(content)) {
            if (val && typeof val === 'object') {
                processObj(key, val);
            }
        }
    }
  }

  function getLogsForEffect(effectName: string, visited: Set<string>): string[] {
      if (visited.has(effectName)) return [];
      visited.add(effectName);
      
      const effect = effectRegistry.get(effectName);
      if (!effect) return [];
      
      const logs: string[] = [];
      const regex = /log_([a-z_]+)(?:\:\s*([a-z_]+))?/g;
      let match;
      while ((match = regex.exec(effect.program)) !== null) {
          const logType = match[1];
          const customArg = match[2];
          
          if (logType === "custom_effect" && customArg) {
              logs.push(customArg);
          } else {
              logs.push(`log_${logType}`);
          }
      }
      
      for (const delegate of (effect.delegates || [])) {
          logs.push(...getLogsForEffect(delegate, visited));
      }
      
      return logs;
  }

  const uniqueEffects = new Set(effectRegistry.values());
  for (const effect of uniqueEffects) {
      const name = effect.name;
      const logs = getLogsForEffect(name, new Set());
      for (const rawLog of logs) {
          const type = effect.type;
          
          let logKey = "";
          switch (rawLog) {
              case "log_ability": logKey = `ability|ability:${name}`; break;
              case "log_announce_item": logKey = `item|item:${name}`; break;
              case "log_activate": logKey = `activate|${type}:${name}`; break;
              case "log_block": logKey = `block|from:${type}:${name}`; break;
              case "log_cant": logKey = `cant|from:${type}:${name}`; break;
              case "log_immune": logKey = `immune|from:${type}:${name}`; break;
              case "log_fail": logKey = `fail|from:${type}:${name}`; break;
              case "log_prepare_move": logKey = `prepare|move:${name}`; break;
              case "log_single_move": logKey = `singlemove|move:${name}`; break;
              case "log_single_turn": logKey = `singleturn|${type}:${name}`; break;
              case "log_start": logKey = `start|${type}:${name}`; break;
              case "log_end": logKey = `end|${type}:${name}`; break;
              case "log_field_start": logKey = `fieldstart|${type}:${name}`; break;
              case "log_field_activate": logKey = `fieldactivate|${type}:${name}`; break;
              case "log_field_end": logKey = `fieldend|${type}:${name}`; break;
              case "log_side_start": logKey = `sidestart|${type}:${name}`; break;
              case "log_side_end": logKey = `sideend|${type}:${name}`; break;
              case "log_status": logKey = `status|status:${name}`; break;
              case "log_weather": logKey = `weather|weather:${name}`; break;
              case "log_fail_heal": logKey = `fail|from:${type}:${name}`; break;
              case "log_fail_unboost": logKey = `fail|from:${type}:${name}`; break;
              default:
                  if (!rawLog.startsWith("log_")) {
                      logKey = rawLog;
                  }
                  break;
          }
          if (logKey) {
            fxlangLogs.add(logKey);
          }
      }
  }

  return fxlangLogs;
}

function generateMatrix() {
  const extracted = extractLogsFromRs();
  const fxlangPatterns = extractLogsFromFxlang();
  
  for (const pattern of fxlangPatterns) {
      extracted.patterns.add(pattern);
  }

  const finalMatrix: string[] = [];

  for (const pattern of Array.from(extracted.patterns).sort()) {
    // Find up to 3 raw logs that match this pattern.
    // The mask algorithm drops details so we do a fuzzy match or just re-run maskLog on raw logs.
    const matching = extracted.raw.filter(log => maskLog(log) === pattern);
    finalMatrix.push(...matching.slice(0, 3));
  }

  fs.writeFileSync(path.resolve(import.meta.dirname, "logs-matrix.json"), JSON.stringify(finalMatrix, null, 2));
  fs.writeFileSync(path.resolve(import.meta.dirname, "unique-log-patterns.txt"), Array.from(extracted.patterns).sort().join("\n"));
  
  console.log(`Generated ${extracted.patterns.size} unique patterns in unique-log-patterns.txt`);
  console.log(`Generated ${finalMatrix.length} raw examples in logs-matrix.json`);

  // Auto-discover unhandled logs and inject them into en.ts
  const enTsPath = path.resolve(import.meta.dirname, "../locales/en.ts");
  let enTsContent = fs.readFileSync(enTsPath, "utf-8");
  
  const logsRegex = /(logs: \{)([\s\S]*?)(\n  \})/m;
  const match = enTsContent.match(logsRegex);
  if (match) {
      const blockContent = match[2];
      let added = false;
      const requireFromOf = ['damage', 'heal', 'sethp', 'item', 'itemend', 'itemstart', 'ability', 'abilityend', 'abilitystart', 'cant', 'fail', 'immune', 'block'];

      function getFallbacks(key: string): string[] {
          const parts = key.split('__');
          const title = parts[0];
          
          let baseParts = parts;
          if (title === 'abilitystart' || title === 'itemstart') {
              baseParts = baseParts.map(p => {
                  const pSplit = p.split('_');
                  if (pSplit[0] === 'ability' && pSplit.length > 1 && pSplit[1] !== 'any') return 'ability_any';
                  if (pSplit[0] === 'item' && pSplit.length > 1 && pSplit[1] !== 'any') return 'item_any';
                  return p;
              });
          }

          const results: string[] = [];

          if (title === 'ability' || title === 'item') {
              const hasName = baseParts.some(p => {
                  const pSplit = p.split('_');
                  return pSplit[0] === title && pSplit.length > 1 && pSplit[1] !== 'any';
              });
              const hasFrom = baseParts.some(p => {
                  const pSplit = p.split('_');
                  return pSplit[0] === 'from' && pSplit.length > 1 && pSplit[pSplit.length - 1] !== 'any';
              });
              
              if (hasName && hasFrom) {
                  results.push(baseParts.join('__'));
                  
                  const nameOnly = baseParts.map(p => {
                      const pSplit = p.split('_');
                      if (pSplit[0] === 'from' && pSplit.length > 1 && pSplit[pSplit.length - 1] !== 'any') return `from_${pSplit[1]}_any`;
                      return p;
                  });
                  results.push(nameOnly.join('__'));
                  
                  const fromOnly = baseParts.map(p => {
                      const pSplit = p.split('_');
                      if (pSplit[0] === title && pSplit.length > 1 && pSplit[1] !== 'any') return `${title}_any`;
                      return p;
                  });
                  results.push(fromOnly.join('__'));
              } else {
                  results.push(baseParts.join('__'));
              }
          } else {
              results.push(baseParts.join('__'));
          }

          const genericParts = baseParts.map(p => {
              const partsSplit = p.split('_');
              const k = partsSplit[0];
              const rest = partsSplit.slice(1);
              if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile'].includes(k) && rest.length > 0 && rest[0] !== 'any') {
                  return `${k}_any`;
              }
              if (k === 'from' && rest.length > 1 && rest[rest.length - 1] !== 'any') {
                  return `${k}_${rest[0]}_any`; // from_ability_any
              }
              return p;
          });
          const generic = genericParts.join('__');
          if (!results.includes(generic)) results.push(generic);

          const pureVars = ['by', 'exp', 'level', 'hp', 'atk', 'def', 'spa', 'spd', 'spe', 'stats', 'stat'];
          
          const finalResults: string[] = [];
          for (const pattern of results) {
              if (!finalResults.includes(pattern)) finalResults.push(pattern);
              
              const pParts = pattern.split('__');
              const pTitle = pParts[0];
              const pTags = pParts.slice(1); // Flags are also here but handled implicitly
              
              const noVarsParts = [pTitle, ...pTags.filter(t => !pureVars.includes(t.split('_')[0]))];
              const noVars = noVarsParts.join('__');
              if (!finalResults.includes(noVars)) finalResults.push(noVars);
              
              if (!requireFromOf.includes(pTitle)) {
                  const noFromOfParts = pTags.filter(t => {
                      const k = t.split('_')[0];
                      return k !== 'from' && k !== 'of';
                  });
                  const noFromOf = [pTitle, ...noFromOfParts].join('__');
                  if (!finalResults.includes(noFromOf)) finalResults.push(noFromOf);
                  
                  const noBothParts = noFromOfParts.filter(t => !pureVars.includes(t.split('_')[0]));
                  const noBoth = [pTitle, ...noBothParts].join('__');
                  if (!finalResults.includes(noBoth)) finalResults.push(noBoth);
              }
          }
          
          return finalResults;
      }

      const forceGeneric: Record<string, string[]> = {
          'catch': ['item'],
          'catchfailed': ['item'],
          'deductpp': ['move'],
          'didnotlearnmove': ['move'],
          'item': ['item'],
          'itemend': ['item'],
          'learnedmove': ['move'],
          'removevolatile': ['volatile'],
          'restorepp': ['move'],
          'setpp': ['move'],
          'swapsidecondition': ['move', 'condition']
      };

      const newKeys: string[] = [];

      for (const pattern of Array.from(extracted.patterns)) {
          let p = pattern;
          // Format pattern exactly like LogFormatter does
          const pParts = p.split('|');
          const pTitle = pParts.shift()!;
          const pTags = pParts.filter(x => x.includes(':'));
          const pFlags = pParts.filter(x => !x.includes(':'));
          pTags.sort();
          pFlags.sort();
          p = [pTitle, ...pTags, ...pFlags].join('|');

          const safePattern = p
            .replace(/\|/g, '__')
            .replace(/:/g, '_')
            .replace(/\*/g, 'any')
            .toLowerCase()
            .replace(/[^a-z0-9_]/g, '');
            
          const parts = safePattern.split('__');
          const title = parts[0];
          
          let targetKey = safePattern;
          if (!requireFromOf.includes(title)) {
              const noFromOfParts = parts.filter((p, i) => i === 0 || !['from', 'of'].includes(p.split('_')[0]));
              targetKey = noFromOfParts.join('__');
          }

          if (title in forceGeneric) {
              const targetParts = targetKey.split('__');
              const toGenericize = forceGeneric[title];
              const genericizedParts = targetParts.map(p => {
                  const [k, ...v] = p.split('_');
                  if (toGenericize.includes(k) && v.length > 0 && v[0] !== 'any') {
                      return `${k}_any`;
                  }
                  return p;
              });
              targetKey = genericizedParts.join('__');
          }

          const allKeys = getFallbacks(targetKey);
          for (const k of allKeys) {
              if (k.includes('__silent')) continue;
              
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
          // Parse the existing block into entries
          const entries = new Map<string, string>();
          let currentKey = "";
          let currentContent = "";
          
          const lines = blockContent.split('\n');
          for (const line of lines) {
              if (line.trim().length === 0) continue;
              
              // New key starts with exactly 4 spaces (or 4 spaces and a quote) followed by a word character
              const match = line.match(/^ {4}["']?([a-zA-Z0-9_]+)["']?\s*:/);
              if (match) {
                  if (currentKey) {
                      entries.set(currentKey, currentContent);
                  }
                  currentKey = match[1];
                  currentContent = line;
              } else {
                  currentContent += '\n' + line;
              }
          }
          if (currentKey) {
              entries.set(currentKey, currentContent);
          }

          // Add new keys
          for (const k of newKeys) {
              entries.set(k, `    "${k}": "[UNHANDLED]",`);
          }

          // Sort alphabetically
          const sortedKeys = Array.from(entries.keys()).sort((a, b) => a.localeCompare(b));
          
          const newLines = sortedKeys.map(k => entries.get(k)!);
          const newBlock = `${match[1]}\n${newLines.join('\n')}${match[3]}`;
          enTsContent = enTsContent.replace(logsRegex, newBlock);
          fs.writeFileSync(enTsPath, enTsContent);
          console.log("Updated locales/en.ts with missing [UNHANDLED] logs (sorted).");
      }
  } else {
      console.error("Could not parse en.ts logs block");
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  generateMatrix();
}
