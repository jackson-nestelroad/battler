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

function generateMatrix() {
  const extracted = extractLogsFromRs();

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
  
  const logsRegex = /("logs": \{)([\s\S]*?)(\n  \})/m;
  const match = enTsContent.match(logsRegex);
  if (match) {
      const lines = match[2].split('\n').filter(l => l.trim().length > 0);
      const currentLogs: Record<string, string> = {};
      for (const line of lines) {
          const parts = line.split(':');
          if (parts.length >= 2) {
              const keyMatch = parts[0].match(/"([^"]+)"/);
              if (keyMatch) {
                  const key = keyMatch[1];
                  const value = parts.slice(1).join(':').trim().replace(/,$/, '');
                  currentLogs[key] = value;
              }
          }
      }


      let added = false;
      const requireFromOf = ['damage', 'heal', 'sethp', 'item', 'itemend', 'itemstart', 'ability', 'abilityend', 'abilitystart', 'cant', 'fail', 'immune', 'block'];

      function getFallbacks(key: string): string[] {
          const parts = key.split('__');
          const title = parts[0];
          
          let baseParts = parts;
          if (title === 'abilitystart' || title === 'itemstart') {
              baseParts = baseParts.map(p => {
                  const pSplit = p.split('_');
                  if (pSplit[0] === 'ability' && pSplit.length > 1 && pSplit[1] !== 'ANY') return 'ability_ANY';
                  if (pSplit[0] === 'item' && pSplit.length > 1 && pSplit[1] !== 'ANY') return 'item_ANY';
                  return p;
              });
          }

          const results: string[] = [];

          if (title === 'ability' || title === 'item') {
              const hasName = baseParts.some(p => {
                  const pSplit = p.split('_');
                  return pSplit[0] === title && pSplit.length > 1 && pSplit[1] !== 'ANY';
              });
              const hasFrom = baseParts.some(p => {
                  const pSplit = p.split('_');
                  return pSplit[0] === 'from' && pSplit.length > 1 && pSplit[pSplit.length - 1] !== 'ANY';
              });
              
              if (hasName && hasFrom) {
                  results.push(baseParts.join('__'));
                  
                  const nameOnly = baseParts.map(p => {
                      const pSplit = p.split('_');
                      if (pSplit[0] === 'from' && pSplit.length > 1 && pSplit[pSplit.length - 1] !== 'ANY') return `from_${pSplit[1]}_ANY`;
                      return p;
                  });
                  results.push(nameOnly.join('__'));
                  
                  const fromOnly = baseParts.map(p => {
                      const pSplit = p.split('_');
                      if (pSplit[0] === title && pSplit.length > 1 && pSplit[1] !== 'ANY') return `${title}_ANY`;
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
              if (['ability', 'item', 'move', 'effect', 'condition', 'weather', 'status', 'volatile'].includes(k) && rest.length > 0 && rest[0] !== 'ANY') {
                  return `${k}_ANY`;
              }
              if (k === 'from' && rest.length > 1 && rest[rest.length - 1] !== 'ANY') {
                  return `${k}_${rest[0]}_ANY`; // from_ability_ANY
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

      for (const pattern of Array.from(extracted.patterns)) {
          const safePattern = pattern.replace(/\|/g, '__').replace(/:/g, '_').replace(/\*/g, 'ANY').replace(/\[/g, '').replace(/\]/g, '');
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
                  if (toGenericize.includes(k) && v.length > 0 && v[0] !== 'ANY') {
                      return `${k}_ANY`;
                  }
                  return p;
              });
              targetKey = genericizedParts.join('__');
          }

          const allKeys = getFallbacks(targetKey);
          for (const k of allKeys) {
              if (k.includes('__silent')) continue;
              if (!(k in currentLogs)) {
                  currentLogs[k] = '"[UNHANDLED]"';
                  added = true;
                  console.log(`Auto-added missing log translation: ${k}`);
              }
          }
      }

      if (added) {
          const newLines = Object.keys(currentLogs).sort((a,b) => a.localeCompare(b)).map(k => `    "${k}": ${currentLogs[k]},`);
          const newBlock = `${match[1]}\n${newLines.join('\n')}${match[3]}`;
          enTsContent = enTsContent.replace(logsRegex, newBlock);
          fs.writeFileSync(enTsPath, enTsContent);
          console.log("Updated locales/en.ts with missing [UNHANDLED] logs.");
      }
  } else {
      console.error("Could not parse en.ts logs block");
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  generateMatrix();
}
