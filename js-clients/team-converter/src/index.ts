import type {
  MonData,
  Gender,
  Nature,
  StatTable,
  Type,
  MonPersistentBattleData,
} from "battler-types";

// Helper to normalize Nature names
function normalizeNature(name: string): Nature {
  const normalized = name.charAt(0).toUpperCase() + name.slice(1).toLowerCase();
  const validNatures: Nature[] = [
    "Hardy",
    "Lonely",
    "Adamant",
    "Naughty",
    "Brave",
    "Bold",
    "Docile",
    "Impish",
    "Lax",
    "Relaxed",
    "Modest",
    "Mild",
    "Bashful",
    "Rash",
    "Quiet",
    "Calm",
    "Gentle",
    "Careful",
    "Quirky",
    "Sassy",
    "Timid",
    "Hasty",
    "Jolly",
    "Naive",
    "Serious",
  ];
  if (validNatures.includes(normalized as Nature)) {
    return normalized as Nature;
  }
  return "Serious";
}

// Helper to normalize Type names
function normalizeType(name: string): Type | null {
  const normalized = name.charAt(0).toUpperCase() + name.slice(1).toLowerCase();
  const validTypes: Type[] = [
    "Normal",
    "Fighting",
    "Flying",
    "Poison",
    "Ground",
    "Rock",
    "Bug",
    "Ghost",
    "Steel",
    "Fire",
    "Water",
    "Grass",
    "Electric",
    "Psychic",
    "Ice",
    "Dragon",
    "Dark",
    "Fairy",
    "None",
    "Stellar",
  ];
  if (validTypes.includes(normalized as Type)) {
    return normalized as Type;
  }
  return null;
}

interface MutableMonData {
  name?: string;
  species?: string;
  ability?: string;
  moves: string[];
  item?: string | null;
  pp_boosts?: number[];
  nature?: Nature;
  true_nature?: Nature | null;
  gender?: Gender;
  evs?: StatTable;
  ivs?: StatTable;
  level?: number;
  experience?: number;
  shiny?: boolean;
  friendship?: number;
  ball?: string | null;
  hidden_power_type?: Type | null;
  different_original_trainer?: boolean;
  dynamax_level?: number;
  gigantamax_factor?: boolean;
  tera_type?: Type | null;
  persistent_battle_data?: MonPersistentBattleData;
}

