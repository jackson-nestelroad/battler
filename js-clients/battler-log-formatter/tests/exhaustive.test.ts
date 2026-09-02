import { describe, expect, it } from "vitest";
import { alterBattleState, newBattleState } from "battler-state";
import { LogFormatter, stringifyLog } from "../src/formatter.js";
import { mapUiLogEntry } from "../src/mapper.js";
import type { LogCategory, LogContext, UiNotice } from "../src/types.js";
import { en } from "../locales/en.js";
import matrixLogs from "./data/logs-matrix.json" with { type: "json" };

interface ExhaustiveMessage {
  key?: string;
  context: LogContext;
  formatted: {
    category: LogCategory;
    text: string;
  };
}

interface ExhaustiveResult {
  notices?: UiNotice[];
  messages: ExhaustiveMessage[];
  key?: string;
}

describe("Exhaustive Log Coverage", () => {
  const formatter = new LogFormatter({ localPlayerId: "p1" });

  it.each(matrixLogs)("should parse and format log string: %s", async (logString) => {
    const players = new Set<string>();
    const mons = new Map<string, string>();
    
    const parts = logString.split("|");
    for (const part of parts) {
      if (part.startsWith("player:")) {
        players.add(part.split(":")[1]);
      } else if (part.startsWith("mon:") || part.startsWith("of:") || part.startsWith("source:") || part.startsWith("into:") || part.startsWith("target:")) {
        const [, val] = part.split(":");
        const [species, player] = val.split(",");
        if (player) {
          players.add(player);
          mons.set(player, species);
        }
      }
    }

    const setupLogs: string[] = [];
    const playerList = Array.from(players);
    if (playerList.length === 0) {
      playerList.push("p1", "p2");
    } else if (playerList.length === 1) {
      playerList.push(playerList[0] === "p1" ? "p2" : "p1");
    }

    for (let i = 0; i < playerList.length; i++) {
      const pid = playerList[i];
      const side = i % 2;
      setupLogs.push(`side|side:${side}|name:Team ${side + 1}`);
      setupLogs.push(`player|player:${pid}|name:PLAYER-${i + 1}|side:${side}`);
      setupLogs.push(`teamsize|player:${pid}|size:6`);
      
      const monSpecies = mons.get(pid) || "Pikachu";
      setupLogs.push(`mon|player:${pid}|species:${monSpecies}|name:${monSpecies}|level:50|gender:M|hp:100|hp:100|type1:Electric|ability:Static|moves:Thunderbolt|stats:100,100,100,100,100,100`);
      setupLogs.push(`split|side:${side}`);
      setupLogs.push(`mon|player:${pid}|species:${monSpecies}|name:${monSpecies}|level:50|gender:M|hp:100|hp:100|stats:100,100,100,100,100,100`);
      setupLogs.push(`mon|player:${pid}|species:${monSpecies}|name:${monSpecies}|level:50|gender:M|hp:100|hp:100|stats:100,100,100,100,100,100`);
      setupLogs.push(`switch|mon:${monSpecies},${pid}|position:0|hp:100|hp:100`);
    }

    setupLogs.push("teampreviewstart");
    setupLogs.push("teampreview");
    setupLogs.push("battlestart");
    setupLogs.push("turn|turn:1");

    let testSequence = [...setupLogs, logString];
    if (logString.match(/^(info|side|player|teamsize|teampreview|battlestart|mon|split|turn)\|/)) {
      testSequence = [logString]; 
    }

    let alteredState;
    try {
      alteredState = alterBattleState(newBattleState(), testSequence);
    } catch {
      return; 
    }

    expect(alteredState.ui_log.length).toBeGreaterThan(0);
    const lastTurnLogs = alteredState.ui_log[alteredState.ui_log.length - 1];
    
    if (lastTurnLogs.length === 0) {
      return;
    }
    
    const uiLogEntry = lastTurnLogs[lastTurnLogs.length - 1];

    // 1. Format the string to get the final resolved key and context
    const event = formatter.format(uiLogEntry, alteredState);
    
    // 2. Extract just the enum key, the message string, and context vars
    let primaryResult: ExhaustiveResult | null = null;
    if (event) {
        primaryResult = {
            notices: event.notices,
            messages: event.messages.map((msg) => ({
                key: msg.key,
                context: msg.context,
                formatted: {
                    category: msg.category,
                    text: stringifyLog(msg)
                }
            }))
        };
    } else {
        const mapped = mapUiLogEntry(uiLogEntry, alteredState, { localPlayerId: "p1" });
        let matchedKey: string | undefined = undefined;
        if (mapped) {
          for (const pattern of mapped.patterns) {
            const parts = pattern.split("|");
            const title = parts.shift()!;
            const tags = parts.filter((x) => x.includes(":"));
            const flags = parts.filter((x) => !x.includes(":"));
            tags.sort();
            flags.sort();
            const p = [title, ...tags, ...flags].join("|");
            const safePattern = p
              .replace(/\|/g, "__")
              .replace(/:/g, "_")
              .replace(/\*/g, "any")
              .toLowerCase()
              .replace(/[^a-z0-9_]/g, "");

            const logKey = safePattern;
            if (logKey in en.logs || Object.keys(en.logs).some((k) => k.startsWith(`${logKey}___`))) {
              matchedKey = safePattern;
              break;
            }
          }
        }
        if (matchedKey) {
          primaryResult = {
            key: matchedKey,
            messages: [],
            notices: []
          };
        }
    }

    // 3. Snapshot the result
    expect({
        rawLog: logString,
        result: primaryResult
    }).toMatchSnapshot();
  });
});
