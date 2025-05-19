use std::sync::LazyLock;

use battler_data::{
    Fraction,
    MoveCategory,
    Stat,
    Type,
    TypeEffectiveness,
};
use indexmap::IndexMap;

use crate::{
    common::{
        Output,
        Range,
        RangeDistribution,
    },
    damage::{
        DamageContext,
        MonType,
    },
};

// Dynamic extensions to the damage calculator.
//
// If a function type takes in a

/// Modifies the battle state from an effect on the field.
pub(crate) type ModifyStateFromField = fn(&mut DamageContext);
/// Modifies the battle state from an effect on a side.
pub(crate) type ModifyStateFromSide = fn(&mut DamageContext, MonType);
/// Modifies the battle state from an effect on a Mon.
pub(crate) type ModifyStateFromMon = fn(&mut DamageContext, MonType);
/// Modifies the move being used.
pub(crate) type ModifyMove = fn(&mut DamageContext);
/// Applies fixed damage.
pub(crate) type ApplyFixedDamage = fn(&DamageContext) -> Option<Range<u64>>;
/// Modifies the move data.
pub(crate) type ModifyMoveData = fn(&mut DamageContext);
/// Modifies the move base power.
pub(crate) type ModifyBasePower = fn(&DamageContext, &mut Output<Fraction<u64>>);
/// Modifies a stat calculation.
pub(crate) type ModifyStat = fn(&DamageContext, MonType, &mut Output<Range<Fraction<u64>>>);
/// Modifies damage from weather (before critical hit and randomization).
pub(crate) type ModifyDamageFromWeather = fn(&DamageContext, &mut Output<Range<Fraction<u64>>>);
/// Modifies type effectiveness.
pub(crate) type ModifyTypeEffectiveness = fn(&DamageContext, &mut Output<Fraction<u64>>);
/// Modifies damage.
pub(crate) type ModifyDamage = fn(&DamageContext, &mut Output<RangeDistribution<Fraction<u64>>>);
/// Modifies the battle state after a hit.
pub(crate) type ModifyStateAfterHit = fn(&mut DamageContext);
/// Checks some Mon state.
pub(crate) type CheckMonState = fn(&DamageContext, MonType) -> Option<bool>;

macro_rules! type_powering_ability {
    ( $name:literal, $typ:expr ) => {
        (|context: &DamageContext, mon_type: MonType, value: &mut Output<Range<Fraction<u64>>>| {
            if context.move_data.primary_type == $typ
                && context
                    .mon(mon_type)
                    .health
                    .is_some_and(|health| health <= Fraction::new(1, 3))
            {
                value.mul(Fraction::new(3, 2), $name);
            }
        }) as _
    };
}

macro_rules! gem {
    ( $name:literal, $typ:expr ) => {
        (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
            if context.move_data.primary_type == $typ {
                base_power.mul(Fraction::new(13, 10), $name);
            }
        }) as _
    };
}

macro_rules! type_powering_item {
    ( $name:literal, $typ:expr ) => {
        (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
            if context.move_data.primary_type == $typ {
                base_power.mul(Fraction::new(6, 5), $name);
            }
        }) as _
    };
}

macro_rules! damage_reducing_berry {
    ( $name:literal, $typ:expr ) => {
        (|context: &DamageContext, damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
            if context.move_data.primary_type == $typ
                && context.properties.mov.type_effectiveness > 1
            {
                damage.div(2, $name);
                if context.defender.has_ability(["Ripen"]) {
                    damage.div(2, "Ripen");
                }
            }
        }) as _
    };
    ( $name:literal ) => {
        (|context: &DamageContext, damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
            if context.properties.mov.type_effectiveness > 1 {
                damage.div(2, $name);
                if context.defender.has_ability(["Ripen"]) {
                    damage.div(2, "Ripen");
                }
            }
        }) as _
    };
}

