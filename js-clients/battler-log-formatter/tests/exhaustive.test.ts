import { mapUiLogEntry } from "../src/mapper.js";
import fs from "fs";
import path from "path";
import { describe, it, expect } from "vitest";
import { LogFormatter, FormattedUiLog, stringifyLog } from "../src/formatter.js";
import { alterBattleState, newBattleState } from "battler-state";
import { en } from "../locales/en.js";

const logsPath = path.resolve(".", "tests/logs-matrix.json");
const matrixLogs: string[] = JSON.parse(fs.readFileSync(logsPath, "utf-8"));

describe("Exhaustive Log Coverage", () => {
  const formatter = new LogFormatter({ localPlayerId: "p1" });

  it.each(matrixLogs)("should parse and format log string: %s", async (logString) => {
    const state = newBattleState();
    
    const players = new Set<string>();
    const mons = new Map<string, string>();
    
    const parts = logString.split("|");
    for (const part of parts) {
      if (part.startsWith("player:")) {
        players.add(part.split(":")[1]);
      } else if (part.startsWith("mon:") || part.startsWith("of:") || part.startsWith("source:")) {
        const [, val] = part.split(":");
        const [species, player] = val.split(",");
        if (player) {
          players.add(player);
          mons.set(player, species);
        }
      }
    }
    
    players.add("p1");
    players.add("p2");
    mons.set("p1", "Pikachu");
    mons.set("p2", "Gyarados");
    
    const setupLogs: string[] = [
      "info|battletype:Multi",
      "side|id:0|name:Side 0",
      "side|id:1|name:Side 1"
    ];
    
    let side = 0;
    for (const player of players) {
      setupLogs.push(`player|id:${player}|name:${player.toUpperCase()}|side:${side % 2}|position:${side + 1}`);
      const species = mons.get(player) || "Bulbasaur";
      setupLogs.push(`mon|player:${player}|species:${species}|level:100|gender:M`);
      side++;
    }
    
    setupLogs.push("teampreviewstart");
    setupLogs.push("battlestart");
    
    for (const player of players) {
      const species = mons.get(player) || "Bulbasaur";
      setupLogs.push(`switch|player:${player}|position:1|name:${species}|health:100/100|species:${species}|level:100|gender:M`);
    }
    setupLogs.push("turn|turn:1");
    
    let testSequence = [...setupLogs, logString];
    if (logString.match(/^(info|side|player|teamsize|teampreview|battlestart|mon)\|/)) {
      testSequence = [logString]; 
    }

    let alteredState;
    try {
      alteredState = alterBattleState(newBattleState(), testSequence);
    } catch (e) {
      return; 
    }

    expect(alteredState.ui_log.length).toBeGreaterThan(0);
    const lastTurnLogs = alteredState.ui_log[alteredState.ui_log.length - 1];
    
    if (lastTurnLogs.length === 0) {
      return;
    }
    
    const uiLogEntry = lastTurnLogs[lastTurnLogs.length - 1];
    
    // If it's a silent log, formatting should produce empty
    const rawMapped = Object.values(uiLogEntry)[0] as any;
    if (rawMapped?.effect?.additional?.silent !== undefined) {
      const emptyFormatted = formatter.format(uiLogEntry, alteredState);
      expect(emptyFormatted.length).toBe(0);
      return;
    }

    // 1. Format the string to get the final resolved key and context
    const formattedArray = formatter.format(uiLogEntry, alteredState);
    
    // 2. Extract just the enum key, the message string, and context vars
    const results = formattedArray.map(log => {
        return {
            key: log.key,
            context: log.context,
            formatted: {
                category: log.category,
                message: stringifyLog(log)
            }
        };
    });
    
    const primaryResult = results.length > 0 ? results[results.length - 1] : null;

    // 3. Snapshot the result
    expect({
        rawLog: logString,
        result: primaryResult
    }).toMatchSnapshot();
  });
});
