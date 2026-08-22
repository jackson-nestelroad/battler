import fs from "fs";
import path from "path";
import { generateCombinatorics } from '../src/mapper.js';

const scraperConfig = JSON.parse(fs.readFileSync(path.resolve(import.meta.dirname, "data/scraper-config.json"), "utf-8"));

export function maskLog(line: string): string[] {
  line = line.trim();
  if (!line) return [];
  
  const parts = line.split('|').map(p => p.trim());
  let title = parts[0];
  
  const tags: string[] = [];
  const flags: string[] = [];
  
  for (let i = 1; i < parts.length; i++) {
    const p = parts[i];
    if (!p) continue;
    
    if (p.includes(':')) {
      const splitIndex = p.indexOf(':');
      let k = p.substring(0, splitIndex);
      const v = p.substring(splitIndex + 1);
      
      if (scraperConfig.excludeTags.includes(k)) continue;
      
      let keepSpecific = false;
      if (scraperConfig.keepSpecificTags.includes(k)) {
          keepSpecific = true;
      }
      
      if (keepSpecific) {
        tags.push(`${k}:${v}`);
      } else {
        tags.push(`${k}:*`);
      }
    } else {
      flags.push(p);
    }
  }

  for (const rule of scraperConfig.collapseDimensions || []) {
      if (title === rule.match.title) {
          for (let i = 0; i < tags.length; i++) {
              const [k] = tags[i].split(':');
              if (rule.collapse.includes(k)) {
                  tags[i] = `${k}:*`;
              }
          }
      }
  }

  const basePattern = [title, ...tags, ...flags].join('|');
  const results = [basePattern];

  for (const rule of scraperConfig.injectDimensions || []) {
      if (title === rule.match.title) {
          for (const injected of rule.inject) {
              results.push([title, ...tags, injected, ...flags].join('|'));
          }
      }
  }

  return results;
}

const RUST_TESTS_DIR = path.resolve(import.meta.dirname, "../../../battler/tests");

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
              const patterns = maskLog(item);
              for (const pattern of patterns) {
                allPatterns.add(pattern);
              }
            }
          }
        }
      } catch (e) {
      }
    }
  }

  return { raw: Array.from(allLogs), patterns: allPatterns };
}

function extractLogsFromFxlang(): string[] {
  const FXLANG_TESTS_DIR = path.resolve(import.meta.dirname, "../../../battler/tests/data");
  
  let files: string[] = [];
  try {
      files = fs.readdirSync(FXLANG_TESTS_DIR)
          .filter(f => f.endsWith(".txt"))
          .map(f => path.resolve(FXLANG_TESTS_DIR, f));
  } catch (e) {
      return [];
  }

  const fxlangLogs: string[] = [];

  for (const file of files) {
      const content = fs.readFileSync(file, "utf-8");
      const lines = content.split('\n');
      let inLogs = false;
      for (const line of lines) {
          if (line.trim() === 'LOGS:') {
              inLogs = true;
              continue;
          }
          if (inLogs && line.trim() && !line.startsWith('>>') && !line.startsWith('=')) {
              fxlangLogs.push(line.trim());
          }
          if (line.trim() === '') {
              inLogs = false;
          }
      }
  }

  return fxlangLogs;
}

function generateMatrix() {
  const extracted = extractLogsFromRs();
  const fxlangPatterns = extractLogsFromFxlang();
  
  for (const pattern of fxlangPatterns) {
      const maskedList = maskLog(pattern);
      for (const masked of maskedList) {
          extracted.patterns.add(masked);
      }
  }

  const finalMatrix: string[] = [];

  for (const pattern of Array.from(extracted.patterns).sort()) {
    const matching = extracted.raw.filter(log => maskLog(log).includes(pattern));
    finalMatrix.push(...matching.slice(0, 3));
  }

  fs.writeFileSync(path.resolve(import.meta.dirname, "logs-matrix.json"), JSON.stringify(finalMatrix, null, 2));
  fs.writeFileSync(path.resolve(import.meta.dirname, "unique-log-patterns.txt"), Array.from(extracted.patterns).sort().join("\n"));
  
  console.log(`Generated ${extracted.patterns.size} unique patterns in unique-log-patterns.txt`);
  console.log(`Generated ${finalMatrix.length} raw examples in logs-matrix.json`);

  const enTsPath = path.resolve(import.meta.dirname, "../locales/en.ts");
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
          const pParts = p.split('|');
          const pTitle = pParts.shift()!;
          const pTags = pParts.filter(x => x.includes(':'));
          const pFlags = pParts.filter(x => !x.includes(':'));
          
          const rawPatterns = generateCombinatorics(pTitle, pTags, pFlags);
          const allKeys = rawPatterns.map(rp => rp
            .replace(/\|/g, '__')
            .replace(/:/g, '_')
            .replace(/\*/g, 'any')
            .toLowerCase()
            .replace(/[^a-z0-9_]/g, '')
          );
          
          for (const k of allKeys) {
              if (k.includes('__silent')) continue;
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
          
          const lines = blockContent.split('\n');
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
                  currentContent += '\n' + line;
              }
          }
          if (currentKey) {
              entries.set(currentKey, currentContent);
          }

          for (const k of newKeys) {
              entries.set(k, `    "${k}": "[UNHANDLED]",`);
          }

          const sortedKeys = Array.from(entries.keys()).sort((a, b) => a.localeCompare(b));
          
          const newLines = sortedKeys.map(k => entries.get(k)!);
          const newBlock = `${match[1]}\n${newLines.join('\n')}${match[3]}`;
          enTsContent = enTsContent.replace(logsRegex, newBlock);
          fs.writeFileSync(enTsPath, enTsContent);
          console.log("Updated locales/en.ts with missing [UNHANDLED] logs (sorted).");
      }
      
      const undefinedKeys = new Set<string>();
      const lines = blockContent.split('\n');
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
                  let isProtected = false;
                  
                  const parts = k.split('__');
                  for (let i = 1; i < parts.length; i++) {
                      const prefix = parts.slice(0, i).join('__');
                      if (undefinedKeys.has(prefix)) {
                          isProtected = true;
                          break;
                      }
                  }
                  
                  if (!isProtected && parts.length > 1) {
                      const lastPart = parts[parts.length - 1];
                      if (lastPart.includes('_')) {
                          const tagParts = lastPart.split('_');
                          tagParts[tagParts.length - 1] = 'any';
                          const wildcardLast = tagParts.join('_');
                          const wildcardKey = [...parts.slice(0, -1), wildcardLast].join('__');
                          if (undefinedKeys.has(wildcardKey)) {
                              isProtected = true;
                          }
                      }
                  }

                  if (!isProtected) {
                      staleKeys.push(k);
                  }
              }
          }
      }
      
      if (staleKeys.length > 0) {
          fs.writeFileSync(path.resolve(import.meta.dirname, "data/stale-keys.txt"), staleKeys.join('\n'));
          console.log(`Found ${staleKeys.length} stale keys. Wrote to data/stale-keys.txt`);
      } else {
          fs.writeFileSync(path.resolve(import.meta.dirname, "data/stale-keys.txt"), "");
          console.log("No stale keys found.");
      }
      
  } else {
      console.error("Could not parse en.ts logs block");
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  generateMatrix();
}