pub(crate) static MODIFY_STATE_FROM_MON_HOOKS: LazyLock<IndexMap<&str, ModifyStateFromMon>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "condition:Gastro Acid",
                (|context: &mut DamageContext, mon_type: MonType| {
                    context.mon_mut(mon_type).ability = None;
                }) as _,
            ),
            (
                "condition:Embargo",
                (|context: &mut DamageContext, mon_type: MonType| {
                    context.mon_mut(mon_type).item = None;
                }) as _,
            ),
            (
                "item:Utility Umbrella",
                (|context: &mut DamageContext, mon_type: MonType| {
                    if context.field.has_weather([
                        "Rain",
                        "Heavy Rain",
                        "Harsh Sunlight",
                        "Extremely Harsh Sunlight",
                    ]) {
                        context.mon_properties_mut(mon_type).weather_suppressed = true;
                    }
                }) as _,
            ),
        ])
    });

pub(crate) static MODIFY_STATE_FROM_SIDE_HOOKS: LazyLock<IndexMap<&str, ModifyStateFromSide>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "ability:Air Lock",
                (|context: &mut DamageContext, _: MonType| {
                    context.field.weather = None;
                }) as _,
            ),
            (
                "ability:Cloud Nine",
                (|context: &mut DamageContext, _: MonType| {
                    context.field.weather = None;
                }) as _,
            ),
        ])
    });

pub(crate) static MODIFY_STATE_FROM_FIELD_HOOKS: LazyLock<IndexMap<&str, ModifyStateFromField>> =
    LazyLock::new(|| IndexMap::from_iter([]));

pub(crate) static MODIFY_MOVE_HOOKS: LazyLock<IndexMap<&str, ModifyMove>> = LazyLock::new(|| {
    IndexMap::from_iter([(
        "move:Nature Power",
        (|context: &mut DamageContext| {
            let move_name = if context.field.has_terrain(["Electric Terrain"]) {
                "Thunderbolt"
            } else if context.field.has_terrain(["Grassy Terrain"]) {
                "Energy Ball"
            } else if context.field.has_terrain(["Misty Terrain"]) {
                "Moonblast"
            } else if context.field.has_terrain(["Psychic Terrain"]) {
                "Psychic Ball"
            } else if context.field.has_environment(["Cave"]) {
                "Power Gem"
            } else if context.field.has_environment(["Sand"]) {
                "Earth Power"
            } else if context.field.has_environment(["Water"]) {
                "Hydro Pump"
            } else if context.field.has_environment(["Ice"]) {
                "Ice Beam"
            } else if context.field.has_environment(["Sky"]) {
                "Air Slash"
            } else if context.field.has_environment(["Grass"]) {
                "Energy Ball"
            } else if context.field.has_environment(["Volcano"]) {
                "Lava Plume"
            } else {
                "Tri Attack"
            };
            context.mov.name = move_name.to_owned();
        }) as _,
    )])
});

pub(crate) static APPLY_FIXED_DAMAGE_HOOKS: LazyLock<IndexMap<&str, ApplyFixedDamage>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "move:Seismic Toss",
                (|context: &DamageContext| Some(context.attacker.level.into())) as _,
            ),
            (
                "move:Psywave",
                (|context: &DamageContext| {
                    Some(Range::new(
                        50 * context.attacker.level / 100,
                        150 * context.attacker.level / 100,
                    ))
                }) as _,
            ),
            (
                "move:Super Fang",
                (|context: &DamageContext| {
                    let hp = context
                        .calculate_stat(MonType::Defender, Stat::HP)
                        .value()
                        .map(|health| Fraction::from(health));
                    let health = hp * context.defender.health.unwrap_or(Fraction::from(1u64));
                    let damage = health / 2;
                    let damage = damage.map(|damage| damage.floor().max(1));
                    Some(damage)
                }) as _,
            ),
            (
                "move:Endeavor",
                (|context: &DamageContext| {
                    let attacker_hp = context
                        .calculate_stat(MonType::Attacker, Stat::HP)
                        .value()
                        .map(|health| Fraction::from(health));
                    let attacker_health =
                        attacker_hp * context.defender.health.unwrap_or(Fraction::from(1u64));
                    let defender_hp = context
                        .calculate_stat(MonType::Defender, Stat::HP)
                        .value()
                        .map(|health| Fraction::from(health));
                    let defender_health =
                        defender_hp * context.defender.health.unwrap_or(Fraction::from(1u64));
                    let damage = defender_health - attacker_health;
                    let damage = damage.map(|damage| damage.floor());
                    Some(damage)
                }) as _,
            ),
        ])
    });

