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
      for (const pattern of Array.from(extracted.patterns)) {
          const safePattern = pattern.replace(/\|/g, '__').replace(/:/g, '_').replace(/\*/g, 'ANY').replace(/\[/g, '').replace(/\]/g, '');
          if (!(safePattern in currentLogs)) {
              currentLogs[safePattern] = '"[UNHANDLED]"';
              added = true;
              console.log(`Auto-added missing log translation: ${safePattern}`);
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
