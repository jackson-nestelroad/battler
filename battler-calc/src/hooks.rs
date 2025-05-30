use std::sync::LazyLock;

use battler_data::{
    Fraction,
    MoveCategory,
    MoveFlag,
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
    simulate::{
        Hit,
        MonType,
        MoveContext,
        StatusEffect,
    },
};

// Dynamic extensions to the damage calculator.
//
// If a function type takes in a

/// Modifies the battle state from an effect on the field.
pub(crate) type ModifyStateFromField = fn(&mut MoveContext);
/// Modifies the battle state from an effect on a side.
pub(crate) type ModifyStateFromSide = fn(&mut MoveContext, MonType);
/// Modifies the battle state from an effect on a Mon.
pub(crate) type ModifyStateFromMon = fn(&mut MoveContext, MonType);
/// Modifies the move being used.
pub(crate) type ModifyMove = fn(&mut MoveContext);
/// Fails the move before it hits.
pub(crate) type FailMoveBeforeHit = fn(&mut MoveContext, &mut Hit) -> bool;
/// Applies fixed damage.
pub(crate) type ApplyFixedDamage = fn(&MoveContext) -> Option<Range<u64>>;
/// Modifies the move data.
pub(crate) type ModifyMoveData = fn(&mut MoveContext);
/// Modifies the move base power.
pub(crate) type ModifyBasePower = fn(&MoveContext, &mut Output<Fraction<u64>>);
/// Modifies a stat calculation.
pub(crate) type ModifyStat = fn(&MoveContext, MonType, &mut Output<Range<Fraction<u64>>>);
/// Modifies damage from weather (before critical hit and randomization).
pub(crate) type ModifyDamageFromWeather = fn(&MoveContext, &mut Output<Range<Fraction<u64>>>);
/// Modifies type effectiveness.
pub(crate) type ModifyTypeEffectiveness = fn(&MoveContext, &mut Output<Fraction<u64>>);
/// Modifies damage.
pub(crate) type ModifyDamage = fn(&mut MoveContext, &mut Output<RangeDistribution<Fraction<u64>>>);
/// Modifies a heal amount.
pub(crate) type ModifyDirectDamage =
    fn(&mut MoveContext, MonType, &mut Output<Range<Fraction<u64>>>);
/// Modifies the status effect.
pub(crate) type ModifyStatusEffect = fn(&mut MoveContext, MonType, &mut StatusEffect);
/// Modifies the battle state after a hit.
pub(crate) type ModifyStateAfterHit = fn(&mut MoveContext);
/// Checks some Mon state.
pub(crate) type CheckMonState = fn(&MoveContext, MonType) -> Option<bool>;

macro_rules! type_powering_ability {
    ( $name:literal, $typ:expr ) => {
        (|context: &MoveContext, mon_type: MonType, value: &mut Output<Range<Fraction<u64>>>| {
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
        (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
            if context.move_data.primary_type == $typ {
                base_power.mul(Fraction::new(13, 10), $name);
            }
        }) as _
    };
}

macro_rules! type_powering_item {
    ( $name:literal, $typ:expr ) => {
        (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
            if context.move_data.primary_type == $typ {
                base_power.mul(Fraction::new(6, 5), $name);
            }
        }) as _
    };
}

macro_rules! damage_reducing_berry {
    ( $name:literal, $typ:expr ) => {
        (|context: &mut MoveContext, damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
            if context.move_data.primary_type == $typ
                && context.properties.mov.type_effectiveness > 1
            {
                damage.mul(Fraction::new(1u64, 2u64), $name);
                if context.defender.has_ability(["Ripen"]) {
                    damage.mul(Fraction::new(1u64, 2u64), "Ripen");
                }
                context.defender.item = None;
            }
        }) as _
    };
    ( $name:literal ) => {
        (|context: &mut MoveContext, damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
            if context.properties.mov.type_effectiveness > 1 {
                damage.mul(Fraction::new(1u64, 2u64), $name);
                if context.defender.has_ability(["Ripen"]) {
                    damage.mul(Fraction::new(1u64, 2u64), "Ripen");
                }
                context.defender.item = None;
            }
        }) as _
    };
}

