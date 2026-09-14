import { describe, expect, it } from "vitest";
import {
  cleanJsonData,
  extractItemSpecial,
  extractMoveEffects,
  formatCompactJson,
  linkifyDelegates,
  resolveDelegateTarget,
} from "./fxlangFormatter";
import type { ItemData, MoveData } from "battler-types";

describe("fxlangFormatter", () => {
  describe("cleanJsonData", () => {
    it("strips null, undefined, false, and 0 boosts", () => {
      const input = {
        name: "Test",
        isNull: null,
        isFalse: false,
        isTrue: true,
        boosts: {
          atk: 1,
          def: 0,
          spe: 0,
        },
        emptyObj: {
          innerNull: null,
        },
      };

      expect(cleanJsonData(input)).toEqual({
        name: "Test",
        isTrue: true,
        boosts: {
          atk: 1,
        },
      });
    });

    it("strips 0 boosts when using singular boost field", () => {
      const input = {
        boost: {
          atk: 1,
          def: 0,
        },
      };

      expect(cleanJsonData(input)).toEqual({
        boost: {
          atk: 1,
        },
      });
    });
  });

  describe("formatCompactJson", () => {
    it("collapses single-element arrays with their brackets", () => {
      const input = {
        callbacks: {
          on_update: ["if $mon.status == par:", ["cure_status: $mon"]],
        },
      };

      const formatted = formatCompactJson(input);
      // ["cure_status: $mon"] should be on one line with brackets
      expect(formatted).toContain('["cure_status: $mon"]');
      // The parent array has 2 statements, so it should be on multiple lines
      expect(formatted).toContain('[\n      "if $mon.status == par:",\n      ["cure_status: $mon"]\n    ]');
    });

    it("does not merge multi-statement arrays onto a single line", () => {
      const input = {
        on_after_move_secondary_effects_damage: [
          "require $damage != 0 else return",
          "activate_ability: $original_hp",
        ],
      };

      const formatted = formatCompactJson(input);
      expect(formatted).toContain(
        '[\n    "require $damage != 0 else return",\n    "activate_ability: $original_hp"\n  ]',
      );
    });

    it("handles empty arrays and primitive single-property objects", () => {
      expect(formatCompactJson([])).toBe("[]");
      expect(formatCompactJson({})).toBe("{}");
      expect(formatCompactJson({ status: "par" })).toBe('{ "status": "par" }');
    });
  });

  describe("linkifyDelegates", () => {
    it("converts delegate string tokens into interactive span elements", () => {
      const html = '<span style="color:#CE9178">"condition:gemitembase"</span>';
      const linked = linkifyDelegates(html);
      expect(linked).toContain('class="fxlang-delegate-link"');
      expect(linked).toContain('data-delegate-prefix="condition"');
      expect(linked).toContain('data-delegate-name="gemitembase"');
      expect(linked).toContain('title="View definition of condition:gemitembase"');
    });

    it("generically supports any prefix", () => {
      const html = '"ability:emergencyexit" "item:choiceband" "species:ogerpon" "move:tackle" "abilitycondition:moldbreaker" "custom:myid"';
      const linked = linkifyDelegates(html);
      expect(linked).toContain('data-delegate-prefix="ability" data-delegate-name="emergencyexit"');
      expect(linked).toContain('data-delegate-prefix="item" data-delegate-name="choiceband"');
      expect(linked).toContain('data-delegate-prefix="species" data-delegate-name="ogerpon"');
      expect(linked).toContain('data-delegate-prefix="move" data-delegate-name="tackle"');
      expect(linked).toContain('data-delegate-prefix="abilitycondition" data-delegate-name="moldbreaker"');
      expect(linked).toContain('data-delegate-prefix="custom" data-delegate-name="myid"');
    });

    it("linkifies HitEffect condition fields as condition resource delegates", () => {
      const fields = [
        ['"volatile_status": "flinch"', "flinch"],
        ['"side_condition": "stealthrock"', "stealthrock"],
        ['"slot_condition": "wish"', "wish"],
        ['"weather": "raindance"', "raindance"],
        ['"pseudo_weather": "trickroom"', "trickroom"],
        ['"terrain": "electricterrain"', "electricterrain"],
        ['"status": "par"', "par"],
      ];

      for (const [codeSnippet, expectedName] of fields) {
        const linked = linkifyDelegates(codeSnippet);
        expect(linked).toContain(`data-delegate-prefix="condition"`);
        expect(linked).toContain(`data-delegate-name="${expectedName}"`);
        expect(linked).toContain(`title="View definition of condition:${expectedName}"`);
        expect(linked).toContain(`>${expectedName}</span>"`);
      }
    });

    it("linkifies HitEffect condition fields inside Shiki-highlighted HTML spans", () => {
      const shikiHtml =
        '<span class="line"><span style="color:#9CDCFE">  "volatile_status"</span><span style="color:#D4D4D4">: </span><span style="color:#CE9178">"flinch"</span></span>';
      const linked = linkifyDelegates(shikiHtml);
      expect(linked).toContain('class="fxlang-delegate-link"');
      expect(linked).toContain('data-delegate-prefix="condition"');
      expect(linked).toContain('data-delegate-name="flinch"');
      expect(linked).toContain('title="View definition of condition:flinch"');
      expect(linked).toContain('>flinch</span>');
    });

    it("does not double-wrap already-prefixed delegate strings in condition fields", () => {
      const snippet = '{ "volatile_status": "condition:flinch" }';
      const linked = linkifyDelegates(snippet);
      const occurrences = (linked.match(/class="fxlang-delegate-link"/g) || []).length;
      expect(occurrences).toBe(1);
      expect(linked).toContain('data-delegate-name="flinch"');
    });
  });

  describe("resolveDelegateTarget", () => {
    it("resolves prefixes to appropriate ResourceType", () => {
      expect(resolveDelegateTarget("ability", "levitate")).toEqual({ type: "ability", name: "levitate" });
      expect(resolveDelegateTarget("abilitycondition", "moldbreaker")).toEqual({ type: "ability", name: "moldbreaker" });
      expect(resolveDelegateTarget("move", "tackle")).toEqual({ type: "move", name: "tackle" });
      expect(resolveDelegateTarget("movecondition", "dive")).toEqual({ type: "move", name: "dive" });
      expect(resolveDelegateTarget("item", "choiceband")).toEqual({ type: "item", name: "choiceband" });
      expect(resolveDelegateTarget("itemcondition", "foo")).toEqual({ type: "item", name: "foo" });
      expect(resolveDelegateTarget("species", "ogerpon")).toEqual({ type: "species", name: "ogerpon" });
      expect(resolveDelegateTarget("condition", "gemitembase")).toEqual({ type: "condition", name: "gemitembase" });
      expect(resolveDelegateTarget("clause", "sleepclause")).toEqual({ type: "condition", name: "sleepclause" });
    });
  });

  describe("extractMoveEffects", () => {
    it("extracts move mechanics in alphabetical order followed by hit/user/secondary effect pipeline", () => {
      const mockMove = {
        name: "Test Move",
        damage: 40,
        ohko_type: "Normal",
        user_switch: "Normal",
        self_destruct: "Normal",
        recoil: { base: "DamageDealt", recoil: [1, 4] },
        drain_percent: [1, 2],
        force_stab: true,
        override_offensive_mon: "Target",
        override_offensive_stat: "Defense",
        override_defensive_mon: "User",
        override_defensive_stat: "SpecialDefense",
        crit_ratio: 2,
        ignore_accuracy: true,
        ignore_defensive: true,
        ignore_evasion: true,
        ignore_offensive: true,
        multiaccuracy: true,
        multihit: [2, 5],
        will_crit: true,
        advanced_targeting: { no_random_target: true, tracks_target: false, smart_target: false },
        hit_effect: { status: "par" },
        user_effect: { boost: { atk: 1 } },
        user_effect_chance: [1, 10],
        secondary_effects: [{ chance: [3, 10], effect: { status: "brn" } }],
      } as unknown as MoveData;

      const extracted = extractMoveEffects(mockMove);
      expect(extracted).toBeDefined();
      const keys = Object.keys(extracted!);

      expect(keys).toEqual([
        "advanced_targeting",
        "crit_ratio",
        "damage",
        "drain_percent",
        "force_stab",
        "ignore_accuracy",
        "ignore_defensive",
        "ignore_evasion",
        "ignore_offensive",
        "multiaccuracy",
        "multihit",
        "ohko_type",
        "override_defensive_mon",
        "override_defensive_stat",
        "override_offensive_mon",
        "override_offensive_stat",
        "recoil",
        "self_destruct",
        "user_switch",
        "will_crit",
        "hit_effect",
        "user_effect",
        "user_effect_chance",
        "secondary_effects",
      ]);
    });

    it("omits empty or false values cleanly", () => {
      const mockVanillaMove = {
        name: "Tackle",
        damage: null,
        ohko_type: null,
        user_switch: null,
        self_destruct: null,
        recoil: null,
        drain_percent: null,
        force_stab: false,
        override_offensive_mon: null,
        override_offensive_stat: null,
        override_defensive_mon: null,
        override_defensive_stat: null,
        crit_ratio: 1,
        ignore_accuracy: false,
        ignore_defensive: false,
        ignore_evasion: false,
        ignore_offensive: false,
        multiaccuracy: false,
        multihit: null,
        will_crit: false,
        advanced_targeting: { no_random_target: false, tracks_target: false, smart_target: false },
        hit_effect: null,
        user_effect: null,
        user_effect_chance: null,
        secondary_effects: [],
      } as unknown as MoveData;

      expect(extractMoveEffects(mockVanillaMove)).toBeUndefined();
    });
  });

  describe("extractItemSpecial", () => {
    it("extracts and wraps special item data", () => {
      const mockItem = {
        name: "Test Berry",
        special_data: {
          berry: {
            natural_gift: { base_power: 80, primary_type: "Fire" },
          },
        },
      } as unknown as ItemData;

      expect(extractItemSpecial(mockItem)).toEqual({
        special_data: {
          berry: {
            natural_gift: { base_power: 80, primary_type: "Fire" },
          },
        },
      });
    });

    it("returns undefined for items without special data", () => {
      const mockItem = {
        name: "Leftovers",
        special_data: {},
      } as unknown as ItemData;

      expect(extractItemSpecial(mockItem)).toBeUndefined();
    });

    it("extracts forme changes and player usage mechanics", () => {
      const mockPlate = {
        name: "Flame Plate",
        force_forme: "Arceus-Fire",
      } as unknown as ItemData;

      expect(extractItemSpecial(mockPlate)).toEqual({
        force_forme: "Arceus-Fire",
      });

      const mockPotion = {
        name: "Potion",
        target: "Party",
        input: "Move",
      } as unknown as ItemData;

      expect(extractItemSpecial(mockPotion)).toEqual({
        target: "Party",
        input: "Move",
      });
    });
  });
});