pub(crate) static MODIFY_MOVE_DATA_HOOKS: LazyLock<IndexMap<&str, ModifyMoveData>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "move:Hidden Power",
                (|context: &mut DamageContext| {
                    if let Some(typ) = context.attacker.hidden_power_type {
                        context.move_data.primary_type = typ;
                    }
                }) as _,
            ),
            (
                "move:Weather Ball",
                (|context: &mut DamageContext| {
                    if context.properties.attacker.weather_suppressed {
                        return;
                    }

                    if context
                        .field
                        .has_weather(["Harsh Sunlight", "Extremely Harsh Sunlight"])
                    {
                        context.move_data.primary_type = Type::Fire;
                    } else if context.field.has_weather(["Rain", "Heavy Rain"]) {
                        context.move_data.primary_type = Type::Water;
                    } else if context.field.has_weather(["Hail", "Snow"]) {
                        context.move_data.primary_type = Type::Ice;
                    } else if context.field.has_weather(["Sandstorm"]) {
                        context.move_data.primary_type = Type::Rock;
                    }
                }) as _,
            ),
        ])
    });

pub(crate) static MODIFY_BASE_POWER_HOOKS: LazyLock<IndexMap<&str, ModifyBasePower>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "move:Low Kick",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    let weight = context.defender_species_data.weight;
                    if weight >= 2000 {
                        base_power.set(120u64, "weight >= 200 kg");
                    } else if weight >= 1000 {
                        base_power.set(100u64, "weight >= 100 kg")
                    } else if weight >= 500 {
                        base_power.set(80u64, "weight >= 50 kg")
                    } else if weight >= 250 {
                        base_power.set(60u64, "weight >= 25 kg")
                    } else if weight >= 100 {
                        base_power.set(40u64, "weight >= 10 kg")
                    } else {
                        base_power.set(20u64, "weight < 10 kg");
                    }
                }) as _,
            ),
            (
                "move:Solar Beam",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.field.has_weather([
                        "Rain",
                        "Heavy Rain",
                        "Sandstorm",
                        "Hail",
                        "Snow",
                    ]) {
                        base_power.div(2, "weak weather");
                    }
                }) as _,
            ),
            (
                "move:Triple Kick",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    base_power.mul(context.properties.mov.hit, "additional hit");
                }) as _,
            ),
            (
                "move:Flail",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    let health = context.defender.health.unwrap_or(Fraction::from(1u64));
                    if health >= Fraction::new(688, 1000) {
                        base_power.set(20u64, "hp >= 68.8%");
                    } else if health >= Fraction::new(354, 1000) {
                        base_power.set(40u64, "hp >= 35.4%");
                    } else if health >= Fraction::new(208, 1000) {
                        base_power.set(80u64, "hp >= 20.8%");
                    } else if health >= Fraction::new(104, 1000) {
                        base_power.set(100u64, "hp >= 10.4%");
                    } else if health >= Fraction::new(42, 1000) {
                        base_power.set(150u64, "hp >= 4.2%");
                    } else {
                        base_power.set(200u64, "hp < 4.2%");
                    }
                }) as _,
            ),
            (
                "move:Reversal",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    let health = context.defender.health.unwrap_or(Fraction::from(1u64));
                    if health >= Fraction::new(688, 1000) {
                        base_power.set(20u64, "hp >= 68.8%");
                    } else if health >= Fraction::new(354, 1000) {
                        base_power.set(40u64, "hp >= 35.4%");
                    } else if health >= Fraction::new(208, 1000) {
                        base_power.set(80u64, "hp >= 20.8%");
                    } else if health >= Fraction::new(104, 1000) {
                        base_power.set(100u64, "hp >= 10.4%");
                    } else if health >= Fraction::new(42, 1000) {
                        base_power.set(150u64, "hp >= 4.2%");
                    } else {
                        base_power.set(200u64, "hp < 4.2%");
                    }
                }) as _,
            ),
            (
                "move:Return",
                (|_: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    base_power.set(102u64, "max happiness");
                }) as _,
            ),
            (
                "move:Frustration",
                (|_: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    base_power.set(102u64, "min happiness");
                }) as _,
            ),
            (
                "move:Pursuit",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.defender.has_condition(["Switching"]) {
                        base_power.mul(2u64, "switching out");
                    }
                }) as _,
            ),
            (
                "move:Pursuit",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context
                        .defender
                        .has_status(["Poison", "Bad Poison", "Paralysis", "Burn"])
                    {
                        base_power.mul(2u64, "status boost");
                    }
                }) as _,
            ),
            (
                "move:Smelling Salts",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.defender.has_status(["Paralysis"]) {
                        base_power.mul(2u64, "paralysis boost");
                    }
                }) as _,
            ),
            (
                "move:Knock Off",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.defender.item.is_some() {
                        base_power.mul(Fraction::new(3, 2), "item boost");
                    }
                }) as _,
            ),
            (
                "move:Eruption",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    base_power.mul(
                        context.attacker.health.unwrap_or(Fraction::from(1u64)),
                        "user health",
                    );
                }) as _,
            ),
            (
                "move:Water Spout",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    base_power.mul(
                        context.attacker.health.unwrap_or(Fraction::from(1u64)),
                        "user health",
                    );
                }) as _,
            ),
            (
                "move:Weather Ball",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type != Type::Normal {
                        base_power.mul(
                            context.attacker.health.unwrap_or(Fraction::from(1u64)),
                            "weather boost",
                        );
                    }
                }) as _,
            ),
            (
                "condition:Mud Sport",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Electric {
                        base_power.div(3, "Mud Sport");
                    }
                }) as _,
            ),
            (
                "condition:Water Sport",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Fire {
                        base_power.div(3, "Water Sport");
                    }
                }) as _,
            ),
            (
                "terrain:Misty Terrain:defender",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Dragon {
                        base_power.div(2, "Misty Terrain");
                    }
                }) as _,
            ),
            (
                "terrain:Grassy Terrain:defender",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context
                        .mov
                        .is_named(["Earthquake", "Bulldoze", "Magnitude"])
                    {
                        base_power.div(2, "Grassy Terrain");
                    } else if context.move_data.primary_type == Type::Grass {
                        base_power.mul(Fraction::new(13, 10), "Grassy Terrain");
                    }
                }) as _,
            ),
            (
                "terrain:Electric Terrain:defender",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Electric {
                        base_power.mul(Fraction::new(13, 10), "Electric Terrain");
                    }
                }) as _,
            ),
            (
                "terrain:Psychic Terrain:defender",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Psychic {
                        base_power.mul(Fraction::new(13, 10), "Psychic Terrain");
                    }
                }) as _,
            ),
            (
                "condition:Charge:attacker",
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Electric {
                        base_power.mul(2u64, "Charge");
                    }
                }) as _,
            ),
            ("item:Fire Gem:attacker", gem!("Fire Gem", Type::Fire)),
            ("item:Water Gem:attacker", gem!("Water Gem", Type::Water)),
            (
                "item:Electric Gem:attacker",
                gem!("Electric Gem", Type::Electric),
            ),
            ("item:Grass Gem:attacker", gem!("Grass Gem", Type::Grass)),
            ("item:Ice Gem:attacker", gem!("Ice Gem", Type::Ice)),
            (
                "item:Fighting Gem:attacker",
                gem!("Fighting Gem", Type::Fighting),
            ),
            ("item:Poison Gem:attacker", gem!("Poison Gem", Type::Poison)),
            ("item:Ground Gem:attacker", gem!("Ground Gem", Type::Ground)),
            ("item:Flying Gem:attacker", gem!("Flying Gem", Type::Flying)),
            (
                "item:Psychic Gem:attacker",
                gem!("Psychic Gem", Type::Psychic),
            ),
            ("item:Bug Gem:attacker", gem!("Bug Gem", Type::Bug)),
            ("item:Rock Gem:attacker", gem!("Rock Gem", Type::Rock)),
            ("item:Ghost Gem:attacker", gem!("Ghost Gem", Type::Ghost)),
            ("item:Dark Gem:attacker", gem!("Dark Gem", Type::Dark)),
            ("item:Steel Gem:attacker", gem!("Steel Gem", Type::Steel)),
            ("item:Dragon Gem:attacker", gem!("Dragon Gem", Type::Dragon)),
            ("item:Normal Gem:attacker", gem!("Normal Gem", Type::Normal)),
            ("item:Fairy Gem:attacker", gem!("Fairy Gem", Type::Fairy)),
            (
                "item:Silver Powder:attacker",
                type_powering_item!("Silver Powder", Type::Bug),
            ),
            (
                "item:Metal Coat:attacker",
                type_powering_item!("Metal Coat", Type::Steel),
            ),
            (
                "item:Soft Sand:attacker",
                type_powering_item!("Soft Sand", Type::Ground),
            ),
            (
                "item:Hard Stone:attacker",
                type_powering_item!("Hard Stone", Type::Ground),
            ),
            (
                "item:Miracle Seed:attacker",
                type_powering_item!("Miracle Seed", Type::Grass),
            ),
            (
                "item:Black Glasses:attacker",
                type_powering_item!("Black Glasses", Type::Dark),
            ),
            (
                "item:Black Belt:attacker",
                type_powering_item!("Black Belt", Type::Fighting),
            ),
            (
                "item:Magnet:attacker",
                type_powering_item!("Magnet", Type::Electric),
            ),
            (
                "item:Mystic Water:attacker",
                type_powering_item!("Mystic Water", Type::Water),
            ),
            (
                "item:Sharp Beak:attacker",
                type_powering_item!("Sharp Beak", Type::Flying),
            ),
            (
                "item:Poison Barb:attacker",
                type_powering_item!("Poison Barb", Type::Poison),
            ),
            (
                "item:Never-Melt Ice:attacker",
                type_powering_item!("Never-Melt Ice", Type::Ice),
            ),
            (
                "item:Spell Tag:attacker",
                type_powering_item!("Spell Tag", Type::Ghost),
            ),
            (
                "item:Twisted Spoon:attacker",
                type_powering_item!("Twisted Spoon", Type::Psychic),
            ),
            (
                "item:Charcoal:attacker",
                type_powering_item!("Charcoal", Type::Fire),
            ),
            (
                "item:Dragon Fang:attacker",
                type_powering_item!("Dragon Fang", Type::Dragon),
            ),
            (
                "item:Silk Scarf:attacker",
                type_powering_item!("Silk Scarf", Type::Normal),
            ),
            (
                "item:Sea Incense:attacker",
                type_powering_item!("Sea Incense", Type::Water),
            ),
        ])
    });