pub(crate) static MODIFY_STATE_FROM_MON_HOOKS: LazyLock<IndexMap<&str, ModifyStateFromMon>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "condition:Gastro Acid",
                (|context: &mut MoveContext, mon_type: MonType| {
                    context.mon_mut(mon_type).ability = None;
                }) as _,
            ),
            (
                "condition:Embargo",
                (|context: &mut MoveContext, mon_type: MonType| {
                    context.mon_mut(mon_type).item = None;
                }) as _,
            ),
            (
                "item:Utility Umbrella",
                (|context: &mut MoveContext, mon_type: MonType| {
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
                (|context: &mut MoveContext, _: MonType| {
                    context.field.weather = None;
                }) as _,
            ),
            (
                "ability:Cloud Nine",
                (|context: &mut MoveContext, _: MonType| {
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
        (|context: &mut MoveContext| {
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

pub(crate) static FAIL_MOVE_BEFORE_HIT_HOOKS: LazyLock<IndexMap<&str, FailMoveBeforeHit>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "terrain:Psychic Terrain:defender",
                (|context: &mut MoveContext, _: &mut Hit| context.move_data.priority > 0) as _,
            ),
            (
                "move:Brick Break",
                (|context: &mut MoveContext, _: &mut Hit| {
                    context.field.defender_side.conditions.remove("Reflect");
                    context
                        .field
                        .defender_side
                        .conditions
                        .remove("Light Screen");
                    context.field.defender_side.conditions.remove("Aurora Veil");
                    false
                }) as _,
            ),
            (
                "ability:Sturdy:defender",
                (|context: &mut MoveContext, _: &mut Hit| context.move_data.ohko_type.is_some())
                    as _,
            ),
            (
                "ability:Volt Absorb:defender",
                (|context: &mut MoveContext, hit: &mut Hit| {
                    let fail =
                        !context.flags.indirect && context.move_data.primary_type == Type::Electric;
                    if !context.flags.attacking_self && !context.flags.indirect {
                        hit.status_effect_on_target
                            .heal
                            .get_or_insert_default()
                            .add(
                                (context.max_hp(MonType::Defender) / 4).map(|val| val.floor()),
                                "Volt Absorb",
                            );
                    }
                    fail
                }) as _,
            ),
            (
                "ability:Water Absorb:defender",
                (|context: &mut MoveContext, hit: &mut Hit| {
                    let fail =
                        !context.flags.indirect && context.move_data.primary_type == Type::Water;
                    if !context.flags.attacking_self && !context.flags.indirect {
                        hit.status_effect_on_target
                            .heal
                            .get_or_insert_default()
                            .add(
                                (context.max_hp(MonType::Defender) / 4).map(|val| val.floor()),
                                "Water Absorb",
                            );
                    }
                    fail
                }) as _,
            ),
            (
                "ability:Flash Fire:defender",
                (|context: &mut MoveContext, hit: &mut Hit| {
                    if context.move_data.primary_type == Type::Fire {
                        hit.status_effect_on_target.volatile = Some("Flash Fire".to_owned());
                        return true;
                    }
                    false
                }) as _,
            ),
            (
                "ability:Wonder Guard:defender",
                (|context: &mut MoveContext, _: &mut Hit| {
                    if context.move_data.typeless {
                        return false;
                    }
                    !context.defender.types.iter().any(|typ| {
                        context.type_effectiveness(context.move_data.primary_type, *typ)
                            == TypeEffectiveness::Strong
                    })
                }) as _,
            ),
            (
                "ability:Lightning Rod:defender",
                (|context: &mut MoveContext, hit: &mut Hit| {
                    if context.move_data.primary_type == Type::Electric {
                        hit.status_effect_on_target
                            .boosts
                            .get_or_insert_default()
                            .spa += 1;
                        return true;
                    }
                    false
                }) as _,
            ),
            (
                "ability:Sap Sipper:defender",
                (|context: &mut MoveContext, hit: &mut Hit| {
                    if context.move_data.primary_type == Type::Grass {
                        hit.status_effect_on_target
                            .boosts
                            .get_or_insert_default()
                            .atk += 1;
                        return true;
                    }
                    false
                }) as _,
            ),
            (
                "ability:Soundproof:defender",
                (|context: &mut MoveContext, _: &mut Hit| {
                    context.move_data.flags.contains(&MoveFlag::Sound)
                }) as _,
            ),
        ])
    });

pub(crate) static APPLY_FIXED_DAMAGE_HOOKS: LazyLock<IndexMap<&str, ApplyFixedDamage>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "move:Seismic Toss",
                (|context: &MoveContext| Some(context.attacker.level.into())) as _,
            ),
            (
                "move:Night Shade",
                (|context: &MoveContext| Some(context.attacker.level.into())) as _,
            ),
            (
                "move:Psywave",
                (|context: &MoveContext| {
                    Some(Range::new(
                        50 * context.attacker.level / 100,
                        150 * context.attacker.level / 100,
                    ))
                }) as _,
            ),
            (
                "move:Super Fang",
                (|context: &MoveContext| {
                    let health = context.current_hp(MonType::Defender);
                    let damage = health / 2;
                    let damage = damage.map(|damage| damage.floor().max(1));
                    Some(damage)
                }) as _,
            ),
            (
                "move:Endeavor",
                (|context: &MoveContext| {
                    let attacker_health = context.current_hp(MonType::Attacker);
                    let defender_health = context.current_hp(MonType::Defender);
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
                (|context: &mut MoveContext| {
                    if let Some(typ) = context.attacker.hidden_power_type {
                        context.move_data.primary_type = typ;
                    }
                }) as _,
            ),
            (
                "move:Weather Ball",
                (|context: &mut MoveContext| {
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
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
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
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.field.has_weather([
                        "Rain",
                        "Heavy Rain",
                        "Sandstorm",
                        "Hail",
                        "Snow",
                    ]) {
                        base_power.mul(Fraction::new(1u64, 2u64), "weak weather");
                    }
                }) as _,
            ),
            (
                "move:Triple Kick",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.properties.mov.hit > 1 {
                        base_power.mul(context.properties.mov.hit, "additional hit");
                    }
                }) as _,
            ),
            (
                "move:Flail",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
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
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
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
                (|_: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    base_power.set(102u64, "max happiness");
                }) as _,
            ),
            (
                "move:Frustration",
                (|_: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    base_power.set(102u64, "min happiness");
                }) as _,
            ),
            (
                "move:Pursuit",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.defender.has_condition(["Switching"]) {
                        base_power.mul(2u64, "switching out");
                    }
                }) as _,
            ),
            (
                "move:Facade",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
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
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.defender.has_status(["Paralysis"]) {
                        base_power.mul(2u64, "paralysis boost");
                    }
                }) as _,
            ),
            (
                "move:Knock Off",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.defender.item.is_some() {
                        base_power.mul(Fraction::new(3, 2), "item boost");
                    }
                }) as _,
            ),
            (
                "move:Eruption",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    base_power.mul(
                        context.attacker.health.unwrap_or(Fraction::from(1u64)),
                        "user health",
                    );
                }) as _,
            ),
            (
                "move:Water Spout",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    base_power.mul(
                        context.attacker.health.unwrap_or(Fraction::from(1u64)),
                        "user health",
                    );
                }) as _,
            ),
            (
                "move:Weather Ball",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type != Type::Normal {
                        base_power.mul(2u64, "weather boost");
                    }
                }) as _,
            ),
            (
                "condition:Mud Sport",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Electric {
                        base_power.mul(Fraction::new(1u64, 3u64), "Mud Sport");
                    }
                }) as _,
            ),
            (
                "condition:Water Sport",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Fire {
                        base_power.mul(Fraction::new(1u64, 3u64), "Water Sport");
                    }
                }) as _,
            ),
            (
                "terrain:Misty Terrain:defender",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Dragon {
                        base_power.mul(Fraction::new(1u64, 2u64), "Misty Terrain");
                    }
                }) as _,
            ),
            (
                "terrain:Grassy Terrain:defender",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context
                        .mov
                        .is_named(["Earthquake", "Bulldoze", "Magnitude"])
                    {
                        base_power.mul(Fraction::new(1u64, 2u64), "Grassy Terrain");
                    } else if context.move_data.primary_type == Type::Grass {
                        base_power.mul(Fraction::new(13, 10), "Grassy Terrain");
                    }
                }) as _,
            ),
            (
                "terrain:Electric Terrain:defender",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Electric {
                        base_power.mul(Fraction::new(13, 10), "Electric Terrain");
                    }
                }) as _,
            ),
            (
                "terrain:Psychic Terrain:defender",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.move_data.primary_type == Type::Psychic {
                        base_power.mul(Fraction::new(13, 10), "Psychic Terrain");
                    }
                }) as _,
            ),
            (
                "condition:Charge:attacker",
                (|context: &MoveContext, base_power: &mut Output<Fraction<u64>>| {
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
                (|context: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    if context.move_data.primary_type == Type::Fire {
                        value.mul(2u64, "Flash Fire");
                    }
                }) as _,
            ),
            (
                "ability:Huge Power",
                (|_: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(2u64, "Huge Power");
                }) as _,
            ),
            (
                "ability:Pure Power",
                (|_: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(2u64, "Huge Power");
                }) as _,
            ),
            (
                "ability:Hustle",
                (|_: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(Fraction::new(3, 2), "Hustle");
                }) as _,
            ),
            (
                "ability:Guts",
                (|context: &MoveContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if context.mon(mon_type).status.is_some() {
                        value.mul(2u64, "Guts");
                    }
                }) as _,
            ),
            (
                "ability:Marvel Scale",
                (|context: &MoveContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if context.mon(mon_type).status.is_some() {
                        value.mul(2u64, "Guts");
                    }
                }) as _,
            ),
            (
                "ability:Overgrow",
                type_powering_ability!("Overgrow", Type::Grass),
            ),
            ("ability:Blaze", type_powering_ability!("Blaze", Type::Fire)),
            (
                "ability:Torrent",
                type_powering_ability!("Torrent", Type::Water),
            ),
            ("ability:Swarm", type_powering_ability!("Swarm", Type::Bug)),
            (
                "ability:Thick Fat:opposite",
                (|context: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    if context.move_data.primary_type == Type::Ice
                        || context.move_data.primary_type == Type::Fire
                    {
                        value.mul(Fraction::new(1u64, 2u64), "Thick Fat");
                    }
                }) as _,
            ),
            (
                "item:Choice Band",
                (|_: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(Fraction::new(3, 2), "Choice Band");
                }) as _,
            ),
            (
                "item:Light Ball",
                (|context: &MoveContext,
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
            (|context: &MoveContext,
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
                (|context: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    if context.move_data.primary_type == Type::Fire {
                        value.mul(2u64, "Flash Fire");
                    }
                }) as _,
            ),
            (
                "ability:Overgrow",
                type_powering_ability!("Overgrow", Type::Grass),
            ),
            ("ability:Blaze", type_powering_ability!("Blaze", Type::Fire)),
            (
                "ability:Torrent",
                type_powering_ability!("Torrent", Type::Water),
            ),
            ("ability:Swarm", type_powering_ability!("Swarm", Type::Bug)),
            (
                "ability:Thick Fat:opposite",
                (|context: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    if context.move_data.primary_type == Type::Ice
                        || context.move_data.primary_type == Type::Fire
                    {
                        value.mul(Fraction::new(1u64, 2u64), "Thick Fat");
                    }
                }) as _,
            ),
            (
                "item:Choice Specs",
                (|_: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(Fraction::new(3, 2), "Choice Specs");
                }) as _,
            ),
            (
                "item:Light Ball",
                (|context: &MoveContext,
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
            (|context: &MoveContext,
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
                (|context: &MoveContext,
                  mon_type: MonType,
                  value: &mut Output<Range<Fraction<u64>>>| {
                    if !context.mon(mon_type).has_ability(["Quick Feet"]) {
                        value.mul(Fraction::new(1u64, 2u64), "Paralysis");
                    }
                }) as _,
            ),
            (
                "ability:Swift Swim",
                (|context: &MoveContext,
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
                (|context: &MoveContext,
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
                (|_: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
                    value.mul(Fraction::new(1u64, 2u64), "Macho Brace");
                }) as _,
            ),
            (
                "item:Choice Scarf",
                (|_: &MoveContext, _: MonType, value: &mut Output<Range<Fraction<u64>>>| {
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
        (|context: &MoveContext, damage: &mut Output<Range<Fraction<u64>>>| {
            if context.move_data.primary_type == Type::Water {
                damage.mul(Fraction::new(3, 2), "Rain");
            }
            if context.move_data.primary_type == Type::Fire {
                damage.mul(Fraction::new(1u64, 2u64), "Rain");
            }
        }) as _,
    )])
});

pub(crate) static MODIFY_TYPE_EFFECTIVENESS_HOOKS: LazyLock<
    IndexMap<&str, ModifyTypeEffectiveness>,
> = LazyLock::new(|| {
    IndexMap::from_iter([(
        "weather:Strong Winds:defender",
        (|context: &MoveContext, effectiveness: &mut Output<Fraction<u64>>| {
            if context.defender.has_type([Type::Flying])
                && context.type_effectiveness(context.move_data.primary_type, Type::Flying)
                    == TypeEffectiveness::Strong
            {
                effectiveness.mul(Fraction::new(1u64, 2u64), "Strong Winds");
            }
        }) as _,
    )])
});

pub(crate) static MODIFY_DAMAGE_HOOKS: LazyLock<IndexMap<&str, ModifyDamage>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "status:Burn:attacker",
                (|context: &mut MoveContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.move_data.category == MoveCategory::Physical
                        && !context.attacker.has_ability(["Guts"])
                        && !context.mov.is_named(["Facade"])
                    {
                        damage.mul(Fraction::new(1u64, 2u64), "Burn");
                    }
                }) as _,
            ),
            (
                "condition:Fly:defender",
                (|context: &mut MoveContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.mov.is_named(["Gust", "Twister"]) {
                        damage.mul(2u64, "Fly");
                    }
                }) as _,
            ),
            (
                "condition:Bounce:defender",
                (|context: &mut MoveContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.mov.is_named(["Gust", "Twister"]) {
                        damage.mul(2u64, "Fly");
                    }
                }) as _,
            ),
            (
                "condition:Dig:defender",
                (|context: &mut MoveContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.mov.is_named(["Earthquake", "Magnitude"]) {
                        damage.mul(2u64, "Dig");
                    }
                }) as _,
            ),
            (
                "condition:Dive:defender",
                (|context: &mut MoveContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.mov.is_named(["Surf", "Whirlpool"]) {
                        damage.mul(2u64, "Dive");
                    }
                }) as _,
            ),
            (
                "condition:Minimize:defender",
                (|context: &mut MoveContext,
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
                (|context: &mut MoveContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.move_data.category != MoveCategory::Special || context.mov.crit {
                        return;
                    }
                    if context.field.battle_type != "Singles" {
                        damage.mul(Fraction::new(2, 3), "Light Screen");
                    } else {
                        damage.mul(Fraction::new(1u64, 2u64), "Light Screen");
                    }
                }) as _,
            ),
            (
                "condition:Reflect:defender",
                (|context: &mut MoveContext,
                  damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                    if context.move_data.category != MoveCategory::Physical || context.mov.crit {
                        return;
                    }
                    if context.field.battle_type != "Singles" {
                        damage.mul(Fraction::new(2, 3), "Reflect");
                    } else {
                        damage.mul(Fraction::new(1u64, 2u64), "Reflect");
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

pub(crate) static MODIFY_RECOIL_DAMAGE_HOOKS: LazyLock<IndexMap<&str, ModifyDamage>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "ability:Rock Head:attacker",
            (|_: &mut MoveContext, damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                damage.mul(0u64, "Rock Head");
            }) as _,
        )])
    });

