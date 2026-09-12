import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import type { MoveData } from "battler-types";
import { describe, expect, it } from "vitest";
import { formatMoveEffects } from "../../src/moves/formatter.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

function loadMoveData(genFile: string, moveId: string): MoveData {
  const filePath = path.resolve(__dirname, `../../../../battle-data/data/moves/${genFile}`);
  const content = JSON.parse(fs.readFileSync(filePath, "utf-8"));
  if (!content[moveId]) {
    throw new Error(`Move '${moveId}' not found in ${genFile}`);
  }
  return content[moveId];
}

describe("formatMoveEffects", () => {
  describe("Rule 1: Primary hit_effect never has a chance percentage", () => {
    it("formats primary status moves as declarative sentences", () => {
      const thunderWave: MoveData = {
        name: "Thunder Wave",
        category: "Status",
        primary_type: "Electric",
        base_power: 0,
        accuracy: 90,
        pp: 20,
        priority: 0,
        target: "Normal",
        flags: [],
        damage: null,
        no_pp_boosts: false,
        ohko_type: null,
        user_switch: null,
        self_destruct: null,
        recoil: null,
        drain_percent: null,
        force_stab: false,
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
        user_effect: null,
        user_effect_chance: null,
        secondary_effects: [],
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
        z_move: null,
        max_move: null,
        effect: null,
        condition: null,
      };

      const effects = formatMoveEffects(thunderWave);
      expect(effects).toHaveLength(1);
      expect(effects[0].chance).toBeUndefined();
      expect(effects[0].text).toBe("Paralyzes the target.");
      expect(effects[0].type).toBe("status");
      expect(effects[0].subject).toBe("target");
    });

    it("formats toxic and will-o-wisp", () => {
      const toxic = loadMoveData("gen1.json", "toxic");
      const toxicEff = formatMoveEffects(toxic);
      expect(toxicEff).toHaveLength(1);
      expect(toxicEff[0].chance).toBeUndefined();
      expect(toxicEff[0].text).toBe("Badly poisons the target.");

      const wow = loadMoveData("gen3.json", "willowisp");
      const wowEff = formatMoveEffects(wow);
      expect(wowEff).toHaveLength(1);
      expect(wowEff[0].chance).toBeUndefined();
      expect(wowEff[0].text).toBe("Burns the target.");
    });

    it("formats healing moves on user and allies concisely without 'up to'", () => {
      const recover = loadMoveData("gen1.json", "recover");
      const recoverEff = formatMoveEffects(recover);
      expect(recoverEff).toHaveLength(1);
      expect(recoverEff[0].chance).toBeUndefined();
      expect(recoverEff[0].text).toBe("Restores 50% of the user's HP.");

      const softboiled = loadMoveData("gen1.json", "softboiled");
      const softEff = formatMoveEffects(softboiled);
      expect(softEff).toHaveLength(1);
      expect(softEff[0].text).toBe("Restores 50% of the user's HP.");

      const lifedew = loadMoveData("gen8.json", "lifedew");
      const lifedewEff = formatMoveEffects(lifedew);
      expect(lifedewEff).toHaveLength(1);
      expect(lifedewEff[0].chance).toBeUndefined();
      expect(lifedewEff[0].text).toBe("Restores 25% of the user's and ally's HP.");
    });

    it("formats stat boosts on user", () => {
      const swordsDance = loadMoveData("gen1.json", "swordsdance");
      const sdEff = formatMoveEffects(swordsDance);
      expect(sdEff).toHaveLength(1);
      expect(sdEff[0].chance).toBeUndefined();
      expect(sdEff[0].text).toBe("Raises the user's Attack by 2 stages.");

      const cottonGuard = loadMoveData("gen5.json", "cottonguard");
      const cgEff = formatMoveEffects(cottonGuard);
      expect(cgEff).toHaveLength(1);
      expect(cgEff[0].text).toBe("Raises the user's Defense by 3 stages.");

      const honeClaws = loadMoveData("gen5.json", "honeclaws");
      const hcEff = formatMoveEffects(honeClaws);
      expect(hcEff).toHaveLength(1);
      expect(hcEff[0].text).toBe("Raises the user's Attack and Accuracy by 1 stage.");

      const quiverDance = loadMoveData("gen5.json", "quiverdance");
      const qdEff = formatMoveEffects(quiverDance);
      expect(qdEff).toHaveLength(1);
      expect(qdEff[0].text).toBe("Raises the user's Sp. Atk, Sp. Def, and Speed by 1 stage.");
    });

    it("formats omnibus all-stats boost on user", () => {
      const clangorousSoul = loadMoveData("gen8.json", "clangoroussoul");
      const csEff = formatMoveEffects(clangorousSoul);
      expect(csEff).toHaveLength(1);
      expect(csEff[0].chance).toBeUndefined();
      expect(csEff[0].text).toBe("Raises all of the user's stats by 1 stage.");
    });

    it("formats stat drops on target", () => {
      const screech = loadMoveData("gen1.json", "screech");
      const screechEff = formatMoveEffects(screech);
      expect(screechEff).toHaveLength(1);
      expect(screechEff[0].chance).toBeUndefined();
      expect(screechEff[0].text).toBe("Lowers the target's Defense by 2 stages.");
    });

    it("formats compound moves with status and boost (Toxic Thread)", () => {
      const toxicThread = loadMoveData("gen7.json", "toxicthread");
      const ttEff = formatMoveEffects(toxicThread);
      expect(ttEff).toHaveLength(2);
      expect(ttEff[0].text).toBe("Poisons the target.");
      expect(ttEff[1].text).toBe("Lowers the target's Speed by 1 stage.");
    });
  });

  describe("Rule 2: Primary user_effect only shows chance if user_effect_chance is defined", () => {
    it("formats Close Combat with no chance", () => {
      const closeCombat = loadMoveData("gen4.json", "closecombat");
      const ccEff = formatMoveEffects(closeCombat);
      expect(ccEff).toHaveLength(1);
      expect(ccEff[0].chance).toBeUndefined();
      expect(ccEff[0].text).toBe("Lowers the user's Defense and Sp. Atk by 1 stage.");
      expect(ccEff[0].subject).toBe("user");
    });

    it("formats Armor Cannon with Defense and Sp. Def drop", () => {
      const armorCannon = loadMoveData("gen9.json", "armorcannon");
      const acEff = formatMoveEffects(armorCannon);
      expect(acEff).toHaveLength(1);
      expect(acEff[0].chance).toBeUndefined();
      expect(acEff[0].text).toBe("Lowers the user's Defense and Sp. Def by 1 stage.");
      expect(acEff[0].subject).toBe("user");
    });

    it("formats Superpower with no chance", () => {
      const superpower = loadMoveData("gen3.json", "superpower");
      const spEff = formatMoveEffects(superpower);
      expect(spEff).toHaveLength(1);
      expect(spEff[0].chance).toBeUndefined();
      expect(spEff[0].text).toBe("Lowers the user's Attack and Defense by 1 stage.");
    });

    it("formats Draco Meteor with no chance", () => {
      const dracoMeteor = loadMoveData("gen4.json", "dracometeor");
      const dmEff = formatMoveEffects(dracoMeteor);
      expect(dmEff).toHaveLength(1);
      expect(dmEff[0].chance).toBeUndefined();
      expect(dmEff[0].text).toBe("Lowers the user's Sp. Atk by 2 stages.");
    });

    it("formats V-create with 3 stat drops and Oxford comma", () => {
      const vCreate = loadMoveData("gen5.json", "vcreate");
      const vcEff = formatMoveEffects(vCreate);
      expect(vcEff).toHaveLength(1);
      expect(vcEff[0].chance).toBeUndefined();
      expect(vcEff[0].text).toBe("Lowers the user's Defense, Sp. Def, and Speed by 1 stage.");
    });

    it("formats user_effect with user_effect_chance when provided", () => {
      const mockMoveWithChance: MoveData = {
        ...loadMoveData("gen9.json", "armorcannon"),
        user_effect_chance: "50%",
      };
      const eff = formatMoveEffects(mockMoveWithChance);
      expect(eff).toHaveLength(1);
      expect(eff[0].chance).toBe("50%");
      expect(eff[0].text).toBe("50% chance to lower the user's Defense and Sp. Def by 1 stage.");
    });
  });

  describe("Rule 3: secondary_effects ALWAYS show a chance %, even if 100%", () => {
    it("formats secondary status conditions with explicit chance", () => {
      const thunderbolt = loadMoveData("gen1.json", "thunderbolt");
      const tbEff = formatMoveEffects(thunderbolt);
      expect(tbEff).toHaveLength(1);
      expect(tbEff[0].chance).toBe("10%");
      expect(tbEff[0].text).toBe("10% chance to paralyze the target.");

      const flamethrower = loadMoveData("gen1.json", "flamethrower");
      const ftEff = formatMoveEffects(flamethrower);
      expect(ftEff).toHaveLength(1);
      expect(ftEff[0].chance).toBe("10%");
      expect(ftEff[0].text).toBe("10% chance to burn the target.");

      const iceBeam = loadMoveData("gen1.json", "icebeam");
      const ibEff = formatMoveEffects(iceBeam);
      expect(ibEff).toHaveLength(1);
      expect(ibEff[0].chance).toBe("10%");
      expect(ibEff[0].text).toBe("10% chance to freeze the target.");

      const scald = loadMoveData("gen5.json", "scald");
      const scaldEff = formatMoveEffects(scald);
      expect(scaldEff).toHaveLength(1);
      expect(scaldEff[0].chance).toBe("30%");
      expect(scaldEff[0].text).toBe("30% chance to burn the target.");

      const sludgeBomb = loadMoveData("gen2.json", "sludgebomb");
      const sbEff = formatMoveEffects(sludgeBomb);
      expect(sbEff).toHaveLength(1);
      expect(sbEff[0].chance).toBe("30%");
      expect(sbEff[0].text).toBe("30% chance to poison the target.");
    });

    it("formats secondary effects where chance is omitted as 100%", () => {
      const flameCharge = loadMoveData("gen5.json", "flamecharge");
      const fcEff = formatMoveEffects(flameCharge);
      expect(fcEff).toHaveLength(1);
      expect(fcEff[0].chance).toBe("100%");
      expect(fcEff[0].text).toBe("100% chance to raise the user's Speed by 1 stage.");

      const acidSpray = loadMoveData("gen5.json", "acidspray");
      const asEff = formatMoveEffects(acidSpray);
      expect(asEff).toHaveLength(1);
      expect(asEff[0].chance).toBe("100%");
      expect(asEff[0].text).toBe("100% chance to lower the target's Sp. Def by 2 stages.");

      const inferno = loadMoveData("gen5.json", "inferno");
      const infernoEff = formatMoveEffects(inferno);
      expect(infernoEff).toHaveLength(1);
      expect(infernoEff[0].chance).toBe("100%");
      expect(infernoEff[0].text).toBe("100% chance to burn the target.");
    });

    it("formats secondary stat drop on target", () => {
      const acid = loadMoveData("gen1.json", "acid");
      const acidEff = formatMoveEffects(acid);
      expect(acidEff).toHaveLength(1);
      expect(acidEff[0].chance).toBe("10%");
      expect(acidEff[0].text).toBe("10% chance to lower the target's Sp. Def by 1 stage.");

      const psychic = loadMoveData("gen1.json", "psychic");
      const psychicEff = formatMoveEffects(psychic);
      expect(psychicEff).toHaveLength(1);
      expect(psychicEff[0].chance).toBe("10%");
      expect(psychicEff[0].text).toBe("10% chance to lower the target's Sp. Def by 1 stage.");

      const shadowBall = loadMoveData("gen2.json", "shadowball");
      const sbEff = formatMoveEffects(shadowBall);
      expect(sbEff).toHaveLength(1);
      expect(sbEff[0].chance).toBe("20%");
      expect(sbEff[0].text).toBe("20% chance to lower the target's Sp. Def by 1 stage.");

      const crunch = loadMoveData("gen2.json", "crunch");
      const crunchEff = formatMoveEffects(crunch);
      expect(crunchEff).toHaveLength(1);
      expect(crunchEff[0].chance).toBe("20%");
      expect(crunchEff[0].text).toBe("20% chance to lower the target's Defense by 1 stage.");

      const bulldoze = loadMoveData("gen5.json", "bulldoze");
      const bdEff = formatMoveEffects(bulldoze);
      expect(bdEff).toHaveLength(1);
      expect(bdEff[0].chance).toBe("100%");
      expect(bdEff[0].text).toBe("100% chance to lower the target's Speed by 1 stage.");
    });

    it("formats secondary omnibus stat boost (Ancient Power / Silver Wind)", () => {
      const ancientPower = loadMoveData("gen2.json", "ancientpower");
      const apEff = formatMoveEffects(ancientPower);
      expect(apEff).toHaveLength(1);
      expect(apEff[0].chance).toBe("10%");
      expect(apEff[0].text).toBe("10% chance to raise all of the user's stats by 1 stage.");

      const silverWind = loadMoveData("gen3.json", "silverwind");
      const swEff = formatMoveEffects(silverWind);
      expect(swEff).toHaveLength(1);
      expect(swEff[0].chance).toBe("10%");
      expect(swEff[0].text).toBe("10% chance to raise all of the user's stats by 1 stage.");
    });

    it("formats secondary single stat user boost (Meteor Mash / Power-Up Punch)", () => {
      const meteorMash = loadMoveData("gen3.json", "meteormash");
      const mmEff = formatMoveEffects(meteorMash);
      expect(mmEff).toHaveLength(1);
      expect(mmEff[0].chance).toBe("20%");
      expect(mmEff[0].text).toBe("20% chance to raise the user's Attack by 1 stage.");

      const powerUpPunch = loadMoveData("gen6.json", "poweruppunch");
      const pupEff = formatMoveEffects(powerUpPunch);
      expect(pupEff).toHaveLength(1);
      expect(pupEff[0].chance).toBe("100%");
      expect(pupEff[0].text).toBe("100% chance to raise the user's Attack by 1 stage.");
    });
  });

  describe("Multi-Effect & Multi-Clause Moves", () => {
    it("splits Shell Smash into positive raises and negative drops", () => {
      const shellSmash = loadMoveData("gen5.json", "shellsmash");
      const ssEff = formatMoveEffects(shellSmash);
      expect(ssEff).toHaveLength(2);
      expect(ssEff[0].text).toBe("Raises the user's Attack, Sp. Atk, and Speed by 2 stages.");
      expect(ssEff[1].text).toBe("Lowers the user's Defense and Sp. Def by 1 stage.");
    });

    it("splits Scale Shot into drops and raises", () => {
      const scaleShot = loadMoveData("gen8.json", "scaleshot");
      const scEff = formatMoveEffects(scaleShot);
      expect(scEff).toHaveLength(2);
      expect(scEff[0].text).toBe("Raises the user's Speed by 1 stage.");
      expect(scEff[1].text).toBe("Lowers the user's Defense by 1 stage.");
    });

    it("formats moves with multiple secondary effects", () => {
      const mockMultiSec: MoveData = {
        ...loadMoveData("gen1.json", "thunderbolt"),
        secondary_effects: [
          {
            chance: "10%",
            apply_once: false,
            target: {
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
            user: null,
            source_effect: null,
            effect: null,
          },
          {
            chance: "10%",
            apply_once: false,
            target: {
              status: "brn",
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
            user: null,
            source_effect: null,
            effect: null,
          },
        ],
      };
      const taEff = formatMoveEffects(mockMultiSec);
      expect(taEff).toHaveLength(2);
      expect(taEff[0].text).toBe("10% chance to paralyze the target.");
      expect(taEff[1].text).toBe("10% chance to burn the target.");

      const thunderFang = loadMoveData("gen4.json", "thunderfang");
      const tfEff = formatMoveEffects(thunderFang);
      expect(tfEff).toHaveLength(2);
      expect(tfEff[0].text).toBe("10% chance to paralyze the target.");
      expect(tfEff[1].text).toBe("10% chance to make the target flinch.");
    });

    it("returns empty array for moves with no effects", () => {
      const tackle = loadMoveData("gen1.json", "tackle");
      expect(formatMoveEffects(tackle)).toEqual([]);

      const earthquake = loadMoveData("gen1.json", "earthquake");
      expect(formatMoveEffects(earthquake)).toEqual([]);
    });
  });

  describe("Volatiles: confusion and flinch", () => {
    it("formats primary confusion moves without chance", () => {
      const confuseRay = loadMoveData("gen1.json", "confuseray");
      const crEff = formatMoveEffects(confuseRay);
      expect(crEff).toHaveLength(1);
      expect(crEff[0].chance).toBeUndefined();
      expect(crEff[0].text).toBe("Confuses the target.");
      expect(crEff[0].type).toBe("volatile_status");

      const supersonic = loadMoveData("gen1.json", "supersonic");
      const ssEff = formatMoveEffects(supersonic);
      expect(ssEff).toHaveLength(1);
      expect(ssEff[0].text).toBe("Confuses the target.");
    });

    it("formats secondary flinch effects with chance", () => {
      const headbutt = loadMoveData("gen1.json", "headbutt");
      const hbEff = formatMoveEffects(headbutt);
      expect(hbEff).toHaveLength(1);
      expect(hbEff[0].chance).toBe("30%");
      expect(hbEff[0].text).toBe("30% chance to make the target flinch.");
      expect(hbEff[0].type).toBe("volatile_status");

      const ironHead = loadMoveData("gen4.json", "ironhead");
      const ihEff = formatMoveEffects(ironHead);
      expect(ihEff).toHaveLength(1);
      expect(ihEff[0].chance).toBe("30%");
      expect(ihEff[0].text).toBe("30% chance to make the target flinch.");

      const fakeOut = loadMoveData("gen3.json", "fakeout");
      const foEff = formatMoveEffects(fakeOut);
      expect(foEff).toHaveLength(1);
      expect(foEff[0].chance).toBe("100%");
      expect(foEff[0].text).toBe("100% chance to make the target flinch.");
    });

    it("formats secondary confusion effects with chance", () => {
      const dynamicPunch = loadMoveData("gen2.json", "dynamicpunch");
      const dpEff = formatMoveEffects(dynamicPunch);
      expect(dpEff).toHaveLength(1);
      expect(dpEff[0].chance).toBe("100%");
      expect(dpEff[0].text).toBe("100% chance to confuse the target.");

      const waterPulse = loadMoveData("gen3.json", "waterpulse");
      const wpEff = formatMoveEffects(waterPulse);
      expect(wpEff).toHaveLength(1);
      expect(wpEff[0].chance).toBe("20%");
      expect(wpEff[0].text).toBe("20% chance to confuse the target.");

      const hurricane = loadMoveData("gen5.json", "hurricane");
      const hcEff = formatMoveEffects(hurricane);
      expect(hcEff).toHaveLength(1);
      expect(hcEff[0].chance).toBe("30%");
      expect(hcEff[0].text).toBe("30% chance to confuse the target.");
    });

    it("ignores unsupported volatile statuses", () => {
      const gigaImpact = loadMoveData("gen4.json", "gigaimpact");
      // volatile_status: mustrecharge
      expect(formatMoveEffects(gigaImpact)).toEqual([]);

      const roost = loadMoveData("gen4.json", "roost");
      // volatile_status: roost is ignored, only heal is formatted
      const roostEff = formatMoveEffects(roost);
      expect(roostEff).toHaveLength(1);
      expect(roostEff[0].type).toBe("heal");
      expect(roostEff[0].text).toBe("Restores 50% of the user's HP.");
    });
  });
});