pub(crate) static MODIFY_ATK_STAT_HOOKS: LazyLock<IndexMap<&str, ModifyStat>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "condition:Flash Fire",
                (|context: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    if context.move_data.primary_type == Type::Fire {
                        value.mul(2u64, "Flash Fire");
                    }
                }) as _,
            ),
            (
                "ability:Huge Power",
                (|_: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(2u64, "Huge Power");
                }) as _,
            ),
            (
                "ability:Pure Power",
                (|_: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(2u64, "Huge Power");
                }) as _,
            ),
            (
                "ability:Hustle",
                (|_: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(Fraction::new(3, 2), "Hustle");
                }) as _,
            ),
            (
                "ability:Guts",
                (|context: &DamageContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if context.mon(mon_type).status.is_some() {
                        value.mul(2u64, "Guts");
                    }
                }) as _,
            ),
            (
                "ability:Marvel Scale",
                (|context: &DamageContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if context.mon(mon_type).status.is_some() {
                        value.mul(2u64, "Guts");
                    }
                }) as _,
            ),
            (
                "ability:Overgrow",
                (|context: &DamageContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if context.move_data.primary_type == Type::Grass
                        && context
                            .mon(mon_type)
                            .health
                            .is_some_and(|health| health <= Fraction::new(1, 3))
                    {
                        value.mul(Fraction::new(3, 2), "Overgrow");
                    }
                }) as _,
            ),
            (
                "ability:Blaze",
                (|context: &DamageContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if context.move_data.primary_type == Type::Fire
                        && context
                            .mon(mon_type)
                            .health
                            .is_some_and(|health| health <= Fraction::new(1, 3))
                    {
                        value.mul(Fraction::new(3, 2), "Blaze");
                    }
                }) as _,
            ),
            (
                "ability:Overgrow",
                type_powering_ability!("Overgrow", Type::Water),
            ),
            (
                "ability:Blaze",
                type_powering_ability!("Blaze", Type::Water),
            ),
            (
                "ability:Torrent",
                type_powering_ability!("Torrent", Type::Water),
            ),
            ("ability:Swarm", type_powering_ability!("Swarm", Type::Bug)),
            (
                "ability:Thick Fat:opposite",
                (|context: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    if context.move_data.primary_type == Type::Ice
                        || context.move_data.primary_type == Type::Fire
                    {
                        value.mul(Fraction::new(1, 2), "Thick Fat");
                    }
                }) as _,
            ),
            (
                "item:Choice Band",
                (|_: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(Fraction::new(3, 2), "Choice Band");
                }) as _,
            ),
            (
                "item:Light Ball",
                (|context: &DamageContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if context.species(mon_type).base_species == "Pikachu" {
                        value.mul(2u64, "Light Ball");
                    }
                }) as _,
            ),
        ])
    });

