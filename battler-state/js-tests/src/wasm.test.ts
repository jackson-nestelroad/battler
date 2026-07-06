import { describe, it } from "node:test";
import assert from "node:assert";
import { newBattleState, alterBattleState, stateSelectors } from "battler-state";

const startLogs = [
  "info|battletype:singles",
  "side|id:0|name:Side 1",
  "side|id:1|name:Side 2",
  "maxsidelength|length:1",
  "player|id:player-1|name:Player 1|side:0|position:0",
  "player|id:player-2|name:Player 2|side:1|position:0",
  "teamsize|player:player-1|size:3",
  "teamsize|player:player-2|size:3",
  "battlestart",
  "turn|turn:1",
];

describe("battler-state WASM integration", () => {
  it("creates initial state and alters state with array of logs", () => {
    const initialState = newBattleState();
    const state = alterBattleState(initialState, startLogs);

    assert.ok(state);
    assert.strictEqual(state.battle_type, "singles");
    assert.strictEqual(state.turn, 1);
    assert.ok(Array.isArray(state.ui_log));
    assert.strictEqual(state.ui_log.length, 2); // turn 0 and turn 1 logs
  });

  it("updates state incrementally", () => {
    const initialState = newBattleState();
    const state1 = alterBattleState(initialState, startLogs);

    const state2 = alterBattleState(state1, [...startLogs, "turn|turn:2"]);

    assert.strictEqual(state2.turn, 2);
    assert.strictEqual(state2.battle_type, "singles");
  });

  it("resolves state using stateSelectors", () => {
    const initialState = newBattleState();
    const state = alterBattleState(initialState, [
      "info|battletype:singles",
      "side|id:0|name:Side 1",
      "side|id:1|name:Side 2",
      "maxsidelength|length:1",
      "player|id:player-1|name:Player 1|side:0|position:0",
      "player|id:player-2|name:Player 2|side:1|position:0",
      "teamsize|player:player-1|size:3",
      "teamsize|player:player-2|size:3",
      "battlestart",
      "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5",
      "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5",
      "boost|mon:Squirtle,player-1,1|stat:atk|by:2",
      "weather|weather:Rain",
      "fieldstart|condition:Grassy Terrain",
      "turn|turn:1",
    ]);

    const squirtleRef = {
      player: "player-1",
      mon_index: 0,
      battle_appearance_index: 0,
    };

    // 1. Weather & Terrain
    assert.strictEqual(stateSelectors.fieldWeather(state), "Rain");
    assert.strictEqual(stateSelectors.fieldTerrain(state), "Grassy Terrain");
    assert.deepStrictEqual(stateSelectors.fieldConditions(state), []);

    // 2. Sides & Players
    assert.ok(stateSelectors.side(state, 0));
    assert.deepStrictEqual(stateSelectors.sideConditions(state, 0), []);
    assert.strictEqual(stateSelectors.sideForMon(state, squirtleRef), 0);
    assert.strictEqual(stateSelectors.sideForPlayer(state, "player-2"), 1);
    assert.ok(stateSelectors.player(state, "player-1"));
    assert.ok(stateSelectors.mon(state, squirtleRef));

    // 3. Mon Level, Health, Status, Ability
    assert.strictEqual(stateSelectors.monLevel(state, squirtleRef), 5);
    assert.deepStrictEqual(stateSelectors.monHealth(state, squirtleRef), [100, 100]);
    assert.strictEqual(stateSelectors.monStatus(state, squirtleRef), null);
    assert.strictEqual(stateSelectors.monSpecies(state, squirtleRef), "Squirtle");

    // 4. Boosts & Conditions & Active Position
    assert.deepStrictEqual(stateSelectors.monBoosts(state, squirtleRef), { atk: 2 });
    assert.deepStrictEqual(stateSelectors.monConditions(state, squirtleRef), []);
    assert.strictEqual(stateSelectors.monActivePosition(state, squirtleRef), 0);

    // 5. Convenience helpers
    assert.strictEqual(stateSelectors.monIsFainted(state, squirtleRef), false);
    assert.strictEqual(stateSelectors.monIsActive(state, squirtleRef), true);
    assert.deepStrictEqual(stateSelectors.activeMonByPosition(state, 0, 0), squirtleRef);
    assert.strictEqual(stateSelectors.playerMons(state, "player-1").length, 1);
    assert.strictEqual(stateSelectors.sidePlayers(state, 0).length, 1);

    // 6. MonTypes delegation
    const types = stateSelectors.monTypes(state, squirtleRef, (species) => {
      if (species === "Squirtle") return ["Water"];
      return [];
    });
    assert.deepStrictEqual(types, ["Water"]);

    // 7. OrElse helper functions throw correct errors
    assert.throws(() => stateSelectors.sideOrElse(state, 999), /side not found/);
    assert.throws(() => stateSelectors.playerOrElse(state, "non-existent"), /player not found/);
    assert.throws(
      () =>
        stateSelectors.monOrElse(state, {
          player: "player-1",
          mon_index: 999,
          battle_appearance_index: 0,
        }),
      /mon not found/,
    );
    assert.throws(
      () =>
        stateSelectors.monBattleAppearanceOrElse(state, {
          player: "player-1",
          mon_index: 0,
          battle_appearance_index: 999,
        }),
      /mon battle appearance not found/,
    );
  });
});
