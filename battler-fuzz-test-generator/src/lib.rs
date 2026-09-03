use std::collections::{
    HashMap,
    HashSet,
};

use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleOptions,
    DataStore,
    FieldData,
    FormatData,
    Gender,
    Nature,
    PlayerData,
    PlayerDex,
    PlayerOptions,
    PlayerType,
    SideData,
    Stat,
    StatTable,
    TeamData,
    Type,
    teams::MonData,
};
use battler_data::{
    ItemData,
    ItemFlag,
    ZCrystalSource,
};
use rand::prelude::*;

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

pub struct ItemPools {
    pub items_pool: Vec<String>,
    pub megastones_map: HashMap<String, String>,
    pub type_to_zcrystal: HashMap<Type, String>,
}

fn is_battle_held_item(item: &ItemData) -> bool {
    // Player items (balls, medicine, battle consumables) are not held battle items.
    if item.flags.contains(&ItemFlag::Ball)
        || item.flags.contains(&ItemFlag::Medicine)
        || item.flags.contains(&ItemFlag::Battle)
    {
        return false;
    }

    let has_effect_code = |val: &serde_json::Value| {
        if let Some(obj) = val.as_object() {
            if obj.contains_key("delegates") {
                return true;
            }
            if let Some(callbacks) = obj.get("callbacks").and_then(|c| c.as_object()) {
                let keys: Vec<_> = callbacks.keys().map(|s| s.as_str()).collect();
                if !keys.is_empty()
                    && keys
                        .iter()
                        .all(|&k| k == "on_player_use" || k == "on_player_try_use_item")
                {
                    return false;
                }
                return !keys.is_empty();
            }
        }
        false
    };

    has_effect_code(&item.effect)
        || has_effect_code(&item.condition)
        || item.special_data.judgment.is_some()
        || item.special_data.multi_attack.is_some()
}

pub fn extract_item_pools(store: &dyn DataStore) -> Result<ItemPools> {
    let mut items_pool = Vec::new();
    let mut megastones_map = HashMap::new();
    let mut type_to_zcrystal = HashMap::new();

    let all_item_ids = store.all_item_ids(&|_| true)?;
    for id in all_item_ids {
        if let Some(item) = store.get_item(&id)? {
            // Ensure the item name resolves back to this item ID.
            if battler::Id::from(item.name.as_str()) != id {
                continue;
            }

            let name_str = item.name.clone();
            let mut is_special = false;
            if let Some(mega) = &item.special_data.mega_evolution {
                megastones_map.insert(name_str.clone(), mega.from.clone());
                is_special = true;
            }
            if let Some(z) = &item.special_data.z_crystal {
                if let Some(ZCrystalSource::Type(typ)) = &z.source {
                    type_to_zcrystal.insert(*typ, name_str.clone());
                }
                is_special = true;
            }
            if !is_special && is_battle_held_item(&item) {
                items_pool.push(name_str);
            }
        }
    }

    Ok(ItemPools {
        items_pool,
        megastones_map,
        type_to_zcrystal,
    })
}

pub struct FullRandomPools {
    pub base_species: Vec<battler::Id>,
    pub abilities: Vec<String>,
    pub moves: Vec<String>,
    pub item_pools: ItemPools,
}

pub fn extract_full_random_pools(store: &dyn DataStore) -> Result<FullRandomPools> {
    let item_pools = extract_item_pools(store)?;

    let base_species = store.all_species_ids(&|s| s.forme.is_none() && !s.battle_only_forme)?;

    let mut abilities = Vec::new();
    for id in store.all_ability_ids(&|_| true)? {
        if let Some(ability) = store.get_ability(&id)? {
            if battler::Id::from(ability.name.as_str()) == id {
                abilities.push(ability.name);
            }
        }
    }

    let mut moves = Vec::new();
    for id in store.all_move_ids(&|_| true)? {
        if let Some(mov) = store.get_move(&id)? {
            if battler::Id::from(mov.name.as_str()) == id {
                moves.push(mov.name);
            }
        }
    }

    Ok(FullRandomPools {
        base_species,
        abilities,
        moves,
        item_pools,
    })
}