// Parse Showdown text to MonData[]
export function parseShowdown(text: string): MonData[] {
  const lines = text.split(/\r?\n/);
  const teams: MonData[] = [];
  let currentMon: MutableMonData = {
    moves: [],
  };

  // Helper to push current mon if valid
  const pushCurrentMon = () => {
    if (currentMon.species) {
      const name = currentMon.name || currentMon.species;
      const mon: any = {
        name,
        species: currentMon.species,
        moves: currentMon.moves,
      };

      if (currentMon.ability !== undefined) mon.ability = currentMon.ability;
      if (currentMon.item !== undefined) mon.item = currentMon.item;
      if (currentMon.pp_boosts !== undefined) mon.pp_boosts = currentMon.pp_boosts;
      if (currentMon.nature !== undefined) mon.nature = currentMon.nature;
      if (currentMon.true_nature !== undefined) mon.true_nature = currentMon.true_nature;
      if (currentMon.gender !== undefined) mon.gender = currentMon.gender;
      if (currentMon.evs !== undefined) mon.evs = currentMon.evs;
      if (currentMon.ivs !== undefined) mon.ivs = currentMon.ivs;
      if (currentMon.level !== undefined) mon.level = currentMon.level;
      if (currentMon.experience !== undefined) mon.experience = currentMon.experience;
      if (currentMon.shiny !== undefined) mon.shiny = currentMon.shiny;
      if (currentMon.friendship !== undefined) mon.friendship = currentMon.friendship;
      if (currentMon.ball !== undefined) mon.ball = currentMon.ball;
      if (currentMon.hidden_power_type !== undefined)
        mon.hidden_power_type = currentMon.hidden_power_type;
      if (currentMon.different_original_trainer !== undefined)
        mon.different_original_trainer = currentMon.different_original_trainer;
      if (currentMon.dynamax_level !== undefined) mon.dynamax_level = currentMon.dynamax_level;
      if (currentMon.gigantamax_factor !== undefined)
        mon.gigantamax_factor = currentMon.gigantamax_factor;
      if (currentMon.tera_type !== undefined) mon.tera_type = currentMon.tera_type;
      if (currentMon.persistent_battle_data !== undefined)
        mon.persistent_battle_data = currentMon.persistent_battle_data;

      teams.push(mon as MonData);
    }
    currentMon = { moves: [] };
  };

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (!line) {
      pushCurrentMon();
      continue;
    }

    // Parse moves starting with "-"
    if (line.startsWith("-")) {
      const move = line.slice(1).trim();
      if (move) {
        currentMon.moves.push(move);
        // Check for Hidden Power type (e.g. "Hidden Power [Ice]")
        const hpMatch = move.match(/Hidden Power\s*\[([a-zA-Z]+)\]/i);
        if (hpMatch) {
          const type = normalizeType(hpMatch[1]);
          if (type) {
            currentMon.hidden_power_type = type;
          }
        }
      }
      continue;
    }

    // Parse options
    if (line.includes(":")) {
      const parts = line.split(":");
      const key = parts[0].trim().toLowerCase();
      const val = parts.slice(1).join(":").trim();

      if (key === "ability") {
        currentMon.ability = val;
      } else if (key === "level") {
        const lvl = parseInt(val, 10);
        if (!isNaN(lvl)) {
          currentMon.level = lvl;
        }
      } else if (key === "shiny") {
        currentMon.shiny = val.toLowerCase() === "yes";
      } else if (key === "friendship") {
        const f = parseInt(val, 10);
        if (!isNaN(f)) {
          currentMon.friendship = f;
        }
      } else if (key === "evs") {
        const evs = { hp: 0, atk: 0, def: 0, spa: 0, spd: 0, spe: 0 };
        const tokens = val.split("/");
        for (const token of tokens) {
          const pair = token.trim().split(/\s+/);
          if (pair.length >= 2) {
            const v = parseInt(pair[0], 10);
            const stat = pair.slice(1).join(" ").toLowerCase();
            if (!isNaN(v)) {
              if (stat === "hp") evs.hp = v;
              else if (stat === "atk" || stat === "attack") evs.atk = v;
              else if (stat === "def" || stat === "defense") evs.def = v;
              else if (
                stat === "spa" ||
                stat === "sp. atk" ||
                stat === "spatk" ||
                stat === "special attack"
              )
                evs.spa = v;
              else if (
                stat === "spd" ||
                stat === "sp. def" ||
                stat === "spdef" ||
                stat === "special defense"
              )
                evs.spd = v;
              else if (stat === "spe" || stat === "speed") evs.spe = v;
            }
          }
        }
        currentMon.evs = evs;
      } else if (key === "ivs") {
        const ivs = { hp: 31, atk: 31, def: 31, spa: 31, spd: 31, spe: 31 };
        const tokens = val.split("/");
        for (const token of tokens) {
          const pair = token.trim().split(/\s+/);
          if (pair.length >= 2) {
            const v = parseInt(pair[0], 10);
            const stat = pair.slice(1).join(" ").toLowerCase();
            if (!isNaN(v)) {
              if (stat === "hp") ivs.hp = v;
              else if (stat === "atk" || stat === "attack") ivs.atk = v;
              else if (stat === "def" || stat === "defense") ivs.def = v;
              else if (
                stat === "spa" ||
                stat === "sp. atk" ||
                stat === "spatk" ||
                stat === "special attack"
              )
                ivs.spa = v;
              else if (
                stat === "spd" ||
                stat === "sp. def" ||
                stat === "spdef" ||
                stat === "special defense"
              )
                ivs.spd = v;
              else if (stat === "spe" || stat === "speed") ivs.spe = v;
            }
          }
        }
        currentMon.ivs = ivs;
      } else if (key === "nature") {
        currentMon.nature = normalizeNature(val);
      } else if (
        key === "gigantamax" ||
        key === "gmax" ||
        key === "gigantamax factor" ||
        key === "gigantamax_factor"
      ) {
        currentMon.gigantamax_factor = val.toLowerCase() === "yes";
      } else if (key === "tera type") {
        const t = normalizeType(val);
        if (t) {
          currentMon.tera_type = t;
        }
      }
      continue;
    }

    // Check if line ends with " Nature"
    if (line.toLowerCase().endsWith(" nature")) {
      const nat = line.slice(0, -7).trim();
      currentMon.nature = normalizeNature(nat);
      continue;
    }

    // If it doesn't match any of the above, it's the first line!
    // Format: Nickname (Species) (Gender) @ Item
    // Let's parse this line
    let nameAndGender = line;
    let item: string | null = null;

    if (line.includes("@")) {
      const idx = line.lastIndexOf("@");
      nameAndGender = line.slice(0, idx).trim();
      item = line.slice(idx + 1).trim();
    }

    let gender: Gender | undefined = undefined;
    if (/\((M|F)\)$/i.test(nameAndGender)) {
      const match = nameAndGender.match(/\((M|F)\)$/i);
      if (match) {
        gender = match[1].toUpperCase() as Gender;
        nameAndGender = nameAndGender.slice(0, nameAndGender.lastIndexOf("(")).trim();
      }
    }

    let species = nameAndGender;
    let nickname: string | null = null;
    if (nameAndGender.endsWith(")")) {
      const openIdx = nameAndGender.lastIndexOf("(");
      const closeIdx = nameAndGender.lastIndexOf(")");
      if (openIdx !== -1 && closeIdx !== -1 && openIdx < closeIdx) {
        nickname = nameAndGender.slice(0, openIdx).trim();
        species = nameAndGender.slice(openIdx + 1, closeIdx).trim();
      }
    }

    currentMon.species = species;
    if (nickname) {
      currentMon.name = nickname;
    } else {
      currentMon.name = species;
    }
    if (item) {
      currentMon.item = item;
    }
    if (gender !== undefined) {
      currentMon.gender = gender;
    }
  }

  // Push final mon if any remains
  pushCurrentMon();

  return teams;
}

