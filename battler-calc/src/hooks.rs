use std::sync::LazyLock;

use battler_data::{
    Fraction,
    MoveCategory,
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
    state::Move,
};

/// Modifies the battle state from an effect on the field.
pub type ModifyStateFromField = Box<fn(&mut DamageContext)>;
/// Modifies the battle state from an effect on a side.
pub type ModifyStateFromSide = Box<fn(&mut DamageContext, MonType)>;
/// Modifies the battle state from an effect on a Mon.
pub type ModifyStateFromMon = Box<fn(&mut DamageContext, MonType)>;
/// Modifies the move being used.
pub type ModifyMove = Box<fn(&mut Move)>;
/// Applies fixed damage.
pub type ApplyFixedDamage = Box<fn(&DamageContext) -> Option<u64>>;
/// Modifies the move data.
pub type ModifyMoveData = Box<fn(&mut DamageContext)>;
/// Modifies the move base power.
pub type ModifyBasePower = Box<fn(&DamageContext, &mut Output<Fraction<u64>>)>;
/// Modifies a stat calculation.
pub type ModifyStat = Box<fn(&DamageContext, &mut Output<Range<Fraction<u64>>>)>;
/// Modifies damage from weather (before critical hit and randomization).
pub type ModifyDamageFromWeather = Box<fn(&DamageContext, &mut Output<Range<Fraction<u64>>>)>;
/// Modifies type effectiveness.
pub type ModifyTypeEffectiveness = Box<fn(&DamageContext, &mut Output<Fraction<u64>>)>;
/// Modifies damage.
pub type ModifyDamage = Box<fn(&DamageContext, &mut Output<RangeDistribution<Fraction<u64>>>)>;
/// Modifies the battle state after a hit.
pub type ModifyStateAfterHit = Box<fn(&mut DamageContext)>;
/// Checks some Mon state.
pub type CheckMonState = Box<fn(&DamageContext, MonType) -> Option<bool>>;

pub static MODIFY_STATE_FROM_FIELD_HOOKS: LazyLock<IndexMap<&str, ModifyStateFromField>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "ability:Air Lock",
            Box::new(
                (|context: &mut DamageContext| {
                    context.field.weather = None;
                }) as _,
            ),
        )])
    });

pub static MODIFY_STATE_FROM_SIDE_HOOKS: LazyLock<IndexMap<&str, ModifyStateFromSide>> =
    LazyLock::new(|| IndexMap::from_iter([]));

pub static MODIFY_STATE_FROM_MON_HOOKS: LazyLock<IndexMap<&str, ModifyStateFromMon>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "condition:Embargo",
                Box::new(
                    (|context: &mut DamageContext, mon_type: MonType| {
                        context.mon_mut(mon_type).item = None;
                    }) as _,
                ),
            ),
            (
                "item:Utility Umbrella",
                Box::new(
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
            ),
        ])
    });

pub static MODIFY_MOVE_HOOKS: LazyLock<IndexMap<&str, ModifyMove>> = LazyLock::new(|| {
    IndexMap::from_iter([(
        "move:Nature Power",
        Box::new(
            (|mov: &mut Move| {
                mov.name = "Tri Attack".to_owned();
            }) as _,
        ),
    )])
});

pub static APPLY_FIXED_DAMAGE_HOOKS: LazyLock<IndexMap<&str, ApplyFixedDamage>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "move:Seismic Toss",
            Box::new((|context: &DamageContext| Some(context.attacker.level)) as _),
        )])
    });

pub static MODIFY_MOVE_DATA_HOOKS: LazyLock<IndexMap<&str, ModifyMoveData>> =
    LazyLock::new(|| IndexMap::from_iter([]));

pub static MODIFY_BASE_POWER_HOOKS: LazyLock<IndexMap<&str, ModifyBasePower>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "move:Solar Beam",
            Box::new(
                (|context: &DamageContext, base_power: &mut Output<Fraction<u64>>| {
                    if context.field.has_weather([
                        "Rain",
                        "Heavy Rain",
                        "Sandstorm",
                        "Hail",
                        "Snow",
                    ]) {
                        base_power.div(2, "Solar Beam: weak weather");
                    }
                }) as _,
            ),
        )])
    });

pub static MODIFY_ATK_STAT_HOOKS: LazyLock<IndexMap<&str, ModifyStat>> =
    LazyLock::new(|| IndexMap::from_iter([]));

pub static MODIFY_DEF_STAT_HOOKS: LazyLock<IndexMap<&str, ModifyStat>> =
    LazyLock::new(|| IndexMap::from_iter([]));