pub(crate) static MODIFY_DEF_STAT_HOOKS: LazyLock<IndexMap<&str, ModifyStat>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "weather:Snow",
            (|context: &DamageContext,
              mon_type: MonType,
              value: &mut Output<Range<Fraction<u64>>>| {
                if context.mon(mon_type).has_type([Type::Ice]) {
                    value.mul(Fraction::new(3, 2), "Snow");
                }
            }) as _,
        )])
    });

pub(crate) static MODIFY_SPA_STAT_HOOKS: LazyLock<IndexMap<&str, ModifyStat>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "condition:Flash Fire",
                (|context: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    if context.move_data.primary_type == Type::Fire {
                        value.mul(2u64, "Flash Fire");
                    }
                }) as _,
            ),
            (
                "ability:Overgrow",
                type_powering_ability!("Overgrow", Type::Water),
            ),
            (
                "ability:Blaze",
                type_powering_ability!("Blaze", Type::Water),
            ),
            (
                "ability:Torrent",
                type_powering_ability!("Torrent", Type::Water),
            ),
            ("ability:Swarm", type_powering_ability!("Swarm", Type::Bug)),
            (
                "ability:Thick Fat:opposite",
                (|context: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    if context.move_data.primary_type == Type::Ice
                        || context.move_data.primary_type == Type::Fire
                    {
                        value.mul(Fraction::new(1, 2), "Thick Fat");
                    }
                }) as _,
            ),
            (
                "item:Choice Specs",
                (|_: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(Fraction::new(3, 2), "Choice Specs");
                }) as _,
            ),
            (
                "item:Light Ball",
                (|context: &DamageContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if context.species(mon_type).base_species == "Pikachu" {
                        value.mul(2u64, "Light Ball");
                    }
                }) as _,
            ),
        ])
    });