pub(crate) static MODIFY_DRAIN_HOOKS: LazyLock<IndexMap<&str, ModifyDamage>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "condition:Liquid Ooze:defender",
            (|_: &mut MoveContext, damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                damage.mul(0u64, "Liquid Ooze");
            }) as _,
        )])
    });

pub(crate) static MODIFY_HEAL_HOOKS: LazyLock<IndexMap<&str, ModifyDirectDamage>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "condition:Heal Block",
            (|_: &mut MoveContext, _: MonType, damage: &mut Output<Range<Fraction<u64>>>| {
                damage.mul(0u64, "Heal Block");
            }) as _,
        )])
    });

pub(crate) static MODIFY_DIRECT_DAMAGE_FROM_HIT_HOOKS: LazyLock<
    IndexMap<&str, ModifyDirectDamage>,
> = LazyLock::new(|| {
    IndexMap::from_iter([(
        "move:Belly Drum",
        (|context: &mut MoveContext,
          mon_type: MonType,
          damage: &mut Output<Range<Fraction<u64>>>| {
            if mon_type == MonType::Defender {
                damage.add(context.max_hp(mon_type) / 2, "Belly Drum");
            }
        }) as _,
    )])
});

pub(crate) static MODIFY_STATUS_EFFECT_HOOKS: LazyLock<IndexMap<&str, ModifyStatusEffect>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "weather:Harsh Sunlight",
                (|_: &mut MoveContext, _: MonType, status_effect: &mut StatusEffect| {
                    status_effect.clear_status_if(["Freeze"]);
                }) as _,
            ),
            (
                "weather:Extremely Harsh Sunlight",
                (|_: &mut MoveContext, _: MonType, status_effect: &mut StatusEffect| {
                    status_effect.clear_status_if(["Freeze"]);
                }) as _,
            ),
            (
                "ability:Oblivious",
                (|_: &mut MoveContext, _: MonType, status_effect: &mut StatusEffect| {
                    status_effect.clear_volatile_if(["Attract"]);
                }) as _,
            ),
            (
                "ability:Own Tempo",
                (|_: &mut MoveContext, _: MonType, status_effect: &mut StatusEffect| {
                    status_effect.clear_volatile_if(["Confusion"]);
                }) as _,
            ),
            (
                "ability:Magma Armor",
                (|_: &mut MoveContext, _: MonType, status_effect: &mut StatusEffect| {
                    status_effect.clear_status_if(["Freeze"]);
                }) as _,
            ),
            (
                "item:Safety Goggles",
                (|context: &mut MoveContext, _: MonType, status_effect: &mut StatusEffect| {
                    if context.move_data.flags.contains(&MoveFlag::Powder) {
                        status_effect.status = None;
                        status_effect.volatile = None;
                    }
                }) as _,
            ),
            (
                "mon",
                (|context: &mut MoveContext,
                  mon_type: MonType,
                  status_effect: &mut StatusEffect| {
                    if context.move_data.flags.contains(&MoveFlag::Powder)
                        && context.mon(mon_type).has_type([Type::Grass])
                    {
                        status_effect.status = None;
                        status_effect.volatile = None;
                    }

                    if context.mon(mon_type).has_type([Type::Poison, Type::Steel]) {
                        status_effect.clear_status_if(["Poison", "Bad Poison"]);
                    }
                    if context.mon(mon_type).has_type([Type::Electric]) {
                        status_effect.clear_status_if(["Paralysis"]);
                    }
                    if context.mon(mon_type).has_type([Type::Fire]) {
                        status_effect.clear_status_if(["Burn"]);
                    }
                    if context.mon(mon_type).has_type([Type::Ice]) {
                        status_effect.clear_status_if(["Freeze"]);
                    }

                    if context.mon(mon_type).has_type([Type::Ghost]) {
                        status_effect.clear_volatile_if(["Trapped"]);
                    }
                }) as _,
            ),
        ])
    });