pub static MODIFY_SPA_STAT_HOOKS: LazyLock<IndexMap<&str, ModifyStat>> =
    LazyLock::new(|| IndexMap::from_iter([]));

pub static MODIFY_SPD_STAT_HOOKS: LazyLock<IndexMap<&str, ModifyStat>> =
    LazyLock::new(|| IndexMap::from_iter([]));

pub static MODIFY_SPE_STAT_HOOKS: LazyLock<IndexMap<&str, ModifyStat>> =
    LazyLock::new(|| IndexMap::from_iter([]));

pub static MODIFY_DAMAGE_FROM_WEATHER_HOOKS: LazyLock<IndexMap<&str, ModifyDamageFromWeather>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "weather:Rain",
            Box::new(
                (|context: &DamageContext, damage: &mut Output<Range<Fraction<u64>>>| {
                    if context.mon_properties(MonType::Defender).weather_suppressed {
                        return;
                    }
                    if context.move_data.primary_type == Type::Water {
                        damage.mul(Fraction::new(3, 2), "Rain");
                    }
                    if context.move_data.primary_type == Type::Fire {
                        damage.div(2, "Rain");
                    }
                }) as _,
            ),
        )])
    });

pub static MODIFY_TYPE_EFFECTIVENESS_HOOKS: LazyLock<IndexMap<&str, ModifyTypeEffectiveness>> =
    LazyLock::new(|| {
        IndexMap::from_iter([(
            "weather:Strong Winds",
            Box::new(
                (|context: &DamageContext, effectiveness: &mut Output<Fraction<u64>>| {
                    if context.mon(MonType::Defender).has_type([Type::Flying])
                        && context.type_effectiveness(context.move_data.primary_type, Type::Flying)
                            == TypeEffectiveness::Strong
                    {
                        effectiveness.div(2, "Strong Winds");
                    }
                }) as _,
            ),
        )])
    });

pub static MODIFY_DAMAGE_HOOKS: LazyLock<IndexMap<&str, ModifyDamage>> = LazyLock::new(|| {
    IndexMap::from_iter([(
        "status:Burn:attacker",
        Box::new(
            (|context: &DamageContext, damage: &mut Output<RangeDistribution<Fraction<u64>>>| {
                if context.move_data.category == MoveCategory::Physical
                    && !context.attacker.has_ability(["Guts"])
                    && !context.mov.is_named(["Facade"])
                {
                    damage.div(2, "Burn");
                }
            }) as _,
        ),
    )])
});

pub static MODIFY_STATE_AFTER_HIT_HOOKS: LazyLock<IndexMap<&str, ModifyStateAfterHit>> =
    LazyLock::new(|| IndexMap::from_iter([]));

pub static MON_IS_GROUNDED_HOOKS: LazyLock<IndexMap<&str, CheckMonState>> = LazyLock::new(|| {
    IndexMap::from_iter([
        (
            "condition:Ingrain",
            Box::new(
                (|_: &DamageContext, _: MonType| {
                    return Some(false);
                }) as _,
            ),
        ),
        (
            "ability:Levitate",
            Box::new(
                (|_: &DamageContext, _: MonType| {
                    return Some(false);
                }) as _,
            ),
        ),
        (
            "mon",
            Box::new(
                (|context: &DamageContext, mon_type: MonType| {
                    if context.mon(mon_type).has_type([Type::Flying]) {
                        return Some(false);
                    }
                    None
                }) as _,
            ),
        ),
    ])
});

pub static MON_NEGATES_IMMUNITY_HOOKS: LazyLock<IndexMap<&str, CheckMonState>> =
    LazyLock::new(|| {
        IndexMap::from_iter([
            (
                "condition:Miracle Eye",
                Box::new(
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
            ),
            (
                "condition:Foresight",
                Box::new(
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
            ),
            (
                "mon",
                Box::new(
                    (|context: &DamageContext, mon_type: MonType| {
                        if context.move_data.primary_type == Type::Ground
                            && context.mon_is_grounded(mon_type)
                        {
                            return Some(true);
                        }
                        None
                    }) as _,
                ),
            ),
        ])
    });

pub static MON_IS_IMMUNE_HOOKS: LazyLock<IndexMap<&str, CheckMonState>> = LazyLock::new(|| {
    IndexMap::from_iter([(
        "mon",
        Box::new(
            (|context: &DamageContext, mon_type: MonType| {
                if context.move_data.primary_type == Type::Ground
                    && !context.mon_is_grounded(mon_type)
                {
                    return Some(true);
                }
                None
            }) as _,
        ),
    )])
});
