import { mapUiLogEntry } from "./src/mapper.js";
import fs from "fs";
import { newBattleState, alterBattleState } from "battler-state";
import { en } from "./locales/en.js";
import { getExpectedEnumKey } from "./src/utils.js";

const logsPath = "tests/logs-matrix.json";
const matrixLogs = JSON.parse(fs.readFileSync(logsPath, "utf-8"));

let mismatches = 0;
for (const logString of matrixLogs) {
    const expectedEnumKey = getExpectedEnumKey(logString);
    if (!expectedEnumKey || ['switch', 'switchout', 'replace', 'useitem', 'waiting__on_any', 'sethp'].includes(expectedEnumKey)) continue;

    // Build state
    const players = new Set();
    const mons = new Map();
    const parts = logString.split("|");
    for (const part of parts) {
      if (part.startsWith("player:")) players.add(part.split(":")[1]);
      else if (part.startsWith("mon:") || part.startsWith("of:") || part.startsWith("source:")) {
        const [, val] = part.split(":");
        const [species, player] = val.split(",");
        if (player) { players.add(player); mons.set(player, species); }
      }
    }
    players.add("p1"); players.add("p2");
    mons.set("p1", "Pikachu"); mons.set("p2", "Gyarados");
    
    const setupLogs = ["info|battletype:Multi", "side|id:0|name:Side 0", "side|id:1|name:Side 1"];
    let side = 0;
    for (const player of players) {
      setupLogs.push(`player|id:${player}|name:${player.toUpperCase()}|side:${side % 2}|position:${side + 1}`);
      const species = mons.get(player) || "Bulbasaur";
      setupLogs.push(`mon|player:${player}|species:${species}|level:100|gender:M`);
      side++;
    }
    setupLogs.push("teampreviewstart"); setupLogs.push("battlestart");
    for (const player of players) {
      const species = mons.get(player) || "Bulbasaur";
      setupLogs.push(`switch|player:${player}|position:1|name:${species}|health:100/100|species:${species}|level:100|gender:M`);
    }
    setupLogs.push("turn|turn:1");
    
    let testSequence = [...setupLogs, logString];
    if (logString.match(/^(info|side|player|teamsize|teampreview|battlestart|mon)\|/)) testSequence = [logString]; 

    let alteredState;
    try { alteredState = alterBattleState(newBattleState(), testSequence); } catch (e) { continue; }

    const lastTurnLogs = alteredState.ui_log[alteredState.ui_log.length - 1];
    if (lastTurnLogs.length === 0) continue;
    const uiLogEntry = lastTurnLogs[lastTurnLogs.length - 1];
    const rawMapped = Object.values(uiLogEntry)[0];
    if (rawMapped?.effect?.additional?.silent !== undefined) continue;

    const patterns = mapUiLogEntry(uiLogEntry, alteredState)?.patterns || [];
    let resolvedEnumKey = null;
    if (patterns.length > 0) {
      for (const p of patterns) {
        const safePattern = p.replace(/\|/g, '__').replace(/:/g, '_').replace(/\*/g, 'any').toLowerCase().replace(/[^a-z0-9_]/g, '');
        if (Object.hasOwn(en.logs, safePattern)) {
          resolvedEnumKey = safePattern;
          break;
        }
      }
    }

    if (resolvedEnumKey !== expectedEnumKey) {
        console.log(`Mismatch for ${logString}`);
        console.log(`Expected: ${expectedEnumKey}`);
        console.log(`Resolved: ${resolvedEnumKey}`);
        console.log(`Mapped Keys: ${patterns.map(p => p.replace(/\|/g, '__').replace(/:/g, '_').replace(/\*/g, 'any').toLowerCase().replace(/[^a-z0-9_]/g, '')).join(", ")}`);
        console.log("---");
        mismatches++;
    }
}
console.log(`Total mismatches: ${mismatches}`);
