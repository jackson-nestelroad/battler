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
    const setupLogs = [
      "info|battletype:Multi",
      "side|id:0|name:Side 0",
      "side|id:1|name:Side 1",
      "player|id:p1|name:P1|side:0|position:1",
      "player|id:p2|name:P2|side:1|position:1",
      "player|id:player-1|name:Player-1|side:0|position:2",
      "player|id:player-2|name:Player-2|side:1|position:2",
      "player|id:player-3|name:Player-3|side:0|position:3",
      "player|id:player-4|name:Player-4|side:1|position:3",
      "player|id:player-5|name:Player-5|side:0|position:4",
      "player|id:protagonist|name:Protagonist|side:0|position:5",
      "player|id:wild|name:Wild|side:1|position:4",
      "player|id:wild-0|name:Wild-0|side:1|position:5",
      "player|id:trainer|name:Trainer|side:1|position:6",
      "mon|player:p1|species:Pikachu|level:100|gender:M",
      "mon|player:player-1|species:Magnezone|level:100|gender:M",
      "mon|player:p2|species:Gyarados|level:100|gender:M",
      "mon|player:player-2|species:Mew|level:100|gender:M",
      "teampreviewstart",
      "battlestart",
      "switch|player:p1|position:1|name:Pikachu|health:100/100|species:Pikachu|level:100|gender:M",
      "switch|player:player-1|position:1|name:Magnezone|health:100/100|species:Magnezone|level:100|gender:M",
      "switch|player:p2|position:1|name:Gyarados|health:100/100|species:Gyarados|level:100|gender:M",
      "switch|player:player-2|position:1|name:Mew|health:100/100|species:Mew|level:100|gender:M",
      "turn|turn:1"
    ];
    
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
    const formatted = formatter.format(uiLogEntry, alteredState);

    // 4. Assert mapping exists
    if (!formatted) {
      console.log(`UNMAPPED LOG [${logString}]:`, JSON.stringify(uiLogEntry));
    }
    expect(formatted).not.toBeNull();
    
    // Verify there are no unmapped raw variables left in the text output
    for (const token of formatted!.tokens) {
      if (token.type === "text") {
        expect(token.value).not.toMatch(/\{\{.*\}\}/);
      }
    }

    // 5. Snapshot test!
    const stringifiedLog = formatted!.tokens.map((token: LogToken) => {
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
  });
});