pub(crate) static MODIFY_STATE_AFTER_HIT_HOOKS: LazyLock<IndexMap<&str, ModifyStateAfterHit>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "move:Thief",
                (|context: &mut MoveContext| {
                    let item = context.defender.item.take();
                    if context.attacker.item.is_none() {
                        context.attacker.item = item;
                    }
                }) as _,
            ),
            (
                "move:Covet",
                (|context: &mut MoveContext| {
                    let item = context.defender.item.take();
                    if context.attacker.item.is_none() {
                        context.attacker.item = item;
                    }
                }) as _,
            ),
            (
                "move:Knock Off",
                (|context: &mut MoveContext| {
                    context.defender.item = None;
                }) as _,
            ),
            (
                "move:Smelling Salts",
                (|context: &mut MoveContext| {
                    if context.defender.has_status(["Paralysis"]) {
                        context.defender.status = None;
                    }
                }) as _,
            ),
            (
                "condition:Rage:defender",
                (|context: &mut MoveContext| {
                    if context.move_data.category == MoveCategory::Status {
                        context.defender.boosts.atk += 1;
                    }
                }) as _,
            ),
            (
                "status:Freeze:defender",
                (|context: &mut MoveContext| {
                    if context.move_data.primary_type == Type::Fire {
                        context.defender.status = None;
                    }
                }) as _,
            ),
            (
                "ability:Rough Skin:defender",
                (|context: &mut MoveContext| {
                    if context.move_makes_contact() {
                        context.chip_off_hp(MonType::Attacker, Fraction::new(1, 8));
                    }
                }) as _,
            ),
            (
                "item:Jaboca Berry:defender",
                (|context: &mut MoveContext| {
                    if context.move_data.category == MoveCategory::Physical {
                        context.chip_off_hp(MonType::Attacker, Fraction::new(1, 8));
                        context.defender.item = None;
                    }
                }) as _,
            ),
            (
                "item:Rowap Berry:defender",
                (|context: &mut MoveContext| {
                    if context.move_data.category == MoveCategory::Special {
                        context.chip_off_hp(MonType::Attacker, Fraction::new(1, 8));
                        context.defender.item = None;
                    }
                }) as _,
            ),
            (
                "item:Cell Battery:defender",
                (|context: &mut MoveContext| {
                    if context.move_data.primary_type == Type::Electric {
                        context.defender.boosts.atk += 1;
                    }
                }) as _,
            ),
        ])
    });

pub(crate) static MON_IS_GROUNDED_HOOKS: LazyLock<IndexMap<&str, CheckMonState>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "condition:Ingrain",
                (|_: &MoveContext, _: MonType| {
                    return Some(true);
                }) as _,
            ),
            (
                "ability:Levitate",
                (|_: &MoveContext, _: MonType| {
                    return Some(false);
                }) as _,
            ),
            (
                "mon",
                (|context: &MoveContext, mon_type: MonType| {
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
                (|context: &MoveContext, mon_type: MonType| {
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
                (|context: &MoveContext, mon_type: MonType| {
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
                (|context: &MoveContext, mon_type: MonType| {
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
            (|context: &MoveContext, mon_type: MonType| {
                if context.move_data.primary_type == Type::Ground
                    && !context.mon_is_grounded(mon_type)
                {
                    return Some(true);
                }
                None
            }) as _,
        )])
    });

pub(crate) static MON_IS_CONTACT_PROOF_HOOKS: LazyLock<IndexMap<&str, CheckMonState>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "item:Protective Pads",
            (|_: &MoveContext, _: MonType| Some(true)) as _,
        )])
    });
