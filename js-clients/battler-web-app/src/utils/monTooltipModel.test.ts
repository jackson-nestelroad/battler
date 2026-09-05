import { describe, expect, it } from "vitest";
import type { MonBattleData } from "battler-types";
import {
  computeExpMetrics,
  formatActiveBoosts,
  formatWeightKg,
  monBattleDataToTooltip,
  NATURE_MODIFIERS,
  publicMonStateToTooltip,
} from "./monTooltipModel";

describe("monTooltipModel", () => {
  describe("NATURE_MODIFIERS", () => {
    it("correctly identifies Adamant nature (+Atk, -SpA)", () => {
      expect(NATURE_MODIFIERS.Adamant).toEqual({ plus: "Atk", minus: "SpA" });
    });

    it("correctly identifies Jolly nature (+Spe, -SpA)", () => {
      expect(NATURE_MODIFIERS.Jolly).toEqual({ plus: "Spe", minus: "SpA" });
    });

    it("correctly identifies Timid nature (+Spe, -Atk)", () => {
      expect(NATURE_MODIFIERS.Timid).toEqual({ plus: "Spe", minus: "Atk" });
    });
  });

  describe("formatActiveBoosts", () => {
    it("returns empty array for empty or zero boosts", () => {
      expect(formatActiveBoosts(null)).toEqual([]);
      expect(
        formatActiveBoosts({
          atk: 0,
          def: 0,
          spa: 0,
          spd: 0,
          spe: 0,
          acc: 0,
          eva: 0,
        }),
      ).toEqual([]);
    });

    it("formats positive and negative boosts cleanly", () => {
      const formatted = formatActiveBoosts({
        atk: 2,
        def: 0,
        spa: 0,
        spd: 0,
        spe: -1,
        acc: 1,
        eva: 0,
      });

      expect(formatted).toEqual([
        { stat: "Atk", stage: 2, label: "+2 Atk" },
        { stat: "Spe", stage: -1, label: "-1 Spe" },
        { stat: "Acc", stage: 1, label: "+1 Acc" },
      ]);
    });
  });

  describe("monBattleDataToTooltip", () => {
    const mockMonBattleData: MonBattleData = {
      species: "Pikachu",
      hp: 120,
      max_hp: 150,
      health: "120/150",
      types: ["Electric"],
      active: true,
      player_team_position: 0,
      player_effective_team_position: 0,
      player_active_position: 0,
      side_position: 0,
      stats: {
        HP: 150,
        Atk: 105,
        Def: 75,
        SpAtk: 95,
        SpDef: 85,
        Spe: 150,
      },
      boosts: {
        atk: 1,
        def: 0,
        spa: 0,
        spd: 0,
        spe: 2,
        acc: 0,
        eva: 0,
      },
      moves: [
        {
          id: "thunderbolt",
          name: "Thunderbolt",
          pp: 14,
          max_pp: 15,
          type: "Electric",
          disabled: false,
          target: "Normal",
        },
        {
          id: "voltswitch",
          name: "Volt Switch",
          pp: 20,
          max_pp: 20,
          type: "Electric",
          disabled: false,
          target: "Normal",
        },
      ],
      ability: "Static",
      item: "Light Ball",
      status: "par",
      weight: 60,
      summary: {
        name: "Sparky",
        species: "Pikachu",
        level: 50,
        gender: "M",
        nature: "Jolly",
        shiny: true,
        ball: "cherishball",
        hp: 120,
        friendship: 255,
        experience: 50000,
        level_experience: 40000,
        next_level_experience: 60000,
        tera_type: "Electric",
        weight: 60,
        stats: { hp: 150, atk: 105, def: 75, spa: 95, spd: 85, spe: 150 },
        evs: { hp: 4, atk: 252, def: 0, spa: 0, spd: 0, spe: 252 },
        ivs: { hp: 31, atk: 31, def: 31, spa: 31, spd: 31, spe: 31 },
        moves: [
          { name: "Thunderbolt", pp: 14, max_pp: 15, typ: "Electric" },
          { name: "Volt Switch", pp: 20, max_pp: 20, typ: "Electric" },
        ],
        ability: "Static",
        item: "Light Ball",
        status: "par",
        hidden_power_type: "Ice",
      },
    };

    it("maps private MonBattleData into a complete MonTooltipViewModel", () => {
      const vm = monBattleDataToTooltip(mockMonBattleData);

      expect(vm.species).toBe("Pikachu");
      expect(vm.name).toBe("Sparky");
      expect(vm.level).toBe(50);
      expect(vm.gender).toBe("M");
      expect(vm.shiny).toBe(true);
      expect(vm.types).toEqual(["Electric"]);
      expect(vm.teraType).toBe("Electric");
      expect(vm.isTerastallized).toBe(false);
      expect(vm.ball).toBe("cherishball");
      expect(vm.hp).toBe(120);
      expect(vm.maxHp).toBe(150);
      expect(vm.hpPercentage).toBe(80);
      expect(vm.status).toBe("par");
      expect(vm.isFainted).toBe(false);
      expect(vm.ownerLabel).toBe("Your Mon");

      expect(vm.ability).toBe("Static");
      expect(vm.item).toBe("Light Ball");
      expect(vm.nature).toBe("Jolly");
      expect(vm.natureModifiers).toEqual({ plus: "Spe", minus: "SpA" });

      expect(vm.boosts).toEqual([
        { stat: "Atk", stage: 1, label: "+1 Atk" },
        { stat: "Spe", stage: 2, label: "+2 Spe" },
      ]);

      expect(vm.moves).toHaveLength(2);
      expect(vm.moves[0]).toEqual({
        name: "Thunderbolt",
        type: "Electric",
        pp: 14,
        maxPp: 15,
        disabled: false,
        revealed: true,
      });

      expect(vm.stats).toBeDefined();
      expect(vm.stats).toHaveLength(6);

      const speStat = vm.stats?.find((s) => s.stat === "Spe");
      expect(speStat).toBeDefined();
      expect(speStat?.value).toBe(150);
      expect(speStat?.ev).toBe(252);
      expect(speStat?.iv).toBe(31);
      expect(speStat?.boost).toBe(2);
      expect(speStat?.isPlus).toBe(true);
      expect(speStat?.isMinus).toBe(false);

      const spaStat = vm.stats?.find((s) => s.stat === "SpA");
      expect(spaStat?.isPlus).toBe(false);
      expect(spaStat?.isMinus).toBe(true);

      expect(vm.hiddenPowerType).toBe("Ice");
      expect(vm.friendship).toBe(255);

      // Verify baseSummary
      expect(vm.baseSummary).toBeDefined();
      expect(vm.baseSummary?.species).toBe("Pikachu");
      expect(vm.baseSummary?.name).toBe("Sparky");
      expect(vm.baseSummary?.ability).toBe("Static");
      expect(vm.baseSummary?.item).toBe("Light Ball");
      expect(vm.baseSummary?.ball).toBe("cherishball");
      expect(vm.baseSummary?.ownerLabel).toBe("Your Mon");
      expect(vm.baseSummary?.friendship).toBe(255);
      expect(vm.baseSummary?.hiddenPowerType).toBe("Ice");
      expect(vm.baseSummary?.moves).toHaveLength(2);
      expect(vm.baseSummary?.moves[0].name).toBe("Thunderbolt");
      expect(vm.baseSummary?.moves[0].pp).toBe(14);
      expect(vm.baseSummary?.moves[0].maxPp).toBe(15);
      expect(vm.baseSummary?.moves[0].type).toBe("Electric");
      expect(vm.baseSummary?.stats?.[0].stat).toBe("HP");
      expect(vm.baseSummary?.stats?.[0].boost).toBe(0);
      expect(vm.baseSummary?.hp).toBe(120);
      expect(vm.baseSummary?.maxHp).toBe(150);
      expect(vm.baseSummary?.hpPercentage).toBe(80);
      expect(vm.baseSummary?.status).toBe("par");
      expect(vm.baseSummary?.isFainted).toBe(false);
    });

    it("detects transformed Mons and ability/item diffs cleanly", () => {
      const transformedDitto: any = {
        ...mockMonBattleData,
        species: "Zamazenta",
        ability: "Dauntless Shield",
        item: null, // item was consumed/lost
        moves: [
          {
            name: "Close Combat",
            type: "Fighting",
            pp: 5,
            max_pp: 5,
            disabled: true, // disabled by Taunt / Torment / Imprison
          },
        ],
        summary: {
          ...mockMonBattleData.summary,
          species: "Ditto",
          ability: "Imposter",
          item: "Focus Sash",
          moves: [{ name: "Transform", pp: 16, max_pp: 16, typ: "Normal" }],
        },
      };

      const vm = monBattleDataToTooltip(transformedDitto);

      expect(vm.species).toBe("Zamazenta");
      expect(vm.isTransformed).toBe(true);
      expect(vm.originalSpecies).toBe("Ditto");

      expect(vm.ability).toBe("Dauntless Shield");
      expect(vm.item).toBe("None (was Focus Sash)");
      expect(vm.moves[0].disabled).toBe(true);

      // Verify baseSummary reflects original Ditto
      expect(vm.baseSummary).toBeDefined();
      expect(vm.baseSummary?.species).toBe("Ditto");
      expect(vm.baseSummary?.ability).toBe("Imposter");
      expect(vm.baseSummary?.item).toBe("Focus Sash");
      expect(vm.baseSummary?.moves[0].name).toBe("Transform");
      expect(vm.baseSummary?.moves[0].maxPp).toBe(16);
      expect(vm.baseSummary?.moves[0].type).toBe("Normal");
    });

    it("identifies fainted status correctly for varied status inputs", () => {
      const faintedMon1 = {
        ...mockMonBattleData,
        hp: 0,
        status: null,
      };
      expect(monBattleDataToTooltip(faintedMon1).isFainted).toBe(true);

      const faintedMon2 = {
        ...mockMonBattleData,
        hp: 100,
        status: "fainted",
      };
      const vm2 = monBattleDataToTooltip(faintedMon2);
      expect(vm2.status).toBe("fnt");
      expect(vm2.isFainted).toBe(true);

      const faintedMon3 = {
        ...mockMonBattleData,
        hp: 100,
        status: "FNT",
      };
      const vm3 = monBattleDataToTooltip(faintedMon3);
      expect(vm3.status).toBe("fnt");
      expect(vm3.isFainted).toBe(true);
    });

    it("preserves base item on Summary tab and shows was-annotation on Battle tab when item is consumed", () => {
      const monWithConsumedGem: any = {
        ...mockMonBattleData,
        species: "Lucario",
        item: null, // item was consumed in battle
        summary: {
          ...mockMonBattleData.summary,
          species: "Lucario",
          item: "Ghost Gem",
        },
      };

      const vm = monBattleDataToTooltip(monWithConsumedGem);
      expect(vm.item).toBe("None (was Ghost Gem)");
      expect(vm.baseSummary?.item).toBe("Ghost Gem");
    });

    it("ensures baseSummary displays undynamaxed HP correctly without exceeding max HP", () => {
      const dynamaxedCloyster: any = {
        ...mockMonBattleData,
        species: "Cloyster",
        hp: 250,
        max_hp: 250,
        health: "250/250",
        summary: {
          ...mockMonBattleData.summary,
          species: "Cloyster",
          hp: 125, // undynamaxed HP
          stats: { hp: 125, atk: 115, def: 200, spa: 105, spd: 65, spe: 90 },
        },
      };

      const vm = monBattleDataToTooltip(dynamaxedCloyster);
      // Battle tab reflects live Dynamaxed stats
      expect(vm.hp).toBe(250);
      expect(vm.maxHp).toBe(250);

      // Summary tab reflects permanent base stats (undynamaxed)
      expect(vm.baseSummary?.hp).toBe(125);
      expect(vm.baseSummary?.maxHp).toBe(125);
      expect(vm.baseSummary?.hpPercentage).toBe(100);
    });

    it("preserves base move max PP and type on summary even if move was replaced in battle (e.g. Mimic)", () => {
      const monWithMimic: any = {
        ...mockMonBattleData,
        moves: [
          {
            id: "flamethrower",
            name: "Flamethrower",
            pp: 5,
            max_pp: 5,
            type: "Fire",
            disabled: false,
          },
        ],
        summary: {
          ...mockMonBattleData.summary,
          moves: [
            {
              name: "Mimic",
              pp: 9,
              max_pp: 10,
              typ: "Normal",
            },
          ],
        },
      };

      const vm = monBattleDataToTooltip(monWithMimic);
      expect(vm.moves[0].name).toBe("Flamethrower");
      expect(vm.baseSummary?.moves).toHaveLength(1);
      expect(vm.baseSummary?.moves[0]).toEqual({
        name: "Mimic",
        type: "Normal",
        pp: 9,
        maxPp: 10,
        revealed: true,
      });
    });

    it("resolves active conditions and transformation from battleState for your Mon", () => {
      const mockState: any = {
        field: {
          sides: [
            {
              players: {
                "player-1": {
                  id: "player-1",
                  mons: [
                    {
                      physical_appearance: {
                        name: "Sparky",
                        species: "Pikachu",
                      },
                      battle_appearances: [{}],
                      volatile_data: {
                        conditions: {
                          Substitute: {},
                          Taunt: {},
                        },
                        transformed: [
                          { species: "Charizard" },
                          { player: "player-2", mon_index: 0 },
                        ],
                      },
                    },
                  ],
                },
              },
              active: [
                {
                  player: "player-1",
                  mon_index: 0,
                  battle_appearance_index: 0,
                },
              ],
            },
          ],
        },
      };

      const monWithExpAndWeight: any = {
        ...mockMonBattleData,
        weight: 60, // 6.0 kg in battle
        summary: {
          ...mockMonBattleData.summary,
          experience: 128000,
          level_experience: 125000,
          next_level_experience: 132651,
          weight: 60,
          tera_type: "Electric",
        },
      };

      const vm = monBattleDataToTooltip(monWithExpAndWeight, mockState);

      expect(vm.conditions).toEqual(["Substitute", "Taunt"]);
      expect(vm.isTransformed).toBe(true);
      expect(vm.originalSpecies).toBe("Pikachu");
      expect(vm.isDynamaxed).toBe(false);
      expect(vm.weightKg).toBe(6.0);
      expect(vm.experience).toBe(128000);
      expect(vm.expToNextLevel).toBe(4651);
      expect(vm.expProgressPercent).toBe(39);
      expect(vm.baseSummary?.teraType).toBe("Electric");
      expect(vm.baseSummary?.weightKg).toBe(6.0);
      expect(vm.baseSummary?.isDynamaxed).toBe(false);
    });

    it("identifies dynamaxed Mon from battleState", () => {
      const dynamaxState: any = {
        field: {
          sides: [
            {
              players: {
                "player-1": {
                  id: "player-1",
                  mons: [
                    {
                      physical_appearance: {
                        name: "Sparky",
                        species: "Pikachu",
                      },
                      battle_appearances: [{}],
                      volatile_data: {
                        conditions: {
                          Dynamax: {},
                        },
                      },
                    },
                  ],
                },
              },
              active: [
                {
                  player: "player-1",
                  mon_index: 0,
                  battle_appearance_index: 0,
                },
              ],
            },
          ],
        },
      };

      const vm = monBattleDataToTooltip(mockMonBattleData, dynamaxState);
      expect(vm.isDynamaxed).toBe(true);
      expect(vm.baseSummary?.isDynamaxed).toBe(false);
    });
  });

  describe("EXP and Weight formatting", () => {
    it("formats weight in kg correctly", () => {
      expect(formatWeightKg(null)).toBeNull();
      expect(formatWeightKg(undefined)).toBeNull();
      expect(formatWeightKg(90)).toBe(9.0);
      expect(formatWeightKg(5500)).toBe(550.0);
    });

    it("computes EXP progress and to-next metrics correctly", () => {
      // Mid-level
      const mid = computeExpMetrics(128000, 125000, 132651);
      expect(mid.experience).toBe(128000);
      expect(mid.expToNextLevel).toBe(4651);
      expect(mid.expProgressPercent).toBe(39);

      // Max level (100)
      const maxLvl = computeExpMetrics(1250000, 1250000, null);
      expect(maxLvl.expToNextLevel).toBe(0);
      expect(maxLvl.expProgressPercent).toBe(100);
    });
  });

  describe("publicMonStateToTooltip", () => {
    it("returns null when battleState is null", () => {
      expect(
        publicMonStateToTooltip(null, {
          Active: { side: 0, position: 0, player: "alice", name: "Pikachu" },
        }),
      ).toBeNull();
    });

    it("returns graceful fallback when mon is not found in state", () => {
      const emptyState = {
        field: { sides: [] },
      } as any;

      const vm = publicMonStateToTooltip(emptyState, {
        Active: { side: 0, position: 0, player: "bob", name: "Bulbasaur" },
      });

      expect(vm).toBeDefined();
      expect(vm?.species).toBe("Bulbasaur");
      expect(vm?.ownerLabel).toBe("Player: bob");
      expect(vm?.stats).toBeNull(); // Never leaks private stats
    });

    it("correctly resolves a previously active mon by name even when another mon has swapped into that slot", () => {
      const mockState = {
        field: {
          sides: [
            {
              players: {
                "player-1": {
                  id: "player-1",
                  name: "Player 1",
                  mons: [],
                },
              },
              active: [],
            },
            {
              players: {
                "ai-random-1": {
                  id: "ai-random-1",
                  name: "AI Random",
                  mons: [
                    {
                      physical_appearance: {
                        name: "Regidrago",
                        species: "Regidrago",
                        gender: "U",
                        shiny: false,
                      },
                      battle_appearances: [
                        {
                          inactive: {
                            level: { known: 50n },
                            health: { known: [80n, 100n] },
                            status: { known: "" },
                            ability: { known: "Dragon's Maw" },
                            item: { known: "" },
                            terastallization: { known: "" },
                            moves: { known: ["Trick-or-Treat"], possibly_includes: [] },
                            move_history: ["Trick-or-Treat"],
                          },
                        },
                      ],
                      fainted: false,
                      volatile_data: {
                        moves: [],
                        ability: null,
                        conditions: {},
                        types: [],
                        added_type: null,
                        stat_boosts: {},
                        forme_change: null,
                        transformed: null,
                      },
                    },
                    {
                      physical_appearance: {
                        name: "Sceptile",
                        species: "Sceptile",
                        gender: "F",
                        shiny: false,
                      },
                      battle_appearances: [
                        {
                          active: {
                            primary_battle_appearance: {
                              level: { known: 50n },
                              health: { known: [100n, 100n] },
                              status: { known: "" },
                              ability: { known: "Overgrow" },
                              item: { known: "" },
                              terastallization: { known: "" },
                              moves: { known: ["Leaf Blade"], possibly_includes: [] },
                              move_history: ["Leaf Blade"],
                            },
                          },
                        },
                      ],
                      fainted: false,
                      volatile_data: {
                        moves: [],
                        ability: null,
                        conditions: {},
                        types: [],
                        added_type: null,
                        stat_boosts: {},
                        forme_change: null,
                        transformed: null,
                      },
                    },
                  ],
                },
              },
              // Sceptile (index 1) is currently active in position 0!
              active: [
                {
                  player: "ai-random-1",
                  mon_index: 1,
                  battle_appearance_index: 0,
                },
              ],
            },
          ],
        },
      } as any;

      // Historical log token from when Regidrago was in position 0
      const regidragoUiMon: any = {
        Active: {
          side: 1,
          position: 0,
          player: "ai-random-1",
          name: "Regidrago",
        },
      };

      const vm = publicMonStateToTooltip(mockState, regidragoUiMon);

      expect(vm).toBeDefined();
      expect(vm?.species).toBe("Regidrago");
      expect(vm?.hp).toBe(80);
      expect(vm?.maxHp).toBe(100);
      expect(vm?.ability).toBe("Dragon's Maw");
      expect(vm?.moves[0].name).toBe("Trick-or-Treat");
      expect(vm?.item).toBe("None");
      expect(vm?.ownerLabel).toBe("Player: ai-random-1");
    });

    it("detects transformed opponent Mon from battleState", () => {
      const mockState: any = {
        field: {
          sides: [
            {
              players: {
                "ai-1": {
                  id: "ai-1",
                  mons: [
                    {
                      physical_appearance: {
                        name: "Ditto",
                        species: "Ditto",
                      },
                      battle_appearances: [
                        {
                          active: {
                            primary_battle_appearance: {
                              level: { known: 50n },
                              health: { known: [100n, 100n] },
                              status: { known: "" },
                              ability: { known: "Blaze" },
                              item: { known: "" },
                              terastallization: { known: "" },
                              moves: { known: ["Flamethrower"], possibly_includes: [] },
                              move_history: ["Flamethrower"],
                            },
                          },
                        },
                      ],
                      volatile_data: {
                        moves: [],
                        ability: null,
                        conditions: {},
                        types: [],
                        added_type: null,
                        stat_boosts: {},
                        forme_change: null,
                        transformed: [
                          { species: "Charizard" },
                          { player: "player-1", mon_index: 0 },
                        ],
                      },
                    },
                  ],
                },
              },
              active: [
                {
                  player: "ai-1",
                  mon_index: 0,
                  battle_appearance_index: 0,
                },
              ],
            },
          ],
        },
      };

      const uiMon: any = {
        Active: {
          side: 0,
          position: 0,
          player: "ai-1",
          name: "Charizard",
        },
      };

      const vm = publicMonStateToTooltip(mockState, uiMon);
      expect(vm).toBeDefined();
      expect(vm?.isTransformed).toBe(true);
      expect(vm?.originalSpecies).toBe("Ditto");
    });

    it("distinguishes unrevealed traits from known values", () => {
      const unrevealedState: any = {
        field: {
          sides: [
            {
              players: {
                "opponent-1": {
                  id: "opponent-1",
                  mons: [
                    {
                      physical_appearance: {
                        name: "Gengar",
                        species: "Gengar",
                      },
                      battle_appearances: [
                        {
                          active: {
                            primary_battle_appearance: {
                              level: { known: 50n },
                              health: { known: [100n, 100n] },
                              status: { known: "" },
                              ability: { possibly_one_of: ["Cursed Body"] },
                              item: { possibly_one_of: ["Life Orb", "Black Sludge"] },
                              terastallization: { known: "" },
                              moves: { known: [], possibly_includes: ["Shadow Ball"] },
                              move_history: [],
                            },
                          },
                        },
                      ],
                      volatile_data: {
                        moves: [],
                        ability: null,
                        conditions: {},
                        types: [],
                        stat_boosts: {},
                      },
                    },
                  ],
                },
              },
              active: [
                {
                  player: "opponent-1",
                  mon_index: 0,
                  battle_appearance_index: 0,
                },
              ],
            },
          ],
        },
      };

      const vm = publicMonStateToTooltip(unrevealedState, {
        Active: { side: 0, position: 0, player: "opponent-1", name: "Gengar" },
      });

      expect(vm).toBeDefined();
      expect(vm?.ability).toBeNull();
      expect(vm?.item).toBeNull();
      expect(vm?.moves).toEqual([]);
    });

    it("reveals opponent item when known in battle state", () => {
      const revealedState: any = {
        field: {
          sides: [
            {
              players: {
                "opponent-1": {
                  id: "opponent-1",
                  mons: [
                    {
                      physical_appearance: {
                        name: "Gengar",
                        species: "Gengar",
                      },
                      battle_appearances: [
                        {
                          active: {
                            primary_battle_appearance: {
                              level: { known: 50n },
                              health: { known: [100n, 100n] },
                              status: { known: "" },
                              ability: { known: "Cursed Body" },
                              item: { known: "Life Orb" },
                              terastallization: { known: "" },
                              moves: { known: ["Shadow Ball"], possibly_includes: [] },
                              move_history: ["Shadow Ball"],
                            },
                          },
                        },
                      ],
                      volatile_data: {
                        moves: [],
                        ability: null,
                        conditions: {},
                        types: [],
                        stat_boosts: {},
                      },
                    },
                  ],
                },
              },
              active: [
                {
                  player: "opponent-1",
                  mon_index: 0,
                  battle_appearance_index: 0,
                },
              ],
            },
          ],
        },
      };

      const vm = publicMonStateToTooltip(revealedState, {
        Active: { side: 0, position: 0, player: "opponent-1", name: "Gengar" },
      });

      expect(vm).toBeDefined();
      expect(vm?.ability).toBe("Cursed Body");
      expect(vm?.item).toBe("Life Orb");
      expect(vm?.moves[0].name).toBe("Shadow Ball");
    });

    it("shows None (was ...) for opponent when item was consumed/lost and previous_item is known", () => {
      const consumedState: any = {
        field: {
          sides: [
            {
              players: {
                "opponent-1": {
                  id: "opponent-1",
                  mons: [
                    {
                      physical_appearance: {
                        name: "Gengar",
                        species: "Gengar",
                      },
                      battle_appearances: [
                        {
                          active: {
                            primary_battle_appearance: {
                              level: { known: 50n },
                              health: { known: [100n, 100n] },
                              status: { known: "" },
                              ability: { known: "Cursed Body" },
                              item: { known: "" },
                              previous_item: { known: "Focus Sash" },
                              terastallization: { known: "" },
                              moves: { known: ["Shadow Ball"], possibly_includes: [] },
                              move_history: ["Shadow Ball"],
                            },
                          },
                        },
                      ],
                      volatile_data: {
                        moves: [],
                        ability: null,
                        conditions: {},
                        types: [],
                        stat_boosts: {},
                      },
                    },
                  ],
                },
              },
              active: [
                {
                  player: "opponent-1",
                  mon_index: 0,
                  battle_appearance_index: 0,
                },
              ],
            },
          ],
        },
      };

      const vm = publicMonStateToTooltip(consumedState, {
        Active: { side: 0, position: 0, player: "opponent-1", name: "Gengar" },
      });

      expect(vm).toBeDefined();
      expect(vm?.item).toBe("None (was Focus Sash)");
    });

    it("displays None when previous_item is None or empty", () => {
      const emptyPrevState: any = {
        field: {
          sides: [
            {
              players: {
                "opponent-1": {
                  id: "opponent-1",
                  mons: [
                    {
                      physical_appearance: {
                        name: "Gengar",
                        species: "Gengar",
                      },
                      battle_appearances: [
                        {
                          active: {
                            primary_battle_appearance: {
                              level: { known: 50n },
                              health: { known: [100n, 100n] },
                              status: { known: "" },
                              ability: { known: "Cursed Body" },
                              item: { known: "" },
                              previous_item: { known: "None" },
                              terastallization: { known: "" },
                              moves: { known: ["Shadow Ball"], possibly_includes: [] },
                              move_history: ["Shadow Ball"],
                            },
                          },
                        },
                      ],
                      volatile_data: {
                        moves: [],
                        ability: null,
                        conditions: {},
                        types: [],
                        stat_boosts: {},
                      },
                    },
                  ],
                },
              },
              active: [
                {
                  player: "opponent-1",
                  mon_index: 0,
                  battle_appearance_index: 0,
                },
              ],
            },
          ],
        },
      };

      const vm = publicMonStateToTooltip(emptyPrevState, {
        Active: { side: 0, position: 0, player: "opponent-1", name: "Gengar" },
      });

      expect(vm).toBeDefined();
      expect(vm?.item).toBe("None");
    });
  });
});

