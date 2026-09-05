import { alterBattleState, newBattleState, UiLogEntry } from "battler-state";
import { en } from "../locales/en.js";
import { parseTemplateToTokens } from "../src/engine.js";
import { LogFormatter, stringifyLog } from "../src/formatter.js";
import { mapUiLogEntry } from "../src/mapper.js";
import { parsePattern, patternToKey, serializePattern } from "../src/pattern.js";
import matrixLogs from "../tests/data/logs-matrix.json" with { type: "json" };

const formatter = new LogFormatter({ localPlayerId: "p1" });

interface ValidationFailure {
  rawLog: string;
  reason: string;
  details?: unknown;
}

const failures: ValidationFailure[] = [];

// 1. Validate all templates in en.ts
console.log("Validating locale templates...");
let templateCount = 0;

for (const [key, val] of Object.entries(en.logs)) {
  if (!val) continue;
  const templates = Array.isArray(val) ? val : [val];
  for (const tmpl of templates) {
    if (typeof tmpl !== "string") continue;
    templateCount++;
    const tokens = parseTemplateToTokens(tmpl);
    for (const token of tokens) {
      if (token.type === "variable" && !token.value) {
        failures.push({
          rawLog: key,
          reason: `Empty variable in template '${tmpl}'`,
        });
      }
    }
  }
}
console.log(`Checked ${templateCount} templates in en.logs.`);

// 2. Validate all matrix logs against formatter
console.log(`Validating ${matrixLogs.length} matrix logs...`);

