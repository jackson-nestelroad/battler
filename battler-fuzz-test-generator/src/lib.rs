use std::{
    collections::{
        HashMap,
        HashSet,
    },
    env,
    fs::File,
    path::Path,
};

use anyhow::{
    Context,
    Result,
};
use battler::{
    BattleType,
    CoreBattleOptions,
    FieldData,
    FormatData,
    Gender,
    Nature,
    PlayerData,
    PlayerDex,
    PlayerOptions,
    PlayerType,
    Rule,
    SerializedRuleSet,
    SideData,
    Stat,
    StatTable,
    TeamData,
    Type,
    teams::MonData,
};
use battler_local_data::LocalDataStore;
use rand::prelude::*;
use serde::Deserialize;

const ALL_NATURES: &[Nature] = &[
    Nature::Hardy,
    Nature::Lonely,
    Nature::Adamant,
    Nature::Naughty,
    Nature::Brave,
    Nature::Bold,
    Nature::Docile,
    Nature::Impish,
    Nature::Lax,
    Nature::Relaxed,
    Nature::Modest,
    Nature::Mild,
    Nature::Bashful,
    Nature::Rash,
    Nature::Quiet,
    Nature::Calm,
    Nature::Gentle,
    Nature::Careful,
    Nature::Quirky,
    Nature::Sassy,
    Nature::Timid,
    Nature::Hasty,
    Nature::Jolly,
    Nature::Naive,
    Nature::Serious,
];

// Helper structs to parse minimal data from items files.
#[derive(Deserialize)]
struct MegaEvolutionData {
    from: String,
}

#[derive(Deserialize)]
struct MegaStoneSpecialData {
    mega_evolution: Option<MegaEvolutionData>,
}

#[derive(Deserialize)]
struct MegaStoneEntry {
    special_data: Option<MegaStoneSpecialData>,
}

#[derive(Deserialize)]
struct ZCrystalData {
    #[serde(rename = "type")]
    typ: Option<Type>,
}

#[derive(Deserialize)]
struct ZCrystalSpecialData {
    z_crystal: Option<ZCrystalData>,
}

#[derive(Deserialize)]
struct ZCrystalEntry {
    special_data: Option<ZCrystalSpecialData>,
}

/// Generates a valid, random Pokémon battle configuration.
pub fn generate_random_battle(
    store: &LocalDataStore,
    seed: Option<u64>,
) -> Result<CoreBattleOptions> {
    let actual_seed = seed.unwrap_or_else(|| rand::rng().random());
    let mut rng = StdRng::seed_from_u64(actual_seed);

    // 1. Locate items path and load candidate lists.
    let data_dir = env::var("DATA_DIR").context("DATA_DIR environment variable is not defined")?;
    let items_path = Path::new(&data_dir).join("items");

    // Load regular items keys.
    let items_file =
        File::open(items_path.join("items.json")).context("failed to open items.json")?;
    let items_map: HashMap<String, serde_json::Value> =
        serde_json::from_reader(items_file).context("failed to parse items.json")?;
    let items_pool: Vec<String> = items_map.keys().cloned().collect();

    // Load megastones.
    let megastones_file =
        File::open(items_path.join("megastones.json")).context("failed to open megastones.json")?;
    let megastones_map: HashMap<String, MegaStoneEntry> =
        serde_json::from_reader(megastones_file).context("failed to parse megastones.json")?;

    // Load zcrystals.
    let zcrystals_file =
        File::open(items_path.join("zcrystals.json")).context("failed to open zcrystals.json")?;
    let zcrystals_map: HashMap<String, ZCrystalEntry> =
        serde_json::from_reader(zcrystals_file).context("failed to parse zcrystals.json")?;

    // Map Type -> Z-Crystal ID
    let mut type_to_zcrystal = HashMap::new();
    for (id, entry) in &zcrystals_map {
        if let Some(special_data) = &entry.special_data {
            if let Some(z_crystal) = &special_data.z_crystal {
                if let Some(typ) = z_crystal.typ {
                    type_to_zcrystal.insert(typ, id.clone());
                }
            }
        }
    }

    // 2. Select format rules and mechanics.
    let mut rules = SerializedRuleSet::new();
    rules.insert(Rule::value_name("Species Clause"));
    rules.insert(Rule::value_name("Item Clause"));

    let mut enable_mega = false;
    let mut enable_z_moves = false;
    let mut enable_dynamax = false;
    let mut enable_tera = false;

    // Pick one mechanic to enable, or none.
    // 0: None, 1: Mega Evolution, 2: Z-Moves, 3: Dynamax, 4: Terastallization
    match rng.random_range(0..5) {
        1 => {
            enable_mega = true;
            rules.insert(Rule::value_name("Mega Evolution"));
        }
        2 => {
            enable_z_moves = true;
            rules.insert(Rule::value_name("Z-Moves"));
        }
        3 => {
            enable_dynamax = true;
            rules.insert(Rule::value_name("Dynamax"));
        }
        4 => {
            enable_tera = true;
            rules.insert(Rule::value_name("Terastallization"));
        }
        _ => {}
    }

    let format = FormatData {
        battle_type: BattleType::Doubles,
        rules,
    };

    // 3. Generate side teams.
    let side_1_team = generate_random_team(
        store,
        &mut rng,
        &items_pool,
        &megastones_map,
        &type_to_zcrystal,
        enable_mega,
        enable_z_moves,
        enable_dynamax,
        enable_tera,
    )?;

    let side_2_team = generate_random_team(
        store,
        &mut rng,
        &items_pool,
        &megastones_map,
        &type_to_zcrystal,
        enable_mega,
        enable_z_moves,
        enable_dynamax,
        enable_tera,
    )?;

    Ok(CoreBattleOptions {
        seed: Some(actual_seed),
        format,
        field: FieldData::default(),
        side_1: SideData {
            name: "Side 1".to_string(),
            players: vec![PlayerData {
                id: "player-1".to_string(),
                name: "Player 1".to_string(),
                player_type: PlayerType::Trainer,
                player_options: PlayerOptions::default(),
                team: side_1_team,
                dex: PlayerDex::default(),
            }],
        },
        side_2: SideData {
            name: "Side 2".to_string(),
            players: vec![PlayerData {
                id: "player-2".to_string(),
                name: "Player 2".to_string(),
                player_type: PlayerType::Trainer,
                player_options: PlayerOptions::default(),
                team: side_2_team,
                dex: PlayerDex::default(),
            }],
        },
    })
}