pub(crate) static MODIFY_SPD_STAT_HOOKS: LazyLock<IndexMap<&str, ModifyStat>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "weather:Sandstorm",
            (|context: &DamageContext,
              mon_type: MonType,
              value: &mut Output<Range<Fraction<u64>>>| {
                if context.mon(mon_type).has_type([Type::Rock]) {
                    value.mul(Fraction::new(3, 2), "Sandstorm");
                }
            }) as _,
        )])
    });

pub(crate) static MODIFY_SPE_STAT_HOOKS: LazyLock<IndexMap<&str, ModifyStat>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "status:Paralysis",
                (|context: &DamageContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if !context.mon(mon_type).has_ability(["Quick Feet"]) {
                        value.div(2, "Paralysis");
                    }
                }) as _,
            ),
            (
                "ability:Swift Swim",
                (|context: &DamageContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if context.mon_properties(mon_type).weather_suppressed {
                        return;
                    }
                    if context.field.has_weather(["Rain", "Heavy Rain"]) {
                        value.mul(2u64, "Swift Swim");
                    }
                }) as _,
            ),
            (
                "ability:Chlorophyll",
                (|context: &DamageContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if context.mon_properties(mon_type).weather_suppressed {
                        return;
                    }
                    if context
                        .field
                        .has_weather(["Harsh Sunlight", "Extremely Harsh Sunlight"])
                    {
                        value.mul(2u64, "Chlorophyll");
                    }
                }) as _,
            ),
            (
                "item:Macho Brace",
                (|_: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.div(2, "Macho Brace");
                }) as _,
            ),
            (
                "item:Choice Scarf",
                (|_: &DamageContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(Fraction::new(3, 2), "Choice Scarf");
                }) as _,
            ),
        ])
    });