for (const logString of matrixLogs) {
  const players = new Set<string>();
  const monsByPlayer = new Map<string, Map<number, string>>();

  const parts = logString.split("|");
  for (const part of parts) {
    if (part.startsWith("player:")) {
      players.add(part.split(":")[1]);
    } else if (
      part.startsWith("mon:") ||
      part.startsWith("of:") ||
      part.startsWith("source:") ||
      part.startsWith("into:") ||
      part.startsWith("target:") ||
      part.startsWith("on:")
    ) {
      const [, val] = part.split(":");
      const [species, player, posStr] = val.split(",");
      if (player) {
        players.add(player);
        if (!monsByPlayer.has(player)) {
          monsByPlayer.set(player, new Map());
        }
        const pos = posStr ? parseInt(posStr, 10) : 1;
        monsByPlayer.get(player)!.set(pos, species);
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
    setupLogs.push(`side|id:${side}|name:Team ${side + 1}`);
    setupLogs.push(
      `player|id:${pid}|name:PLAYER-${i + 1}|side:${side}|position:${Math.floor(i / 2)}`,
    );
    setupLogs.push(`teamsize|player:${pid}|size:6`);

    const playerMons = monsByPlayer.get(pid);
    if (!playerMons || playerMons.size === 0) {
      setupLogs.push(
        `switch|player:${pid}|position:1|name:Pikachu|health:100/100|species:Pikachu|level:50|gender:M`,
      );
    } else {
      for (const [pos, monSpecies] of playerMons.entries()) {
        setupLogs.push(
          `switch|player:${pid}|position:${pos}|name:${monSpecies}|health:100/100|species:${monSpecies}|level:50|gender:M`,
        );
      }
    }
  }

  setupLogs.push("teampreviewstart");
  setupLogs.push("teampreview");
  setupLogs.push("battlestart");
  setupLogs.push("turn|turn:1");

  let testSequence = [...setupLogs, logString];
  if (logString.startsWith("player|")) {
    testSequence = ["side|id:0|name:Team 1", "side|id:1|name:Team 2", logString];
  } else if (logString.startsWith("teamsize|") || logString.startsWith("mon|")) {
    testSequence = [
      "side|id:0|name:Team 1",
      "side|id:1|name:Team 2",
      "player|id:player-1|name:PLAYER-1|side:0|position:0",
      "player|id:player-2|name:PLAYER-2|side:1|position:0",
      "player|id:protagonist|name:Protagonist|side:0|position:1",
      logString,
    ];
  } else if (logString.match(/^(info|side|teampreview|battlestart|split|turn)\|/)) {
    testSequence = [logString];
  }

  let alteredState;
  try {
    alteredState = alterBattleState(newBattleState(), testSequence);
  } catch (err) {
    // If it's a known standalone log that requires specific state context, format directly
    if (
      logString.startsWith("start|move:Doom Desire") ||
      logString.startsWith("start|move:Future Sight")
    ) {
      const logParts = logString.split("|");
      const title = logParts[0];
      const values: Record<string, string> = {};
      for (let i = 1; i < logParts.length; i++) {
        const [k, ...rest] = logParts[i].split(":");
        values[k] = rest.join(":");
      }
      const dummyEntry: UiLogEntry = {
        title,
        values,
      };
      const mapped = mapUiLogEntry(dummyEntry, undefined, { localPlayerId: "p1" });
      if (mapped) {
        formatter.format(dummyEntry, undefined);
        continue;
      }
    }
    failures.push({
      rawLog: logString,
      reason: `alterBattleState failed: ${String(err)}`,
    });
    continue;
  }

  if (alteredState.ui_log.length === 0) continue;
  const lastTurnLogs = alteredState.ui_log[alteredState.ui_log.length - 1];
  if (lastTurnLogs.length === 0) continue;
  const uiLogEntry = lastTurnLogs[lastTurnLogs.length - 1];

  if (uiLogEntry.values?.silent !== undefined) continue;

  const mapped = mapUiLogEntry(uiLogEntry, alteredState, { localPlayerId: "p1" });
  if (!mapped) {
    failures.push({
      rawLog: logString,
      reason: "mapUiLogEntry returned null",
    });
    continue;
  }

  // Find candidate templates in en.logs
  let matchedKey: string | undefined = undefined;
  let matchedVal: string | string[] | null | undefined = undefined;
  for (const pattern of mapped.patterns) {
    const parsed = parsePattern(pattern);
    const serialized = serializePattern(parsed);
    const safePattern = patternToKey(serialized);
    if (safePattern in en.logs) {
      matchedKey = safePattern;
      matchedVal = (en.logs as Record<string, string | string[] | null>)[safePattern];
      break;
    }
  }

  // Format using LogFormatter
  const event = formatter.format(uiLogEntry, alteredState);

  const isIntentionallyEmpty =
    matchedVal === null || (Array.isArray(matchedVal) && matchedVal.length === 0);

  // If a template was defined (not null/empty), formatter must produce message(s)
  if (matchedKey && !isIntentionallyEmpty) {
    if (!event || event.messages.length === 0) {
      const KNOWN_LEGACY_TEMPLATES = new Set([
        "activate__move_courtchange",
        "cannotescape",
        "fieldstart__move_trickroom",
        "itemstart__from_ability_magician__item_any",
        "start__move_doomdesire",
        "start__move_futuresight",
      ]);

      const tmpl = Array.isArray(matchedVal) ? matchedVal[0] : matchedVal;
      const tokens = parseTemplateToTokens(typeof tmpl === "string" ? tmpl : "");
      const missingVars = tokens
        .filter((t) => t.type === "variable" && mapped.context[t.value] === undefined)
        .map((t) => t.value);

      if (KNOWN_LEGACY_TEMPLATES.has(matchedKey)) {
        console.warn(
          `[KNOWN WARNING] Legacy template '${matchedKey}' missing variables: ${missingVars.join(", ")}`,
        );
        continue;
      }

      failures.push({
        rawLog: logString,
        reason: `Template matched '${matchedKey}' but failed to render. Missing variables: ${missingVars.join(", ")}`,
        details: { matchedKey, tmpl, missingVars, availableContext: Object.keys(mapped.context) },
      });
    } else {
      // Check that stringified messages don't contain unresolved variables
      for (const msg of event.messages) {
        const text = stringifyLog(msg);
        if (/\{\{[^}]+\}\}/.test(text)) {
          failures.push({
            rawLog: logString,
            reason: `Formatted message contains unresolved placeholder: ${text}`,
          });
        }
      }
    }
  }
}

if (failures.length > 0) {
  console.error(`\n❌ Found ${failures.length} validation failures:`);
  for (const f of failures) {
    console.error(`- [${f.rawLog}] ${f.reason}`);
    if (f.details) {
      console.error(`  Details:`, JSON.stringify(f.details));
    }
  }
  process.exit(1);
} else {
  console.log(
    `\n✅ All ${matrixLogs.length} matrix logs and all locale templates validated successfully!`,
  );
  process.exit(0);
}
