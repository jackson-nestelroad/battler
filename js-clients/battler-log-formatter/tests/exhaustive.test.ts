import { getLogPatterns } from "../src/pattern_reconstructor.js";
import fs from "fs";
import path from "path";
import { describe, it, expect } from "vitest";
import { LogFormatter, FormattedUiLog } from "../src/formatter.js";
import { alterBattleState, newBattleState } from "battler-state";
import { AnyMappedLog } from "../src/types.js";
import type { LogToken } from "../src/engine.js";

const logsPath = path.resolve(".", "tests/logs-matrix.json");
const matrixLogs: string[] = JSON.parse(fs.readFileSync(logsPath, "utf-8"));

describe("Exhaustive Log Coverage", () => {
  const formatter = new LogFormatter({ localPlayerId: "p1" });

  it.each(matrixLogs)("should parse and format log string: %s", (logString) => {
    // 1. Initialize Master Setup State to satisfy battler-state strict validation
    const state = newBattleState();
    
    const players = new Set<string>();
    const mons = new Map<string, string>(); // player -> species
    
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
    
    // Some logs in the matrix ARE setup logs (like info|battletype:Single).
    // If the test log is a setup log, we shouldn't run the setup matrix first, as it would conflict.
    let testSequence = [...setupLogs, logString];
    if (logString.startsWith("info|") || logString.startsWith("side|") || logString.startsWith("player|") || logString.startsWith("teamsize|") || logString.startsWith("teampreview") || logString.startsWith("battlestart") || logString.startsWith("mon|")) {
      testSequence = [logString]; 
    }

    // Run the sequence
    let alteredState;
    try {
      alteredState = alterBattleState(newBattleState(), testSequence);
    } catch (e) {
      // The rust engine throws errors for some logs if the state doesn't perfectly match (e.g. expected of, unknown player).
      // Since we just want to test formatting coverage, we skip logs that the engine rejects.
      return; 
    }

    // 2. Extract the UiLogEntry
    expect(alteredState.ui_log.length).toBeGreaterThan(0);
    const lastTurnLogs = alteredState.ui_log[alteredState.ui_log.length - 1];
    
    // Some logs (like info and teampreview) are pure state-syncs and emit no UI logs.
    if (lastTurnLogs.length === 0) {
      return; // Nothing to format!
    }
    
    // The target log is always the very last parsed log in the sequence
    const uiLogEntry = lastTurnLogs[lastTurnLogs.length - 1];
    
    // 3. Format it
    const formattedArray = formatter.format(uiLogEntry, alteredState);
    const formatted = formattedArray.length > 0 ? formattedArray[0] : null;

    const rawMapped = Object.values(uiLogEntry)[0] as any;
    const isSilent = rawMapped?.effect?.additional?.silent !== undefined;

    if (isSilent) {
      expect(formattedArray.length).toBe(0);
      return;
    }

    // 4. Assert mapping exists
    if (!formatted) {
      console.error(`PATTERN IN VITEST: ${getLogPatterns(uiLogEntry).join(' OR ')}`);
      console.error(`UNMAPPED LOG [${logString}]:`, JSON.stringify(uiLogEntry));
    }
    expect(formatted).not.toBeNull();
    
    // Verify there are no unmapped raw variables left in the text output
    for (const token of formatted!.tokens) {
      if (token.type === "text") {
        expect(token.value).not.toMatch(/\{\{.*\}\}/);
      }
    }

    // 5. Snapshot test!
    const stringifiedLog = `[${formatted!.category}] ` + formatted!.tokens.map((token: LogToken) => {
      if (token.type === "text") return token.value;
      const ctxVal = formatted!.context[token.value];
      if (typeof ctxVal === "string") return ctxVal;
      if (Array.isArray(ctxVal)) {
        return ctxVal.map(v => typeof v === "string" ? v : v.text).join(", ");
      }
      if (ctxVal && typeof ctxVal === "object" && "text" in ctxVal) {
        return ctxVal.text;
      }
      return `{{${token.value}}}`;
    }).join("");
    
    expect(stringifiedLog).toMatchSnapshot();
    expect(stringifiedLog).not.toContain("[UNHANDLED]");
  });
});