// Export MonData[] to Showdown text format
export function exportToShowdown(team: MonData[]): string {
  const blocks: string[] = [];

  for (const mon of team) {
    const lines: string[] = [];

    // First line: Nickname (Species) (Gender) @ Item
    let firstLine = "";
    if (mon.name && mon.name !== mon.species) {
      firstLine = `${mon.name} (${mon.species})`;
    } else {
      firstLine = mon.species;
    }

    if (mon.gender === "M" || mon.gender === "F") {
      firstLine += ` (${mon.gender})`;
    }

    if (mon.item) {
      firstLine += ` @ ${mon.item}`;
    }
    lines.push(firstLine);

    // Ability
    if (mon.ability) {
      lines.push(`Ability: ${mon.ability}`);
    }

    // Level (only if not 100)
    if (mon.level !== undefined && mon.level !== 100) {
      lines.push(`Level: ${mon.level}`);
    }

    // Shiny
    if (mon.shiny) {
      lines.push("Shiny: Yes");
    }

    // Friendship
    if (mon.friendship !== undefined && mon.friendship !== 255) {
      lines.push(`Friendship: ${mon.friendship}`);
    }

    // EVs
    if (mon.evs) {
      const evParts: string[] = [];
      if (mon.evs.hp) evParts.push(`${mon.evs.hp} HP`);
      if (mon.evs.atk) evParts.push(`${mon.evs.atk} Atk`);
      if (mon.evs.def) evParts.push(`${mon.evs.def} Def`);
      if (mon.evs.spa) evParts.push(`${mon.evs.spa} SpA`);
      if (mon.evs.spd) evParts.push(`${mon.evs.spd} SpD`);
      if (mon.evs.spe) evParts.push(`${mon.evs.spe} Spe`);
      if (evParts.length > 0) {
        lines.push(`EVs: ${evParts.join(" / ")}`);
      }
    }

    // IVs
    if (mon.ivs) {
      const ivParts: string[] = [];
      // Omit IV lines if they are 31, only export if different
      if (mon.ivs.hp !== undefined && mon.ivs.hp !== 31) ivParts.push(`${mon.ivs.hp} HP`);
      if (mon.ivs.atk !== undefined && mon.ivs.atk !== 31) ivParts.push(`${mon.ivs.atk} Atk`);
      if (mon.ivs.def !== undefined && mon.ivs.def !== 31) ivParts.push(`${mon.ivs.def} Def`);
      if (mon.ivs.spa !== undefined && mon.ivs.spa !== 31) ivParts.push(`${mon.ivs.spa} SpA`);
      if (mon.ivs.spd !== undefined && mon.ivs.spd !== 31) ivParts.push(`${mon.ivs.spd} SpD`);
      if (mon.ivs.spe !== undefined && mon.ivs.spe !== 31) ivParts.push(`${mon.ivs.spe} Spe`);
      if (ivParts.length > 0) {
        lines.push(`IVs: ${ivParts.join(" / ")}`);
      }
    }

    // Nature
    if (mon.nature) {
      lines.push(`${mon.nature} Nature`);
    }

    // Gigantamax
    if (mon.gigantamax_factor) {
      lines.push("Gigantamax: Yes");
    }

    // Tera Type
    if (mon.tera_type && mon.tera_type !== "None") {
      lines.push(`Tera Type: ${mon.tera_type}`);
    }

    // Moves
    if (mon.moves) {
      for (const move of mon.moves) {
        lines.push(`- ${move}`);
      }
    }

    blocks.push(lines.join("\n"));
  }

  return blocks.join("\n\n");
}
