import fs from "fs";
import path from "path";

export interface LogDefinition {
  name: string;
  required: string[];
  optional: string[];
  flags: string[];
}

export function parseBattleLogsMd(filePath: string): LogDefinition[] {
  const content = fs.readFileSync(filePath, "utf-8");
  const definitions: LogDefinition[] = [];
  
  const sections = content.split("#### `");
  sections.shift(); // Remove content before first log

  for (const section of sections) {
    const lines = section.split("\n");
    const headerLine = lines[0];
    const nameMatches = headerLine.match(/`?([a-zA-Z0-9]+)`?/g);
    if (!nameMatches) continue;

    const names = nameMatches.map(n => n.replace(/`/g, "").trim()).filter(n => n.length > 0 && n !== "and");

    const required: string[] = [];
    const optional: string[] = [];
    const flags: string[] = [];

    let currentSection: "required" | "optional" | "flags" | null = null;

    for (let i = 1; i < lines.length; i++) {
      const line = lines[i].trim();
      if (line.startsWith("#### `") || line.startsWith("### ")) break;

      if (line.includes("- **Required fields**:")) {
        currentSection = "required";
      } else if (line.includes("- **Optional fields**:") || line.includes("- **Optional fields / flags**:")) {
        currentSection = "optional";
      } else if (line.includes("- **Optional flags**:")) {
        currentSection = "flags";
      } else if (line.startsWith("- `") && currentSection) {
        const match = line.match(/^- `([^:`]+)/);
        if (match) {
          const key = match[1];
          if (currentSection === "required") required.push(key);
          else if (currentSection === "optional") optional.push(key);
          else if (currentSection === "flags") flags.push(key);
        }
      } else if (line.startsWith("- **Examples**:")) {
        currentSection = null;
      }
    }

    for (const name of names) {
      definitions.push({ name, required, optional, flags });
    }
  }

  return definitions;
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
    } else if (fullPath.endsWith(".rs")) {
      results.push(fullPath);
    }
  }
  return results;
}

function extractLogsFromRs(): string[] {
  const files = readAllRsFiles(RUST_TESTS_DIR);
  const allLogs: Set<string> = new Set();

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
            }
          }
        }
      } catch (e) {
      }
    }
  }

  return Array.from(allLogs).sort();
}

function generateMatrix() {
  const mdDefs = parseBattleLogsMd(path.resolve(import.meta.dirname, "../../../battle-logs.md"));
  const rustLogs = extractLogsFromRs();

  const finalMatrix: string[] = [];
  const missingDefs: string[] = [];

  for (const def of mdDefs) {
    const matchingLogs = rustLogs.filter(log => log === def.name || log.startsWith(def.name + "|"));
    
    if (matchingLogs.length === 0) {
      missingDefs.push(def.name);
    } else {
      finalMatrix.push(...matchingLogs.slice(0, 3));
    }
  }

  fs.writeFileSync(path.resolve(import.meta.dirname, "logs-matrix.json"), JSON.stringify(finalMatrix, null, 2));
  
  console.log(`Generated ${finalMatrix.length} log permutations in logs-matrix.json`);
  if (missingDefs.length > 0) {
    console.log(`WARNING: Found 0 Rust test examples for the following ${missingDefs.length} logs:`);
    console.log(missingDefs.join(", "));
  }
}

import { fileURLToPath } from 'url';

if (import.meta.url === `file://${process.argv[1]}`) {
  generateMatrix();
}