pub(crate) static MODIFY_DAMAGE_FROM_WEATHER_HOOKS: LazyLock<
    IndexMap<&str, ModifyDamageFromWeather>,
> = LazyLock::new(|| {
    IndexMap::from_iter([(
        "weather:Rain:defender",
        (|context: &DamageContext, damage: &mut Output<Range<Fraction<u64>>>| {
            if context.move_data.primary_type == Type::Water {
                damage.mul(Fraction::new(3, 2), "Rain");
            }
            if context.move_data.primary_type == Type::Fire {
                damage.div(2, "Rain");
            }
        }) as _,
    )])
});

pub(crate) static MODIFY_TYPE_EFFECTIVENESS_HOOKS: LazyLock<
    IndexMap<&str, ModifyTypeEffectiveness>,
> = LazyLock::new(|| {
    IndexMap::from_iter([(
        "weather:Strong Winds:defender",
        (|context: &DamageContext, effectiveness: &mut Output<Fraction<u64>>| {
            if context.defender.has_type([Type::Flying])
                && context.type_effectiveness(context.move_data.primary_type, Type::Flying)
                    == TypeEffectiveness::Strong
            {
                effectiveness.div(2, "Strong Winds");
            }
        }) as _,
    )])
});

pub(crate) static MODIFY_DAMAGE_HOOKS: LazyLock<IndexMap<&str, ModifyDamage>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "status:Burn:attacker",
                (|context: &DamageContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.move_data.category == MoveCategory::Physical
                        && !context.attacker.has_ability(["Guts"])
                        && !context.mov.is_named(["Facade"])
                    {
                        damage.div(2, "Burn");
                    }
                }) as _,
            ),
            (
                "condition:Fly:defender",
                (|context: &DamageContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.mov.is_named(["Gust", "Twister"]) {
                        damage.mul(2u64, "Fly");
                    }
                }) as _,
            ),
            (
                "condition:Bounce:defender",
                (|context: &DamageContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.mov.is_named(["Gust", "Twister"]) {
                        damage.mul(2u64, "Fly");
                    }
                }) as _,
            ),
            (
                "condition:Dig:defender",
                (|context: &DamageContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.mov.is_named(["Earthquake", "Magnitude"]) {
                        damage.mul(2u64, "Dig");
                    }
                }) as _,
            ),
            (
                "condition:Dive:defender",
                (|context: &DamageContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.mov.is_named(["Surf", "Whirlpool"]) {
                        damage.mul(2u64, "Dive");
                    }
                }) as _,
            ),
            (
                "condition:Minimize:defender",
                (|context: &DamageContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.mov.is_named([
                        "Stomp",
                        "Steamroller",
                        "Body Slam",
                        "Flying Press",
                        "Dragon Rush",
                        "Heat Crash",
                        "Heavy Slam",
                    ]) {
                        damage.mul(2u64, "Minimize");
                    }
                }) as _,
            ),
            (
                "condition:Light Screen:defender",
                (|context: &DamageContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.move_data.category != MoveCategory::Special || context.mov.crit {
                        return;
                    }
                    if context.field.battle_type != "Singles" {
                        damage.mul(Fraction::new(2, 3), "Light Screen");
                    } else {
                        damage.div(2, "Light Screen");
                    }
                }) as _,
            ),
            (
                "condition:Reflect:defender",
                (|context: &DamageContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.move_data.category != MoveCategory::Physical || context.mov.crit {
                        return;
                    }
                    if context.field.battle_type != "Singles" {
                        damage.mul(Fraction::new(2, 3), "Reflect");
                    } else {
                        damage.div(2, "Reflect");
                    }
                }) as _,
            ),
            (
                "item:Occa Berry:defender",
                damage_reducing_berry!("Occa Berry", Type::Fire),
            ),
            (
                "item:Passho Berry:defender",
                damage_reducing_berry!("Passho Berry", Type::Water),
            ),
            (
                "item:Wacan Berry:defender",
                damage_reducing_berry!("Wacan Berry", Type::Electric),
            ),
            (
                "item:Rindo Berry:defender",
                damage_reducing_berry!("Rindo Berry", Type::Grass),
            ),
            (
                "item:Yache Berry:defender",
                damage_reducing_berry!("Yache Berry", Type::Ice),
            ),
            (
                "item:Chople Berry:defender",
                damage_reducing_berry!("Chople Berry", Type::Fighting),
            ),
            (
                "item:Kebia Berry:defender",
                damage_reducing_berry!("Kebia Berry", Type::Poison),
            ),
            (
                "item:Shuca Berry:defender",
                damage_reducing_berry!("Shuca Berry", Type::Ground),
            ),
            (
                "item:Coba Berry:defender",
                damage_reducing_berry!("Coba Berry", Type::Flying),
            ),
            (
                "item:Payapa Berry:defender",
                damage_reducing_berry!("Payapa Berry", Type::Psychic),
            ),
            (
                "item:Tanga Berry:defender",
                damage_reducing_berry!("Tanga Berry", Type::Bug),
            ),
            (
                "item:Charti Berry:defender",
                damage_reducing_berry!("Charti Berry", Type::Rock),
            ),
            (
                "item:Kasib Berry:defender",
                damage_reducing_berry!("Kasib Berry", Type::Ghost),
            ),
            (
                "item:Haban Berry:defender",
                damage_reducing_berry!("Haban Berry", Type::Dragon),
            ),
            (
                "item:Colbur Berry:defender",
                damage_reducing_berry!("Colbur Berry", Type::Dark),
            ),
            (
                "item:Babiri Berry:defender",
                damage_reducing_berry!("Babiri Berry", Type::Steel),
            ),
            (
                "item:Chilan Berry:defender",
                damage_reducing_berry!("Chilan Berry", Type::Normal),
            ),
            (
                "item:Roseli Berry:defender",
                damage_reducing_berry!("Roseli Berry", Type::Fairy),
            ),
            (
                "item:Enigma Berry:defender",
                damage_reducing_berry!("Enigma Berry"),
            ),
        ])
    });

