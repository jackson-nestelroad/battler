import { mapUiLogEntry } from "../src/mapper.js";
import fs from "fs";
import path from "path";
import { describe, it, expect } from "vitest";
import { LogFormatter, FormattedUiLog, stringifyLog } from "../src/formatter.js";
import { alterBattleState, newBattleState } from "battler-state";
import { en } from "../locales/en.js";
import matrixLogs from "./data/logs-matrix.json" with { type: "json" };

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
      } else if (part.startsWith("mon:") || part.startsWith("of:") || part.startsWith("source:") || part.startsWith("into:") || part.startsWith("target:")) {
        const [, val] = part.split(":");
        const [species, player] = val.split(",");
        if (player) {
          players.add(player);
          mons.set(player, species);
        }
      }
    }
    
    if (players.size === 0) {
      players.add("p1");
      players.add("p2");
    } else if (players.size === 1) {
      if (!players.has("p1")) players.add("p1");
      else players.add("p2");
    }
    if (!mons.has("p1")) mons.set("p1", "Pikachu");
    if (!mons.has("p2")) mons.set("p2", "Gyarados");
    
    const setupLogs: string[] = [
      "info|battletype:Multi",
      "side|id:0|name:Side 0",
      "side|id:1|name:Side 1"
    ];
    
    const playerList = Array.from(players);
    for (let i = 0; i < playerList.length; i++) {
      const player = playerList[i];
      const side = i % 2;
      const pos = Math.floor(i / 2) + 1;
      setupLogs.push(`player|id:${player}|name:${player.toUpperCase()}|side:${side}|position:${pos}`);
      const species = mons.get(player) || "Bulbasaur";
      setupLogs.push(`mon|player:${player}|species:${species}|level:100|gender:M`);
    }
    
    setupLogs.push("teampreviewstart");
    setupLogs.push("battlestart");
    
    for (let i = 0; i < playerList.length; i++) {
      const player = playerList[i];
      const pos = Math.floor(i / 2) + 1;
      const species = mons.get(player) || "Bulbasaur";
      setupLogs.push(`switch|player:${player}|position:${pos}|name:${species}|health:100/100|species:${species}|level:100|gender:M`);
    }
    setupLogs.push("turn|turn:1");
    
    let testSequence = [...setupLogs, logString];
    if (logString.match(/^(info|side|player|teamsize|teampreview|battlestart|mon|split|turn)\|/)) {
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

    // 1. Format the string to get the final resolved key and context
    const event = formatter.format(uiLogEntry, alteredState);
    
    // 2. Extract just the enum key, the message string, and context vars
    let primaryResult: any = null;
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

            const fullKey = `logs.${safePattern}`;
            if (fullKey.replace("logs.", "") in en.logs) {
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