fn generate_random_team(
    store: &LocalDataStore,
    rng: &mut StdRng,
    items_pool: &[String],
    megastones_map: &HashMap<String, MegaStoneEntry>,
    type_to_zcrystal: &HashMap<Type, String>,
    enable_mega: bool,
    enable_z_moves: bool,
    enable_dynamax: bool,
    enable_tera: bool,
) -> Result<TeamData> {
    let mut members = Vec::new();
    let mut chosen_species = HashSet::new();
    let mut chosen_items = HashSet::new();

    // Determine if team gets a mega (if Mega is enabled).
    let mega_index = if enable_mega && rng.random_bool(0.5) {
        Some(rng.random_range(0..4))
    } else {
        None
    };

    // Determine if team gets a Z-Crystal (if Z-Moves is enabled).
    // Can only be held by a non-Mega Mon.
    let z_index = if enable_z_moves && rng.random_bool(0.5) {
        let candidates: Vec<usize> = (0..4).filter(|&i| Some(i) != mega_index).collect();
        if !candidates.is_empty() {
            Some(*candidates.choose(rng).unwrap())
        } else {
            None
        }
    } else {
        None
    };

    // Candidate list of base species (forme is None).
    let base_species_pool: Vec<(&battler::Id, &battler_data::SpeciesData)> = store
        .species
        .iter()
        .filter(|(_, s)| s.forme.is_none() && !s.battle_only_forme && !s.learnset.is_empty())
        .collect();

    for i in 0..4 {
        let mut selected_species_id: Option<battler::Id> = None;
        let mut selected_species_data: Option<&battler_data::SpeciesData> = None;
        let mut held_item = None;

        if Some(i) == mega_index {
            // Force Mega species and Mega Stone.
            // Pick a random Mega Stone that maps to a valid base species.
            let mut megastone_choices: Vec<(&String, &MegaStoneEntry)> =
                megastones_map.iter().collect();
            megastone_choices.shuffle(rng);

            for (stone_id, entry) in megastone_choices {
                if let Some(special_data) = &entry.special_data {
                    if let Some(mega_evo) = &special_data.mega_evolution {
                        let from_species_id = battler::Id::from(mega_evo.from.as_str());
                        if let Some(species_data) = store.species.get(&from_species_id) {
                            if species_data.forme.is_none()
                                && !species_data.learnset.is_empty()
                                && !chosen_species.contains(&from_species_id)
                            {
                                selected_species_id = Some(from_species_id);
                                selected_species_data = Some(species_data);
                                held_item = Some(stone_id.clone());
                                break;
                            }
                        }
                    }
                }
            }
        }

        // If not Mega, or if Mega selection failed, pick a random base species.
        if selected_species_id.is_none() {
            let mut pool = base_species_pool.clone();
            pool.shuffle(rng);
            for (id, s) in pool {
                if !chosen_species.contains(id) {
                    selected_species_id = Some((*id).clone());
                    selected_species_data = Some(s);
                    break;
                }
            }
        }

        let species_id = match selected_species_id {
            Some(id) => id,
            None => return Err(anyhow::anyhow!("Failed to select unique species for team")),
        };
        let species_data = selected_species_data.unwrap();

        chosen_species.insert(species_id);
        let species_name = species_data.name.clone();

        // Random ability.
        let mut abilities = species_data.abilities.clone();
        if let Some(ha) = &species_data.hidden_ability {
            abilities.push(ha.clone());
        }
        let ability = abilities
            .choose(rng)
            .cloned()
            .unwrap_or_else(|| "No Ability".to_string());

        let mut learnset_moves: Vec<&String> = species_data
            .learnset
            .iter()
            .filter(|(move_name, sources)| {
                if !store
                    .moves
                    .contains_key(&battler::Id::from(move_name.as_str()))
                {
                    return false;
                }
                sources.iter().any(|source| match source {
                    battler_data::MoveSource::Level(l) => *l <= 50,
                    _ => true,
                })
            })
            .map(|(move_name, _)| move_name)
            .collect();
        learnset_moves.shuffle(rng);
        let num_moves = std::cmp::min(4, learnset_moves.len());
        let selected_moves: Vec<String> = learnset_moves
            .iter()
            .take(num_moves)
            .map(|s| (*s).clone())
            .collect();

        // Random item / Z-Crystal.
        if held_item.is_none() {
            if Some(i) == z_index {
                // Assign Z-Crystal based on one of the move types.
                let mut move_types: Vec<Type> = selected_moves
                    .iter()
                    .filter_map(|move_name| {
                        let move_id = battler::Id::from(move_name.as_str());
                        store.moves.get(&move_id).map(|m| m.primary_type)
                    })
                    .collect();
                move_types.shuffle(rng);

                for typ in move_types {
                    if let Some(z_crystal) = type_to_zcrystal.get(&typ) {
                        if !chosen_items.contains(z_crystal) {
                            held_item = Some(z_crystal.clone());
                            break;
                        }
                    }
                }
            }

            // Fallback to regular items if no item assigned yet.
            if held_item.is_none() {
                let mut item_choices = items_pool.to_vec();
                item_choices.shuffle(rng);
                for item_name in item_choices {
                    if !chosen_items.contains(&item_name) {
                        held_item = Some(item_name);
                        break;
                    }
                }
            }
        }

        if let Some(ref item) = held_item {
            chosen_items.insert(item.clone());
        }

        // Random nature.
        let nature = *ALL_NATURES.choose(rng).unwrap();

        // Random gender based on species gender ratio.
        let gender = match species_data.gender_ratio {
            0 => Gender::Male,
            254 => Gender::Female,
            255 => Gender::Unknown,
            ratio => {
                let val = rng.random_range(1..=252);
                if val < ratio {
                    Gender::Female
                } else {
                    Gender::Male
                }
            }
        };

        // EVs and IVs.
        let ivs = StatTable {
            hp: 31,
            atk: 31,
            def: 31,
            spa: 31,
            spd: 31,
            spe: 31,
        };

        let mut evs = StatTable::default();
        let mut ev_stats = [
            Stat::HP,
            Stat::Atk,
            Stat::Def,
            Stat::SpAtk,
            Stat::SpDef,
            Stat::Spe,
        ];
        ev_stats.shuffle(rng);
        evs.set(ev_stats[0], 252);
        evs.set(ev_stats[1], 252);

        // Dynamax level.
        let dynamax_level = if enable_dynamax { 10 } else { 0 };

        // Tera type.
        let tera_type = if enable_tera {
            let all_types = [
                Type::Normal,
                Type::Fighting,
                Type::Flying,
                Type::Poison,
                Type::Ground,
                Type::Rock,
                Type::Bug,
                Type::Ghost,
                Type::Steel,
                Type::Fire,
                Type::Water,
                Type::Grass,
                Type::Electric,
                Type::Psychic,
                Type::Ice,
                Type::Dragon,
                Type::Dark,
                Type::Fairy,
            ];
            Some(*all_types.choose(rng).unwrap())
        } else {
            None
        };

        members.push(MonData {
            name: species_name.clone(),
            species: species_name,
            ability,
            moves: selected_moves,
            item: held_item,
            pp_boosts: Vec::new(),
            nature,
            true_nature: None,
            gender,
            evs,
            ivs,
            level: 50,
            experience: 0,
            shiny: rng.random_bool(0.01),
            friendship: 255,
            ball: Some("pokeball".to_string()),
            hidden_power_type: None,
            different_original_trainer: false,
            dynamax_level,
            gigantamax_factor: false,
            tera_type,
            persistent_battle_data: Default::default(),
        });
    }

    Ok(TeamData {
        members,
        bag: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use battler::{
        Dex,
        config::Format,
        teams::TeamValidator,
    };

    use super::*;

    #[test]
    fn validates_random_battles() {
        let store = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&store).unwrap();

        for i in 0..50 {
            let options = generate_random_battle(&store, Some(i)).unwrap();
            options.validate().unwrap();

            let format = Format::new(options.format, &dex).unwrap();
            let validator = TeamValidator::new(&format, &dex);

            let mut team_1 = options.side_1.players[0].team.clone();
            let problems_1 = validator.validate_team(&mut team_1);
            assert!(
                problems_1.is_empty(),
                "Seed {} - Team 1 has validation problems: {:?}",
                i,
                problems_1
            );

            let mut team_2 = options.side_2.players[0].team.clone();
            let problems_2 = validator.validate_team(&mut team_2);
            assert!(
                problems_2.is_empty(),
                "Seed {} - Team 2 has validation problems: {:?}",
                i,
                problems_2
            );
        }
    }
}