/// Generates an unconstrained "True Chaos" battle with completely randomized teams and stackable
/// mechanics.
pub fn generate_full_random_battle(
    store: &dyn DataStore,
    battle_type: BattleType,
    team_size: usize,
    seed: Option<u64>,
) -> Result<CoreBattleOptions> {
    let actual_seed = seed.unwrap_or_else(|| rand::rng().random());
    let mut rng = StdRng::seed_from_u64(actual_seed);

    let pools = extract_full_random_pools(store)?;

    let mut rules = Vec::new();
    let enable_mega = rng.random_bool(0.5);
    let enable_z_moves = rng.random_bool(0.5);
    let enable_dynamax = rng.random_bool(0.5);
    let enable_tera = rng.random_bool(0.5);

    if enable_mega {
        rules.push("Mega Evolution".to_owned());
    }
    if enable_z_moves {
        rules.push("Z-Moves".to_owned());
    }
    if enable_dynamax {
        rules.push("Dynamax".to_owned());
    }
    if enable_tera {
        rules.push("Terastallization".to_owned());
    }

    let format = FormatData { battle_type, rules };

    let side_1_team = generate_full_random_team(
        store,
        &mut rng,
        team_size,
        &pools,
        enable_mega,
        enable_z_moves,
        enable_dynamax,
        enable_tera,
    )?;

    let side_2_team = generate_full_random_team(
        store,
        &mut rng,
        team_size,
        &pools,
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

pub fn generate_full_random_team(
    store: &dyn DataStore,
    rng: &mut StdRng,
    team_size: usize,
    pools: &FullRandomPools,
    enable_mega: bool,
    enable_z_moves: bool,
    enable_dynamax: bool,
    enable_tera: bool,
) -> Result<TeamData> {
    let mut members = Vec::new();

    let mega_index = if enable_mega && team_size > 0 {
        Some(rng.random_range(0..team_size))
    } else {
        None
    };

    let z_index = if enable_z_moves {
        let candidates: Vec<usize> = (0..team_size).filter(|&i| Some(i) != mega_index).collect();
        if !candidates.is_empty() {
            Some(*candidates.choose(rng).unwrap())
        } else {
            None
        }
    } else {
        None
    };

    for i in 0..team_size {
        let (_species_id, species_data, mut held_item) = if Some(i) == mega_index {
            if let Some((stone, base_species_name)) =
                pools.item_pools.megastones_map.iter().choose(rng)
            {
                let base_species_id = battler::Id::from(base_species_name.as_str());
                let s_data = store
                    .get_species(&base_species_id)?
                    .ok_or_else(|| anyhow::anyhow!("Species not found: {base_species_id}"))?;
                (base_species_id, s_data, Some(stone.clone()))
            } else {
                let s_id = pools
                    .base_species
                    .choose(rng)
                    .ok_or_else(|| anyhow::anyhow!("No base species available"))?
                    .clone();
                let s_data = store
                    .get_species(&s_id)?
                    .ok_or_else(|| anyhow::anyhow!("Species not found: {s_id}"))?;
                (s_id, s_data, None)
            }
        } else {
            let s_id = pools
                .base_species
                .choose(rng)
                .ok_or_else(|| anyhow::anyhow!("No base species available"))?
                .clone();
            let s_data = store
                .get_species(&s_id)?
                .ok_or_else(|| anyhow::anyhow!("Species not found: {s_id}"))?;
            (s_id, s_data, None)
        };
        let species_name = species_data.name.clone();

        // Random ability from ALL abilities.
        let ability = pools
            .abilities
            .choose(rng)
            .cloned()
            .unwrap_or_else(|| "No Ability".to_string());

        // Random 4 moves from ALL moves.
        let num_moves = std::cmp::min(4, pools.moves.len());
        let selected_moves: Vec<String> = pools.moves.sample(rng, num_moves).cloned().collect();

        // Held item assignment.
        if held_item.is_none() {
            if Some(i) == z_index {
                let mut move_types: Vec<Type> = Vec::new();
                for move_name in &selected_moves {
                    let move_id = battler::Id::from(move_name.as_str());
                    if let Some(m) = store.get_move(&move_id)? {
                        move_types.push(m.primary_type);
                    }
                }
                if let Some(typ) = move_types.choose(rng) {
                    if let Some(z_crystal) = pools.item_pools.type_to_zcrystal.get(typ) {
                        held_item = Some(z_crystal.clone());
                    }
                }
                if held_item.is_none() {
                    if let Some(z_crystal) = pools.item_pools.type_to_zcrystal.values().choose(rng)
                    {
                        held_item = Some(z_crystal.clone());
                    }
                }
            } else if let Some(item) = pools.item_pools.items_pool.choose(rng) {
                held_item = Some(item.clone());
            }
        }

        let nature = *ALL_NATURES.choose(rng).unwrap();

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

        let dynamax_level = if enable_dynamax { 10 } else { 0 };

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

/// Generates a valid, random battle configuration with randomized teams.
pub fn generate_random_battle(
    store: &dyn DataStore,
    battle_type: BattleType,
    team_size: usize,
    seed: Option<u64>,
) -> Result<CoreBattleOptions> {
    let actual_seed = seed.unwrap_or_else(|| rand::rng().random());
    let mut rng = StdRng::seed_from_u64(actual_seed);

    let item_pools = extract_item_pools(store)?;

    // 2. Select format rules and mechanics.
    let mut rules = Vec::new();
    rules.push("Species Clause".to_owned());
    rules.push("Item Clause".to_owned());

    let mut enable_mega = false;
    let mut enable_z_moves = false;
    let mut enable_dynamax = false;
    let mut enable_tera = false;

    // Pick one mechanic to enable, or none.
    // 0: None, 1: Mega Evolution, 2: Z-Moves, 3: Dynamax, 4: Terastallization
    match rng.random_range(0..5) {
        1 => {
            enable_mega = true;
            rules.push("Mega Evolution".to_owned());
        }
        2 => {
            enable_z_moves = true;
            rules.push("Z-Moves".to_owned());
        }
        3 => {
            enable_dynamax = true;
            rules.push("Dynamax".to_owned());
        }
        4 => {
            enable_tera = true;
            rules.push("Terastallization".to_owned());
        }
        _ => {}
    }

    let format = FormatData { battle_type, rules };

    // 3. Generate side teams.
    let side_1_team = generate_random_team(
        store,
        &mut rng,
        team_size,
        &item_pools.items_pool,
        &item_pools.megastones_map,
        &item_pools.type_to_zcrystal,
        enable_mega,
        enable_z_moves,
        enable_dynamax,
        enable_tera,
    )?;

    let side_2_team = generate_random_team(
        store,
        &mut rng,
        team_size,
        &item_pools.items_pool,
        &item_pools.megastones_map,
        &item_pools.type_to_zcrystal,
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

pub fn generate_random_team(
    store: &dyn DataStore,
    rng: &mut StdRng,
    team_size: usize,
    items_pool: &[String],
    megastones_map: &HashMap<String, String>,
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
    let mega_index = if enable_mega && team_size > 0 {
        Some(rng.random_range(0..team_size))
    } else {
        None
    };

    // Determine if team gets a Z-Crystal (if Z-Moves is enabled).
    // Can only be held by a non-Mega Mon.
    let z_index = if enable_z_moves {
        let candidates: Vec<usize> = (0..team_size).filter(|&i| Some(i) != mega_index).collect();
        if !candidates.is_empty() {
            Some(*candidates.choose(rng).unwrap())
        } else {
            None
        }
    } else {
        None
    };

    // Candidate list of base species (forme is None).
    let base_species_pool = store.all_species_ids(&|s| {
        s.forme.is_none() && !s.battle_only_forme && !s.learnset.is_empty()
    })?;

    for i in 0..team_size {
        let mut selected_species_id: Option<battler::Id> = None;
        let mut selected_species_data: Option<battler_data::SpeciesData> = None;
        let mut held_item = None;

        if Some(i) == mega_index {
            // Force Mega species and Mega Stone.
            let mut megastone_choices: Vec<(&String, &String)> = megastones_map.iter().collect();
            megastone_choices.shuffle(rng);

            for (stone_id, from_species_name) in megastone_choices {
                let from_species_id = battler::Id::from(from_species_name.as_str());
                if let Some(species_data) = store.get_species(&from_species_id)? {
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

        // If not Mega, or if Mega selection failed, pick a random base species.
        if selected_species_id.is_none() {
            let mut pool = base_species_pool.clone();
            pool.shuffle(rng);
            for id in pool {
                if !chosen_species.contains(&id) {
                    if let Some(s) = store.get_species(&id)? {
                        selected_species_id = Some(id);
                        selected_species_data = Some(s);
                        break;
                    }
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

        let mut learnset_moves: Vec<&String> = Vec::new();
        for (move_name, sources) in &species_data.learnset {
            let move_id = battler::Id::from(move_name.as_str());
            if store.get_move(&move_id)?.is_none() {
                continue;
            }
            if sources.iter().any(|source| match source {
                battler_data::MoveSource::Level(l) => *l <= 50,
                _ => true,
            }) {
                learnset_moves.push(move_name);
            }
        }
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
                let mut move_types: Vec<Type> = Vec::new();
                for move_name in &selected_moves {
                    let move_id = battler::Id::from(move_name.as_str());
                    if let Some(m) = store.get_move(&move_id)? {
                        move_types.push(m.primary_type);
                    }
                }
                move_types.shuffle(rng);

                for typ in move_types {
                    if let Some(z_crystal) = type_to_zcrystal.get(&typ) {
                        if !chosen_items.contains(z_crystal) {
                            held_item = Some(z_crystal.clone());
                            break;
                        }
                    }
                }

                if held_item.is_none() {
                    let mut crystals: Vec<&String> = type_to_zcrystal.values().collect();
                    crystals.shuffle(rng);
                    for z_crystal in crystals {
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
    use battler_local_data::LocalDataStore;

    use super::*;

    #[test]
    fn validates_random_battles() {
        let store = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&store).unwrap();

        for i in 0..50 {
            let options = generate_random_battle(&store, BattleType::Doubles, 4, Some(i)).unwrap();
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

    #[test]
    fn validates_different_battle_types_and_sizes() {
        let store = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&store).unwrap();

        let configurations = [
            (BattleType::Singles, 3),
            (BattleType::Singles, 6),
            (BattleType::Doubles, 4),
            (BattleType::Doubles, 6),
        ];

        for (battle_type, team_size) in configurations {
            for i in 0..10 {
                let options =
                    generate_random_battle(&store, battle_type, team_size, Some(i)).unwrap();
                options.validate().unwrap();

                assert_eq!(options.format.battle_type, battle_type);
                assert_eq!(options.side_1.players[0].team.members.len(), team_size);
                assert_eq!(options.side_2.players[0].team.members.len(), team_size);

                let format = Format::new(options.format, &dex).unwrap();
                let validator = TeamValidator::new(&format, &dex);

                let mut team_1 = options.side_1.players[0].team.clone();
                let problems_1 = validator.validate_team(&mut team_1);
                assert!(
                    problems_1.is_empty(),
                    "BattleType {:?} TeamSize {} Seed {} - Team 1 problems: {:?}",
                    battle_type,
                    team_size,
                    i,
                    problems_1
                );

                let mut team_2 = options.side_2.players[0].team.clone();
                let problems_2 = validator.validate_team(&mut team_2);
                assert!(
                    problems_2.is_empty(),
                    "BattleType {:?} TeamSize {} Seed {} - Team 2 problems: {:?}",
                    battle_type,
                    team_size,
                    i,
                    problems_2
                );
            }
        }
    }

    #[test]
    fn generates_mega_stone_when_mega_enabled() {
        let store = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&store).unwrap();
        let item_pools = extract_item_pools(&store).unwrap();

        let mut format_data = FormatData::default();
        format_data.rules.push("Species Clause".to_string());
        format_data.rules.push("Item Clause".to_string());
        format_data.rules.push("Mega Evolution".to_string());
        let format = Format::new(format_data, &dex).unwrap();
        let validator = TeamValidator::new(&format, &dex);

        for seed in 0..20 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut team = generate_random_team(
                &store,
                &mut rng,
                6,
                &item_pools.items_pool,
                &item_pools.megastones_map,
                &item_pools.type_to_zcrystal,
                true,  // enable_mega
                false, // enable_z_moves
                false, // enable_dynamax
                false, // enable_tera
            )
            .unwrap();

            // Validate that the team is legal according to the engine.
            let problems = validator.validate_team(&mut team);
            assert!(
                problems.is_empty(),
                "Seed {} team validation failed: {:?}",
                seed,
                problems
            );

            // Exactly 1 mon should hold a Mega Stone matching its species.
            let mega_mons: Vec<_> = team
                .members
                .iter()
                .filter(|m| {
                    if let Some(ref item) = m.item {
                        item_pools.megastones_map.contains_key(item)
                    } else {
                        false
                    }
                })
                .collect();

            assert_eq!(
                mega_mons.len(),
                1,
                "Seed {}: expected exactly 1 Mega Mon, found {}",
                seed,
                mega_mons.len()
            );

            let mega_mon = mega_mons[0];
            let held_stone = mega_mon.item.as_ref().unwrap();
            let required_species = item_pools.megastones_map.get(held_stone).unwrap();
            assert_eq!(
                &mega_mon.species, required_species,
                "Seed {}: Mega Mon species {} did not match required species {} for stone {}",
                seed, mega_mon.species, required_species, held_stone
            );

            // No mon should hold a Z-Crystal.
            let z_crystals: Vec<_> = team
                .members
                .iter()
                .filter(|m| {
                    if let Some(ref item) = m.item {
                        item_pools.type_to_zcrystal.values().any(|z| z == item)
                    } else {
                        false
                    }
                })
                .collect();
            assert!(
                z_crystals.is_empty(),
                "Seed {}: expected no Z-Crystals, found {:?}",
                seed,
                z_crystals
            );
        }
    }

    #[test]
    fn generates_z_crystal_when_z_moves_enabled() {
        let store = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&store).unwrap();
        let item_pools = extract_item_pools(&store).unwrap();

        let mut format_data = FormatData::default();
        format_data.rules.push("Species Clause".to_string());
        format_data.rules.push("Item Clause".to_string());
        format_data.rules.push("Z-Moves".to_string());
        let format = Format::new(format_data, &dex).unwrap();
        let validator = TeamValidator::new(&format, &dex);

        for seed in 0..20 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut team = generate_random_team(
                &store,
                &mut rng,
                6,
                &item_pools.items_pool,
                &item_pools.megastones_map,
                &item_pools.type_to_zcrystal,
                false, // enable_mega
                true,  // enable_z_moves
                false, // enable_dynamax
                false, // enable_tera
            )
            .unwrap();

            let problems = validator.validate_team(&mut team);
            assert!(
                problems.is_empty(),
                "Seed {} team validation failed: {:?}",
                seed,
                problems
            );

            // Exactly 1 mon should hold a Z-Crystal.
            let z_mons: Vec<_> = team
                .members
                .iter()
                .filter(|m| {
                    if let Some(ref item) = m.item {
                        item_pools.type_to_zcrystal.values().any(|z| z == item)
                    } else {
                        false
                    }
                })
                .collect();

            assert_eq!(
                z_mons.len(),
                1,
                "Seed {}: expected exactly 1 Z-Crystal Mon, found {}",
                seed,
                z_mons.len()
            );

            // No mon should hold a Mega Stone.
            let mega_mons: Vec<_> = team
                .members
                .iter()
                .filter(|m| {
                    if let Some(ref item) = m.item {
                        item_pools.megastones_map.contains_key(item)
                    } else {
                        false
                    }
                })
                .collect();
            assert!(
                mega_mons.is_empty(),
                "Seed {}: expected no Mega Stones, found {:?}",
                seed,
                mega_mons
            );
        }
    }

    #[test]
    fn generates_both_mega_and_z_crystal_when_both_enabled() {
        let store = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&store).unwrap();
        let item_pools = extract_item_pools(&store).unwrap();

        let mut format_data = FormatData::default();
        format_data.rules.push("Species Clause".to_string());
        format_data.rules.push("Item Clause".to_string());
        format_data.rules.push("Mega Evolution".to_string());
        format_data.rules.push("Z-Moves".to_string());
        let format = Format::new(format_data, &dex).unwrap();
        let validator = TeamValidator::new(&format, &dex);

        for seed in 0..20 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut team = generate_random_team(
                &store,
                &mut rng,
                6,
                &item_pools.items_pool,
                &item_pools.megastones_map,
                &item_pools.type_to_zcrystal,
                true,  // enable_mega
                true,  // enable_z_moves
                false, // enable_dynamax
                false, // enable_tera
            )
            .unwrap();

            let problems = validator.validate_team(&mut team);
            assert!(
                problems.is_empty(),
                "Seed {} team validation failed: {:?}",
                seed,
                problems
            );

            let mega_mons: Vec<_> = team
                .members
                .iter()
                .filter(|m| {
                    if let Some(ref item) = m.item {
                        item_pools.megastones_map.contains_key(item)
                    } else {
                        false
                    }
                })
                .collect();
            assert_eq!(
                mega_mons.len(),
                1,
                "Seed {}: expected 1 Mega Mon, found {}",
                seed,
                mega_mons.len()
            );

            let z_mons: Vec<_> = team
                .members
                .iter()
                .filter(|m| {
                    if let Some(ref item) = m.item {
                        item_pools.type_to_zcrystal.values().any(|z| z == item)
                    } else {
                        false
                    }
                })
                .collect();
            assert_eq!(
                z_mons.len(),
                1,
                "Seed {}: expected 1 Z-Move Mon, found {}",
                seed,
                z_mons.len()
            );

            // Ensure the Mega Mon and Z-Move Mon are different team members.
            assert_ne!(
                mega_mons[0].species, z_mons[0].species,
                "Seed {}: Mega and Z-Crystal assigned to same mon: {}",
                seed, mega_mons[0].species
            );
        }
    }

    #[test]
    fn does_not_generate_special_items_when_disabled() {
        let store = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let dex = Dex::new(&store).unwrap();
        let item_pools = extract_item_pools(&store).unwrap();

        let mut format_data = FormatData::default();
        format_data.rules.push("Species Clause".to_string());
        format_data.rules.push("Item Clause".to_string());
        let format = Format::new(format_data, &dex).unwrap();
        let validator = TeamValidator::new(&format, &dex);

        for seed in 0..20 {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut team = generate_random_team(
                &store,
                &mut rng,
                6,
                &item_pools.items_pool,
                &item_pools.megastones_map,
                &item_pools.type_to_zcrystal,
                false, // enable_mega
                false, // enable_z_moves
                false, // enable_dynamax
                false, // enable_tera
            )
            .unwrap();

            let problems = validator.validate_team(&mut team);
            assert!(
                problems.is_empty(),
                "Seed {} team validation failed: {:?}",
                seed,
                problems
            );

            let special_items: Vec<_> = team
                .members
                .iter()
                .filter(|m| {
                    if let Some(ref item) = m.item {
                        item_pools.megastones_map.contains_key(item)
                            || item_pools.type_to_zcrystal.values().any(|z| z == item)
                    } else {
                        false
                    }
                })
                .collect();
            assert!(
                special_items.is_empty(),
                "Seed {}: expected no special items when disabled, found {:?}",
                seed,
                special_items
            );
        }
    }

    #[test]
    fn item_pools_filters_out_non_battle_items() {
        let store = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let item_pools = extract_item_pools(&store).unwrap();

        // Ensure key battle items are present.
        assert!(item_pools.items_pool.contains(&"Leftovers".to_string()));
        assert!(item_pools.items_pool.contains(&"Choice Band".to_string()));
        assert!(item_pools.items_pool.contains(&"Choice Scarf".to_string()));
        assert!(item_pools.items_pool.contains(&"Focus Sash".to_string()));
        assert!(item_pools.items_pool.contains(&"Life Orb".to_string()));
        assert!(item_pools.items_pool.contains(&"Flame Orb".to_string()));
        assert!(item_pools.items_pool.contains(&"Toxic Orb".to_string()));
        assert!(item_pools.items_pool.contains(&"Iron Plate".to_string()));
        assert!(item_pools.items_pool.contains(&"Sitrus Berry".to_string()));
        assert!(item_pools.items_pool.contains(&"Lum Berry".to_string()));
        assert!(item_pools.items_pool.contains(&"Fire Gem".to_string()));

        // Ensure non-battle and fluff items are filtered out.
        assert!(!item_pools.items_pool.contains(&"Poké Ball".to_string()));
        assert!(!item_pools.items_pool.contains(&"Potion".to_string()));
        assert!(!item_pools.items_pool.contains(&"Guard Spec.".to_string()));
        assert!(!item_pools.items_pool.contains(&"Poké Doll".to_string()));
        assert!(!item_pools.items_pool.contains(&"Fluffy Tail".to_string()));
        assert!(!item_pools.items_pool.contains(&"TR01".to_string()));
        assert!(!item_pools.items_pool.contains(&"Exp. Candy XL".to_string()));
        assert!(!item_pools.items_pool.contains(&"Armorite Ore".to_string()));
        assert!(
            !item_pools
                .items_pool
                .contains(&"Tera Shard Ground".to_string())
        );

        // Megastones and Z-Crystals should not be in items_pool either.
        for mega_stone in item_pools.megastones_map.keys() {
            assert!(
                !item_pools.items_pool.contains(mega_stone),
                "items_pool should not contain Mega Stone {}",
                mega_stone
            );
        }
        for z_crystal in item_pools.type_to_zcrystal.values() {
            assert!(
                !item_pools.items_pool.contains(z_crystal),
                "items_pool should not contain Z-Crystal {}",
                z_crystal
            );
        }
    }

    #[test]
    fn generates_full_random_battles() {
        let store = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        for i in 0..10 {
            let options =
                generate_full_random_battle(&store, BattleType::Singles, 6, Some(i)).unwrap();
            options.validate().unwrap();

            assert_eq!(options.format.battle_type, BattleType::Singles);
            assert_eq!(options.side_1.players[0].team.members.len(), 6);
            assert_eq!(options.side_2.players[0].team.members.len(), 6);
            assert!(!options.format.rules.contains(&"Species Clause".to_string()));
            assert!(!options.format.rules.contains(&"Item Clause".to_string()));

            for player in &options.side_1.players {
                for mon in &player.team.members {
                    assert!(!mon.species.is_empty());
                    assert!(!mon.ability.is_empty());
                    assert_eq!(mon.moves.len(), 4);
                }
            }
        }
    }
}
