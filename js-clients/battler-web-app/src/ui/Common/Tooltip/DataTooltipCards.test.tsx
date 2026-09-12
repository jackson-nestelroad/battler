import { renderToStaticMarkup } from "react-dom/server";
import type { AbilityData, ConditionData, ItemData, MoveData, SpeciesData } from "battler-types";
import { describe, expect, it } from "vitest";
import AbilityTooltipCard from "./AbilityTooltipCard";
import ConditionTooltipCard from "./ConditionTooltipCard";
import ItemTooltipCard from "./ItemTooltipCard";
import MoveTooltipCard from "./MoveTooltipCard";
import SimpleDataTooltipCard from "./SimpleDataTooltipCard";
import SpeciesTooltipCard from "./SpeciesTooltipCard";
import TooltipFlagsSection from "./TooltipFlagsSection";

describe("Data Tooltip Cards", () => {
  describe("MoveTooltipCard", () => {
    const mockMove: MoveData = {
      name: "Thunderbolt",
      category: "Special",
      primary_type: "Electric",
      base_power: 90,
      accuracy: 100,
      pp: 15,
      priority: 0,
      target: "Normal",
      flags: ["Protect", "Mirror"],
      damage: null,
      no_pp_boosts: false,
      ohko_type: null,
      user_switch: null,
      self_destruct: null,
      recoil: null,
      drain_percent: null,
      force_stab: false,
      hit_effect: null,
      user_effect: null,
      user_effect_chance: null,
      secondary_effects: [
        {
          chance: "1/10",
          apply_once: false,
          target: {
            boosts: null,
            heal_percent: null,
            status: "par",
            volatile_status: null,
            side_condition: null,
            slot_condition: null,
            weather: null,
            pseudo_weather: null,
            terrain: null,
            force_switch: false,
          },
          user: null,
          source_effect: null,
          effect: null,
        },
      ],
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
      advanced_targeting: {
        no_random_target: false,
        tracks_target: false,
        smart_target: false,
      },
      z_move: null,
      max_move: null,
      effect: null,
      condition: null,
    };

    it("renders move properties correctly", () => {
      const html = renderToStaticMarkup(<MoveTooltipCard data={mockMove} />);
      expect(html).toContain("Thunderbolt");
      expect(html).toContain("Move");
      expect(html).toContain("Special");
      expect(html).toContain('data-category="special"');
      expect(html).toContain('src="/assets/categories/special.png"');
      expect(html).toContain("90");
      expect(html).toContain("100%");
      expect(html).toContain("15 (max 24)");
      expect(html).toContain("Normal");
      expect(html).toContain("Protect");
      expect(html).toContain("Effect");
      // Priority 0 is omitted
      expect(html).not.toContain("Priority:");
    });

    it("renders status move with dash for base power, exempt accuracy, and signed priority", () => {
      const mockStatusMove: MoveData = {
        ...mockMove,
        name: "Baby-Doll Eyes",
        category: "Status",
        primary_type: "Fairy",
        base_power: 0,
        accuracy: "exempt",
        pp: 30,
        priority: 1,
      };
      const html = renderToStaticMarkup(<MoveTooltipCard data={mockStatusMove} />);
      expect(html).toContain("Baby-Doll Eyes");
      expect(html).toContain("Status");
      expect(html).toContain('data-category="status"');
      expect(html).toContain('src="/assets/categories/status.png"');
      expect(html).toContain("—"); // base power and accuracy
      expect(html).toContain("30 (max 48)");
      expect(html).toContain("Priority:");
      expect(html).toContain("+1");
    });

    it("renders unboostable move PP without redundant max suffix", () => {
      const unboostableMove: MoveData = {
        ...mockMove,
        name: "Revival Blessing",
        pp: 1,
        no_pp_boosts: true,
      };
      const html = renderToStaticMarkup(<MoveTooltipCard data={unboostableMove} />);
      expect(html).toContain("Revival Blessing");
      expect(html).toContain(">1<");
      expect(html).not.toContain("1 (max 1)");
    });

    it("renders move description when provided", () => {
      const html = renderToStaticMarkup(
        <MoveTooltipCard
          data={mockMove}
          description={{
            description:
              "A strong electric attack that may also leave the target with paralysis.",
            source: "Scarlet / Violet",
          }}
        />,
      );
      expect(html).toContain(
        "A strong electric attack that may also leave the target with paralysis.",
      );
    });

    it("renders secondary effects with chance percentage", () => {
      const html = renderToStaticMarkup(<MoveTooltipCard data={mockMove} />);
      expect(html).toContain("Effects");
      expect(html).toContain("10% chance to paralyze the target.");
    });

    it("renders primary hit_effect without chance percentage", () => {
      const mockStatusMove: MoveData = {
        ...mockMove,
        name: "Thunder Wave",
        secondary_effects: [],
        hit_effect: {
          status: "par",
          boosts: null,
          heal_percent: null,
          volatile_status: null,
          side_condition: null,
          slot_condition: null,
          weather: null,
          pseudo_weather: null,
          terrain: null,
          force_switch: false,
        },
      };
      const html = renderToStaticMarkup(<MoveTooltipCard data={mockStatusMove} />);
      expect(html).toContain("Effects");
      expect(html).toContain("Paralyzes the target.");
      expect(html).not.toContain("chance");
    });

    it("renders healing effects concisely", () => {
      const mockHealMove: MoveData = {
        ...mockMove,
        name: "Recover",
        target: "User",
        secondary_effects: [],
        hit_effect: {
          status: null,
          boosts: null,
          heal_percent: "50%",
          volatile_status: null,
          side_condition: null,
          slot_condition: null,
          weather: null,
          pseudo_weather: null,
          terrain: null,
          force_switch: false,
        },
      };
      const html = renderToStaticMarkup(<MoveTooltipCard data={mockHealMove} />);
      expect(html).toContain("Restores 50% of the user&#x27;s HP.");
    });

    it("renders user stat drops without chance", () => {
      const mockUserDropMove: MoveData = {
        ...mockMove,
        name: "Close Combat",
        secondary_effects: [],
        user_effect: {
          status: null,
          boosts: {
            def: -1,
            spa: -1,
            atk: 0,
            spd: 0,
            spe: 0,
            acc: 0,
            eva: 0,
          },
          heal_percent: null,
          volatile_status: null,
          side_condition: null,
          slot_condition: null,
          weather: null,
          pseudo_weather: null,
          terrain: null,
          force_switch: false,
        },
      };
      const html = renderToStaticMarkup(<MoveTooltipCard data={mockUserDropMove} />);
      expect(html).toContain("Lowers the user&#x27;s Defense and Sp. Atk by 1 stage.");
      expect(html).not.toContain("chance");
    });

    it("omits effects section when move has no effects", () => {
      const noEffectMove: MoveData = {
        ...mockMove,
        secondary_effects: [],
        hit_effect: null,
        user_effect: null,
      };
      const html = renderToStaticMarkup(<MoveTooltipCard data={noEffectMove} />);
      expect(html).not.toContain("Effects");
    });
  });

  describe("AbilityTooltipCard", () => {
    const mockAbility: AbilityData = {
      name: "Levitate",
      flags: ["Breakable"],
      effect: null,
      condition: null,
    };

    it("renders ability name and flags", () => {
      const html = renderToStaticMarkup(<AbilityTooltipCard data={mockAbility} />);
      expect(html).toContain("Levitate");
      expect(html).toContain("Ability");
      expect(html).toContain("Breakable");
      expect(html).toContain("Effect");
    });

    it("renders ability description when provided", () => {
      const html = renderToStaticMarkup(
        <AbilityTooltipCard
          data={mockAbility}
          description={{
            description:
              "By floating in the air, the Pokémon receives full immunity to all Ground-type moves.",
            source: "Scarlet / Violet",
          }}
        />,
      );
      expect(html).toContain(
        "By floating in the air, the Pokémon receives full immunity to all Ground-type moves.",
      );
    });
  });

  describe("ItemTooltipCard", () => {
    const mockItem: ItemData = {
      name: "Leftovers",
      target: null,
      input: null,
      special_data: {
        fling: {
          power: 10,
          use_item: true,
          hit_effect: null,
        },
        natural_gift: null,
        mega_evolution: null,
        z_crystal: null,
        ultra_burst: null,
        judgment: null,
        techno_blast: null,
        multi_attack: null,
      },
      force_forme: null,
      flags: ["Battle"],
      effect: null,
      condition: null,
    };

    it("renders item name and flags, omitting target even when present", () => {
      const targetedItem: ItemData = {
        ...mockItem,
        name: "Potion",
        target: "Active",
      };
      const html = renderToStaticMarkup(<ItemTooltipCard data={targetedItem} />);
      expect(html).toContain("Potion");
      expect(html).toContain("Item");
      expect(html).toContain("Battle");
      expect(html).toContain("Effect");
      expect(html).not.toContain("Target:");
      expect(html).not.toContain("Active");
    });

    it("renders item description when provided", () => {
      const html = renderToStaticMarkup(
        <ItemTooltipCard
          data={mockItem}
          description={{
            description:
              "An item to be held by a Pokémon. The holder's HP is gradually restored during battle.",
            source: "Scarlet / Violet",
          }}
        />,
      );
      expect(html).toContain("The holder&#x27;s HP is gradually restored during battle.");
    });
  });

  describe("ConditionTooltipCard", () => {
    const mockCondition: ConditionData = {
      name: "Rain",
      condition_type: "Weather",
      no_copy: true,
      condition: null,
    };

    it("renders condition details correctly", () => {
      const html = renderToStaticMarkup(<ConditionTooltipCard data={mockCondition} />);
      expect(html).toContain("Rain");
      expect(html).toContain("Weather");
      expect(html).toContain("No copy");
      expect(html).toContain("Effect");
    });

    it("renders condition description when provided", () => {
      const html = renderToStaticMarkup(
        <ConditionTooltipCard
          data={mockCondition}
          description={{
            description:
              "Rain pours down for five turns, powering up Water-type moves and weakening Fire-type moves.",
            source: "Scarlet / Violet",
          }}
        />,
      );
      expect(html).toContain(
        "Rain pours down for five turns, powering up Water-type moves and weakening Fire-type moves.",
      );
    });
  });

  describe("SpeciesTooltipCard", () => {
    const mockSpecies: SpeciesData = {
      name: "Garchomp",
      base_species: "Garchomp",
      forme: null,
      class: "Mach Pokémon",
      color: "Blue",
      primary_type: "Dragon",
      secondary_type: "Ground",
      abilities: ["Sand Veil"],
      hidden_ability: "Rough Skin",
      gender_ratio: 127,
      catch_rate: 45,
      can_hatch: true,
      egg_groups: ["Monster", "Dragon"],
      hatch_time: 40,
      height: 19,
      weight: 950,
      base_exp_yield: 270,
      leveling_rate: "Slow",
      ev_yield: { hp: 0, atk: 3, def: 0, spa: 0, spd: 0, spe: 0 },
      base_friendship: 50,
      max_hp: null,
      base_stats: { hp: 108, atk: 130, def: 95, spa: 80, spd: 85, spe: 102 },
      prevo: "Gabite",
      evos: [],
      evolution_data: null,
      base_forme: null,
      formes: [],
      cosmetic_formes: [],
      battle_only_forme: false,
      required_moves: [],
      required_items: [],
      changes_from: null,
      gigantamax_move: null,
      events: {},
      learnset: {},
      effect: null,
      flags: ["SubLegendary"],
    };

    it("renders species details, class subtitle as Mon, dual types, base stats table, BST, unified abilities with HA, gender split bar, and egg groups", () => {
      const html = renderToStaticMarkup(<SpeciesTooltipCard data={mockSpecies} />);
      expect(html).toContain("Garchomp");
      expect(html).toContain("Mach Mon");
      expect(html).not.toContain("Mach Pokémon");
      expect(html).toContain("Dragon");
      expect(html).toContain("Ground");
      expect(html).toContain("Base Stats");
      expect(html).toContain("108");
      expect(html).toContain("130");
      expect(html).toContain("600"); // BST total
      expect(html).toContain("Sand Veil");
      expect(html).toContain("Rough Skin");
      expect(html).toContain("/");
      expect(html).toContain("(H)");
      expect(html).toMatch(/role="button"[^>]*><span>Sand Veil<\/span>/);
      expect(html).toMatch(/role="button"[^>]*><span>Rough Skin<\/span>/);
      expect(html).toContain("50%");
      expect(html).toContain("♂");
      expect(html).toContain("♀");
      expect(html).toContain("role=\"meter\"");
      expect(html).toContain("aria-valuemin=\"0\"");
      expect(html).toContain("aria-valuemax=\"100\"");
      expect(html).toContain("Monster, Dragon");
      expect(html).toContain("95.0 kg");
      expect(html).toContain("SubLegendary");
      expect(html).toContain("Effect");
    });

    it("formats class without Pokémon suffix into Descriptor Mon (e.g. Tricky Fox -> Tricky Fox Mon)", () => {
      const zorua: SpeciesData = {
        ...mockSpecies,
        name: "Zorua",
        class: "Tricky Fox",
      };
      const html = renderToStaticMarkup(<SpeciesTooltipCard data={zorua} />);
      expect(html).toContain("Tricky Fox Mon");
    });

    it("renders max_hp override in base stats when specified", () => {
      const shedinja: SpeciesData = {
        ...mockSpecies,
        name: "Shedinja",
        class: "Shed Pokémon",
        max_hp: 1,
        base_stats: { hp: 1, atk: 90, def: 45, spa: 30, spd: 30, spe: 40 },
      };
      const html = renderToStaticMarkup(<SpeciesTooltipCard data={shedinja} />);
      expect(html).toContain("Shed Mon");
      expect(html).toContain("1 (max 1)");
    });

    it("renders genderless, male-only, and female-only gender ratios correctly", () => {
      const genderlessHtml = renderToStaticMarkup(
        <SpeciesTooltipCard data={{ ...mockSpecies, gender_ratio: 255 }} />,
      );
      expect(genderlessHtml).toContain("Genderless");
      expect(genderlessHtml).not.toContain("role=\"meter\"");

      const maleOnlyHtml = renderToStaticMarkup(
        <SpeciesTooltipCard data={{ ...mockSpecies, gender_ratio: 0 }} />,
      );
      expect(maleOnlyHtml).toContain("100%");
      expect(maleOnlyHtml).toContain("♂");
      expect(maleOnlyHtml).not.toContain("♀");

      const femaleOnlyHtml = renderToStaticMarkup(
        <SpeciesTooltipCard data={{ ...mockSpecies, gender_ratio: 254 }} />,
      );
      expect(femaleOnlyHtml).toContain("100%");
      expect(femaleOnlyHtml).toContain("♀");
      expect(femaleOnlyHtml).not.toContain("♂");
    });

    it("deduplicates redundant abilities in species abilities array", () => {
      const dupeSpecies: SpeciesData = {
        ...mockSpecies,
        abilities: ["Levitate", "Levitate"],
        hidden_ability: null,
      };
      const html = renderToStaticMarkup(<SpeciesTooltipCard data={dupeSpecies} />);
      const matches = html.match(/Levitate/g);
      // "Levitate" appears once inside the single trigger
      expect(matches).not.toBeNull();
      expect(matches!.length).toBe(1);
    });

    it("renders Pokédex species description when provided", () => {
      const html = renderToStaticMarkup(
        <SpeciesTooltipCard
          data={mockSpecies}
          description={{
            description: "It flies through the sky at Mach speed, searching for prey.",
            source: "Scarlet / Violet",
          }}
        />,
      );
      expect(html).toContain("It flies through the sky at Mach speed, searching for prey.");
    });
  });

  describe("SimpleDataTooltipCard", () => {
    it("renders name, subtitle, and sorted flags", () => {
      const html = renderToStaticMarkup(
        <SimpleDataTooltipCard
          name="Intimidate"
          subtitle="Ability"
          flags={["Breakable"]}
        />,
      );
      expect(html).toContain("Intimidate");
      expect(html).toContain("Ability");
      expect(html).toContain("Breakable");
    });

    it("renders description when provided", () => {
      const html = renderToStaticMarkup(
        <SimpleDataTooltipCard
          name="Intimidate"
          subtitle="Ability"
          description={{
            description: "Lowers opposing Pokémon's Attack stat.",
            source: "Scarlet / Violet",
          }}
        />,
      );
      expect(html).toContain("Lowers opposing Pokémon&#x27;s Attack stat.");
    });

    it("omits flags section when no flags provided", () => {
      const html = renderToStaticMarkup(
        <SimpleDataTooltipCard name="None" subtitle="Item" flags={[]} />,
      );
      expect(html).toContain("None");
      expect(html).toContain("Item");
      expect(html).not.toContain("Flags");
    });
  });

  describe("TooltipFlagsSection", () => {
    it("renders sorted flag badges when flags are provided", () => {
      const html = renderToStaticMarkup(
        <TooltipFlagsSection flags={["Protect", "Contact", "Mirror"]} />,
      );
      expect(html).toContain("Flags");
      expect(html).toContain("Contact");
      expect(html).toContain("Mirror");
      expect(html).toContain("Protect");
      // Verify alphabetical order in output
      const contactIdx = html.indexOf("Contact");
      const mirrorIdx = html.indexOf("Mirror");
      const protectIdx = html.indexOf("Protect");
      expect(contactIdx).toBeLessThan(mirrorIdx);
      expect(mirrorIdx).toBeLessThan(protectIdx);
    });

    it("returns null / empty markup when flags is null, undefined, or empty", () => {
      expect(renderToStaticMarkup(<TooltipFlagsSection flags={null} />)).toBe("");
      expect(renderToStaticMarkup(<TooltipFlagsSection flags={undefined} />)).toBe("");
      expect(renderToStaticMarkup(<TooltipFlagsSection flags={[]} />)).toBe("");
    });
  });
});