pub(crate) static MODIFY_STATE_AFTER_HIT_HOOKS: LazyLock<IndexMap<&str, ModifyStateAfterHit>> =
    LazyLock::new(|| IndexMap::from_iter([]));

pub(crate) static MON_IS_GROUNDED_HOOKS: LazyLock<IndexMap<&str, CheckMonState>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "condition:Ingrain",
                (|_: &DamageContext, _: MonType| {
                    return Some(true);
                }) as _,
            ),
            (
                "ability:Levitate",
                (|_: &DamageContext, _: MonType| {
                    return Some(false);
                }) as _,
            ),
            (
                "mon",
                (|context: &DamageContext, mon_type: MonType| {
                    if context.mon(mon_type).has_type([Type::Flying]) {
                        return Some(false);
                    }
                    None
                }) as _,
            ),
        ])
    });

pub(crate) static MON_NEGATES_IMMUNITY_HOOKS: LazyLock<IndexMap<&str, CheckMonState>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "condition:Miracle Eye",
                (|context: &DamageContext, mon_type: MonType| {
                    if !context.mon(mon_type).has_type([Type::Dark]) {
                        return None;
                    }
                    if context.move_data.primary_type == Type::Psychic {
                        return Some(true);
                    }
                    return None;
                }) as _,
            ),
            (
                "condition:Foresight",
                (|context: &DamageContext, mon_type: MonType| {
                    if !context.mon(mon_type).has_type([Type::Ghost]) {
                        return None;
                    }
                    let move_type = context.move_data.primary_type;
                    if move_type == Type::Normal || move_type == Type::Fighting {
                        return Some(true);
                    }
                    return None;
                }) as _,
            ),
            (
                "mon",
                (|context: &DamageContext, mon_type: MonType| {
                    if context.move_data.primary_type == Type::Ground
                        && context.mon_is_grounded(mon_type)
                    {
                        return Some(true);
                    }
                    None
                }) as _,
            ),
        ])
    });

pub(crate) static MON_IS_IMMUNE_HOOKS: LazyLock<IndexMap<&str, CheckMonState>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "mon",
            (|context: &DamageContext, mon_type: MonType| {
                if context.move_data.primary_type == Type::Ground
                    && !context.mon_is_grounded(mon_type)
                {
                    return Some(true);
                }
                None
            }) as _,
        )])
    });
