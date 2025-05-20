use std::{
    borrow::Borrow,
    fmt::Display,
    hash::Hash,
    ops::Neg,
    sync::LazyLock,
};

use ahash::HashSet;
use anyhow::{
    Error,
    Result,
};
use battler_data::{
    DataStoreByName,
    Fraction,
    MonOverride,
    MoveCategory,
    MoveData,
    MoveFlag,
    MultihitType,
    SpeciesData,
    Stat,
    Type,
    TypeEffectiveness,
};
use indexmap::IndexMap;
use num::{
    integer::Average,
    traits::SaturatingSub,
};

use crate::{
    common::{
        Output,
        Range,
        RangeDistribution,
    },
    hooks,
    state::{
        Field,
        Mon,
        Move,
        Side,
    },
    stats,
};

/// Input for the damage calculator.
pub struct DamageCalculatorInput<'d> {
    /// Data source.
    pub data: &'d dyn DataStoreByName,
    /// Field state.
    pub field: Field,
    /// Attacker state.
    pub attacker: Mon,
    /// Defender state.
    pub defender: Mon,
    /// Move being used.
    pub mov: Move,
}

impl<'d> TryInto<DamageContext<'d>> for DamageCalculatorInput<'d> {
    type Error = Error;
    fn try_into(self) -> Result<DamageContext<'d>> {
        let move_data = self
            .data
            .get_move_by_name(&self.mov.name)?
            .ok_or_else(|| Error::msg(format!("move {} does not exist", self.mov.name)))?;
        let attacker_species_data = self
            .data
            .get_species_by_name(&self.attacker.name)?
            .ok_or_else(|| Error::msg(format!("mon {} does not exist", self.attacker.name)))?;
        let defender_species_data = self
            .data
            .get_species_by_name(&self.defender.name)?
            .ok_or_else(|| Error::msg(format!("mon {} does not exist", self.defender.name)))?;
        Ok(DamageContext {
            data: self.data,
            field: self.field,
            attacker: self.attacker,
            defender: self.defender,
            mov: self.mov,
            move_data,
            attacker_species_data,
            defender_species_data,
            properties: Properties::default(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonType {
    Attacker,
    Defender,
}

impl Neg for MonType {
    type Output = Self;
    fn neg(self) -> Self::Output {
        match self {
            Self::Attacker => Self::Defender,
            Self::Defender => Self::Attacker,
        }
    }
}

impl Display for MonType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Attacker => write!(f, "attacker"),
            Self::Defender => write!(f, "defender"),
        }
    }
}

#[derive(Default)]
pub(crate) struct MonProperties {
    pub weather_suppressed: bool,
}

#[derive(Default)]
pub(crate) struct MoveProperties {
    pub hit: u64,
    pub type_effectiveness: Fraction<u64>,
}

#[derive(Default)]
pub(crate) struct Properties {
    pub attacker: MonProperties,
    pub defender: MonProperties,
    pub mov: MoveProperties,
}

pub(crate) struct DamageContext<'d> {
    pub data: &'d dyn DataStoreByName,
    pub field: Field,
    pub attacker: Mon,
    pub defender: Mon,
    pub mov: Move,
    pub move_data: MoveData,
    pub attacker_species_data: SpeciesData,
    pub defender_species_data: SpeciesData,
    pub properties: Properties,
}

impl<'d> DamageContext<'d> {
    pub fn side(&self, mon_type: MonType) -> &Side {
        match mon_type {
            MonType::Attacker => &self.field.attacker_side,
            MonType::Defender => &self.field.defender_side,
        }
    }

    #[allow(unused)]
    pub fn side_mut(&mut self, mon_type: MonType) -> &mut Side {
        match mon_type {
            MonType::Attacker => &mut self.field.attacker_side,
            MonType::Defender => &mut self.field.defender_side,
        }
    }

    pub fn mon(&self, mon_type: MonType) -> &Mon {
        match mon_type {
            MonType::Attacker => &self.attacker,
            MonType::Defender => &self.defender,
        }
    }

    pub fn mon_mut(&mut self, mon_type: MonType) -> &mut Mon {
        match mon_type {
            MonType::Attacker => &mut self.attacker,
            MonType::Defender => &mut self.defender,
        }
    }

    pub fn species(&self, mon_type: MonType) -> &SpeciesData {
        match mon_type {
            MonType::Attacker => &self.attacker_species_data,
            MonType::Defender => &self.defender_species_data,
        }
    }

    #[allow(unused)]
    pub fn species_mut(&mut self, mon_type: MonType) -> &mut SpeciesData {
        match mon_type {
            MonType::Attacker => &mut self.attacker_species_data,
            MonType::Defender => &mut self.defender_species_data,
        }
    }

    pub fn mon_properties(&self, mon_type: MonType) -> &MonProperties {
        match mon_type {
            MonType::Attacker => &self.properties.attacker,
            MonType::Defender => &self.properties.defender,
        }
    }

    pub fn mon_properties_mut(&mut self, mon_type: MonType) -> &mut MonProperties {
        match mon_type {
            MonType::Attacker => &mut self.properties.attacker,
            MonType::Defender => &mut self.properties.defender,
        }
    }

    pub fn type_effectiveness(&self, offense: Type, defense: Type) -> TypeEffectiveness {
        self.data
            .get_type_chart()
            .map(|type_chart| {
                type_chart
                    .types
                    .get(&offense)
                    .and_then(|row| row.get(&defense))
                    .cloned()
            })
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    pub fn mon_is_grounded(&self, mon_type: MonType) -> bool {
        check_mon_state(self, mon_type, &hooks::MON_IS_GROUNDED_HOOKS).unwrap_or(true)
    }

    pub fn mon_negates_immunity(&self, mon_type: MonType) -> bool {
        check_mon_state(self, mon_type, &hooks::MON_NEGATES_IMMUNITY_HOOKS).unwrap_or(false)
    }

    pub fn mon_is_immune(&self, mon_type: MonType) -> bool {
        check_mon_state(self, mon_type, &hooks::MON_IS_IMMUNE_HOOKS).unwrap_or_else(|| {
            self.mon(mon_type).types.iter().any(|typ| {
                self.type_effectiveness(self.move_data.primary_type, *typ)
                    == TypeEffectiveness::None
            })
        })
    }

    pub fn mon_is_contact_proof(&self, mon_type: MonType) -> bool {
        check_mon_state(self, mon_type, &hooks::MON_IS_CONTACT_PROOF_HOOKS).unwrap_or(false)
    }

    pub fn move_makes_contact(&self) -> bool {
        self.move_data.flags.contains(&MoveFlag::Contact)
            && !self.mon_is_contact_proof(MonType::Defender)
    }

    pub fn calculate_stat(&self, mon_type: MonType, stat: Stat) -> Output<Range<u64>> {
        calculate_single_stat(self, mon_type, mon_type, stat, None).unwrap_or_default()
    }

    pub fn current_hp(&self, mon_type: MonType) -> Range<Fraction<u64>> {
        let hp = self
            .calculate_stat(mon_type, Stat::HP)
            .value()
            .map(|health| Fraction::from(health));
        hp * self.mon(mon_type).health.unwrap_or(Fraction::from(1u64))
    }

    pub fn chip_off_hp(&mut self, mon_type: MonType, damage: Fraction<u64>) {
        let health = self.mon(mon_type).health.unwrap_or(Fraction::from(1u64));
        let health = health.saturating_sub(&damage);
        self.mon_mut(mon_type).health = Some(health);
    }
}

/// Output of a single hit of a move in the damage calculator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DamageOutput {
    /// Move base power.
    pub base_power: Option<Output<u64>>,
    /// Attack stat.
    pub attack: Option<(Stat, Output<Range<u64>>)>,
    /// Defense stat.
    pub defense: Option<(Stat, Output<Range<u64>>)>,
    /// Type effectiveness modifier.
    pub type_effectiveness: Option<Output<Fraction<u64>>>,
    /// The total HP of the defender.
    ///
    /// Can be used to convert damage to a percentage.
    pub hp: Range<u64>,
    /// Possible damage distribution.
    ///
    /// Distribution is used due to the randomization factor.
    pub damage: Output<RangeDistribution<u64>>,
}

impl DamageOutput {
    fn zero<S>(reason: S) -> Self
    where
        S: Display,
    {
        Self::fixed(Range::from(0), reason)
    }

    fn fixed<S>(damage: Range<u64>, reason: S) -> Self
    where
        S: Display,
    {
        Self {
            damage: Output::start(RangeDistribution::from(damage), reason),
            ..Default::default()
        }
    }
}

impl Default for DamageOutput {
    fn default() -> Self {
        Self {
            base_power: None,
            attack: None,
            defense: None,
            type_effectiveness: None,
            hp: Range::from(1u64),
            damage: Output::from(RangeDistribution::from(Range::from(0u64))),
        }
    }
}

/// Output of the damage calculator.
///
/// A move is generically able to hit multiple times, which will produce multiple damage
/// calculations.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MultiHitDamageOutput {
    pub hits: Vec<DamageOutput>,
}

impl From<DamageOutput> for MultiHitDamageOutput {
    fn from(value: DamageOutput) -> Self {
        Self {
            hits: Vec::from_iter([value]),
        }
    }
}

/// Calculates the possible damage distribution of a move.
pub fn calculate_damage(input: DamageCalculatorInput) -> Result<MultiHitDamageOutput> {
    let move_data = input
        .data
        .get_move_by_name(&input.mov.name)?
        .ok_or_else(|| Error::msg(format!("move {} does not exist", input.mov.name)))?;
    let attacker_species_data = input
        .data
        .get_species_by_name(&input.attacker.name)?
        .ok_or_else(|| Error::msg(format!("mon {} does not exist", input.attacker.name)))?;
    let defender_species_data = input
        .data
        .get_species_by_name(&input.defender.name)?
        .ok_or_else(|| Error::msg(format!("mon {} does not exist", input.defender.name)))?;
    let context = DamageContext {
        data: input.data,
        field: input.field,
        attacker: input.attacker,
        defender: input.defender,
        mov: input.mov,
        move_data,
        attacker_species_data,
        defender_species_data,
        properties: Properties::default(),
    };
    calculate_damage_internal(context)
}

fn calculate_damage_internal(mut context: DamageContext) -> Result<MultiHitDamageOutput> {
    apply_defaults_to_mon(&mut context, MonType::Attacker);
    apply_defaults_to_mon(&mut context, MonType::Defender);

    // First, modify the battle state based on all effects.
    //
    // The idea is to modify the state up front, then all of the damage calculation hooks can assume
    // the battle state is in the correct form.

    // Modify the Mons first, since abilities.
    modify_state_from_mon(&mut context, MonType::Attacker);
    modify_state_from_mon(&mut context, MonType::Defender);
    modify_state_from_side(&mut context, MonType::Attacker);
    modify_state_from_side(&mut context, MonType::Defender);
    modify_state_from_field(&mut context);

    modify_move(&mut context);

    // Move may have changed.
    if context.move_data.name != context.mov.name {
        context.move_data = context
            .data
            .get_move_by_name(&context.mov.name)?
            .ok_or_else(|| Error::msg(format!("move {} does not exist", context.mov.name)))?;
    }

    let hits = match context.move_data.multihit {
        Some(MultihitType::Static(hits)) => hits.into(),
        Some(MultihitType::Range(a, b)) => match context.mov.hits {
            Some(hits) => hits.clamp(a.into(), b.into()),
            None => a.average_floor(&b).into(),
        },
        None => 1,
    };

    let hp = calculate_single_stat(
        &mut context,
        MonType::Defender,
        MonType::Defender,
        Stat::HP,
        None,
    )?;
    let hp = *hp.value();

    let hits = (0..hits)
        .map(|hit| {
            context.properties.mov.hit = hit + 1;
            let mut damage = calculate_damage_for_hit(&mut context)?;
            damage.hp = hp;
            modify_state_after_hit(&mut context);
            Ok(damage)
        })
        .collect::<Result<_>>()?;
    Ok(MultiHitDamageOutput { hits })
}

fn calculate_damage_for_hit(context: &mut DamageContext) -> Result<DamageOutput> {
    if !context.mon_negates_immunity(MonType::Defender) && context.mon_is_immune(MonType::Defender)
    {
        return Ok(DamageOutput::zero("immune"));
    }

    if let Some(fail_from) = fail_move_before_hit(context) {
        return Ok(DamageOutput::zero(fail_from));
    }

    if context.move_data.ohko_type.is_some() {
        return Ok(DamageOutput::fixed(
            *context.calculate_stat(MonType::Defender, Stat::HP).value(),
            "ohko",
        ));
    }

    if let Some(fixed) = apply_fixed_damage(context) {
        return Ok(DamageOutput::fixed(fixed, "fixed"));
    }

    // Modify the MoveData, which can primarily set the starting base power.
    modify_move_data(context);

    // Calculate the dynamic base power.
    let mut base_power = Output::from(Fraction::from(context.move_data.base_power as u64));
    modify_base_power(context, &mut base_power);

    let base_power = base_power.map(|val| val.floor(), "floor");

    if *base_power.value() == 0 {
        let mut output = DamageOutput::zero("no base power");
        output.base_power = Some(base_power);
        return Ok(output);
    }

    let level = context.mon(MonType::Attacker).level;
    let category = context.move_data.category;
    let attack_stat = context
        .move_data
        .override_offensive_stat
        .unwrap_or_else(|| {
            if category == MoveCategory::Physical {
                Stat::Atk
            } else {
                Stat::SpAtk
            }
        });
    let defense_stat = context
        .move_data
        .override_defensive_stat
        .unwrap_or_else(|| {
            if category == MoveCategory::Physical {
                Stat::Def
            } else {
                Stat::SpDef
            }
        });
    let attacker = match context.move_data.override_offensive_mon {
        Some(MonOverride::Target) => MonType::Defender,
        Some(MonOverride::User) | None => MonType::Attacker,
    };
    let defender = match context.move_data.override_offensive_mon {
        Some(MonOverride::User) => MonType::Attacker,
        Some(MonOverride::Target) | None => MonType::Defender,
    };

    let mut attack_boost = context.mon(attacker).boosts.get(attack_stat.try_into()?);
    let mut defense_boost = context.mon(defender).boosts.get(defense_stat.try_into()?);

    if context.move_data.ignore_offensive || (context.mov.crit && attack_boost < 0) {
        attack_boost = 0;
    }
    if context.move_data.ignore_defensive || (context.mov.crit && defense_boost > 0) {
        defense_boost = 0;
    }

    let attack = calculate_single_stat(
        context,
        attacker,
        MonType::Attacker,
        attack_stat,
        Some(attack_boost),
    )?;
    let defense = calculate_single_stat(
        context,
        defender,
        MonType::Defender,
        defense_stat,
        Some(defense_boost),
    )?;

    let level_component = 2 * level / 5 + 2;
    let mut base_damage_range: Output<Range<u64>> = Output::start(*attack.value(), "attack");
    base_damage_range.mul(level_component, "attacker level");
    base_damage_range.mul(*base_power.value(), "base power");
    base_damage_range.div(*defense.value(), "defense");
    base_damage_range.div(50, "constant");

    base_damage_range.add(2u64, "constant");

    let (type_effectiveness, damage) = apply_damage_modifiers(context, base_damage_range)?;

    Ok(DamageOutput {
        base_power: Some(base_power),
        attack: Some((attack_stat, attack)),
        defense: Some((defense_stat, defense)),
        type_effectiveness: Some(type_effectiveness),
        damage,
        ..Default::default()
    })
}

fn apply_damage_modifiers(
    context: &mut DamageContext,
    base_damage_range: Output<Range<u64>>,
) -> Result<(Output<Fraction<u64>>, Output<RangeDistribution<u64>>)> {
    let mut damage =
        base_damage_range.map(|range| range.map(|val| Fraction::from(val)), "fraction");

    if context.mov.spread {
        damage.mul(Fraction::new(3, 4), "spread");
    }

    modify_damage_from_weather(context, &mut damage);

    if context.mov.crit {
        damage.mul(Fraction::new(3, 2), "crit");
    }

    // Create a distribution based on randomization.
    let mut damage = damage.map(
        |damage| {
            RangeDistribution::from_iter((0u64..16).map(|n| 100 - n).map(|n| damage * n / 100))
        },
        "randomize",
    );

    if !context.move_data.typeless
        && context
            .mon(MonType::Attacker)
            .has_type([context.move_data.primary_type])
    {
        damage.mul(Fraction::new(3, 2), "stab");
    }

    let mut type_effectiveness = Output::from(Fraction::from(1u64));
    for defense_type in &context.mon(MonType::Defender).types {
        match context.type_effectiveness(context.move_data.primary_type, *defense_type) {
            // Immunity is handled much earlier, so it should never occur here.
            TypeEffectiveness::None => (),
            TypeEffectiveness::Normal => (),
            TypeEffectiveness::Strong => {
                type_effectiveness.mul(2, format!("super effective against {defense_type}"))
            }
            TypeEffectiveness::Weak => type_effectiveness.mul(
                Fraction::new(1, 2),
                format!("not very effective against {defense_type}"),
            ),
        }
    }
    modify_type_effectiveness(context, &mut type_effectiveness);

    context.properties.mov.type_effectiveness = *type_effectiveness.value();
    damage.mul(*type_effectiveness.value(), "type effectiveness");

    modify_damage(context, &mut damage);

    let damage = damage.map(
        |val| {
            RangeDistribution::from_iter(
                val.into_iter()
                    .map(|range| range.map(|val| val.floor().max(1))),
            )
        },
        "floor",
    );
    Ok((type_effectiveness, damage))
}

/// Calculates stats that would be used during damage calculation, based on the given battle state.
pub fn calculate_stats(input: DamageCalculatorInput) -> Result<stats::Stats<Output<Range<u64>>>> {
    let context = input.try_into()?;
    let base_stats = calculate_base_stats(&context, MonType::Attacker)?;
    let hp = calculate_single_stat_internal(
        &context,
        MonType::Attacker,
        MonType::Attacker,
        Stat::HP,
        &base_stats,
        None,
    );
    let atk = calculate_single_stat_internal(
        &context,
        MonType::Attacker,
        MonType::Attacker,
        Stat::Atk,
        &base_stats,
        None,
    );
    let def = calculate_single_stat_internal(
        &context,
        MonType::Attacker,
        MonType::Attacker,
        Stat::Def,
        &base_stats,
        None,
    );
    let spa = calculate_single_stat_internal(
        &context,
        MonType::Attacker,
        MonType::Attacker,
        Stat::SpAtk,
        &base_stats,
        None,
    );
    let spd = calculate_single_stat_internal(
        &context,
        MonType::Attacker,
        MonType::Attacker,
        Stat::SpDef,
        &base_stats,
        None,
    );
    let spe = calculate_single_stat_internal(
        &context,
        MonType::Attacker,
        MonType::Attacker,
        Stat::Spe,
        &base_stats,
        None,
    );
    Ok(stats::Stats {
        hp,
        atk,
        def,
        spa,
        spd,
        spe,
    })
}

fn calculate_base_stats(
    context: &DamageContext,
    mon_type: MonType,
) -> Result<stats::Stats<Range<u64>>> {
    let mon = context.mon(mon_type);
    stats::calculate_stats(
        context.data,
        &mon.name,
        mon.level,
        mon.nature,
        mon.ivs
            .as_ref()
            .map(|stats| stats::Stats {
                hp: Range::from(stats.hp as u64),
                atk: Range::from(stats.atk as u64),
                def: Range::from(stats.def as u64),
                spa: Range::from(stats.spa as u64),
                spd: Range::from(stats.spd as u64),
                spe: Range::from(stats.spe as u64),
            })
            .as_ref(),
        mon.evs
            .as_ref()
            .map(|stats| stats::Stats {
                hp: Range::from(stats.hp as u64),
                atk: Range::from(stats.atk as u64),
                def: Range::from(stats.def as u64),
                spa: Range::from(stats.spa as u64),
                spd: Range::from(stats.spd as u64),
                spe: Range::from(stats.spe as u64),
            })
            .as_ref(),
    )
}

fn calculate_single_stat(
    context: &DamageContext,
    mon_type: MonType,
    stat_user: MonType,
    stat: Stat,
    boost: Option<i8>,
) -> Result<Output<Range<u64>>> {
    Ok(calculate_single_stat_internal(
        context,
        mon_type,
        stat_user,
        stat,
        &calculate_base_stats(context, mon_type)?,
        boost,
    ))
}

fn calculate_single_stat_internal(
    context: &DamageContext,
    mon_type: MonType,
    stat_user: MonType,
    stat: Stat,
    base_stats: &stats::Stats<Range<u64>>,
    boost: Option<i8>,
) -> Output<Range<u64>> {
    if stat == Stat::HP {
        return Output::from(Range::from(base_stats.hp));
    }

    let mut value = Output::from(base_stats.get(stat).map(|val| Fraction::<u64>::from(val)));

    let boost = match boost {
        Some(boost) => boost,
        // SAFETY: All stats except HP can convert to Boost. HP triggers an early return.
        None => context.mon(mon_type).boosts.get(stat.try_into().unwrap()),
    };

    static BOOST_TABLE: LazyLock<[Fraction<u16>; 7]> = LazyLock::new(|| {
        [
            Fraction::new(1, 1),
            Fraction::new(3, 2),
            Fraction::new(2, 1),
            Fraction::new(5, 2),
            Fraction::new(3, 1),
            Fraction::new(7, 2),
            Fraction::new(4, 1),
        ]
    });

    let boost = boost.clamp(-6, 6);
    // SAFETY: boost.abs() is between 0 and 6.
    let boost_fraction = BOOST_TABLE.get(boost.abs() as usize).unwrap().convert();
    if boost > 0 {
        value.mul(boost_fraction, "boost");
    } else if boost < 0 {
        value.mul(boost_fraction.inverse(), "drop");
    }

    modify_stat(context, stat, stat_user, &mut value);

    value.map(|val| val.map(|val| val.floor()), "floor")
}

fn apply_defaults_to_mon(context: &mut DamageContext, mon_type: MonType) {
    if context.mon(mon_type).types.is_empty() {
        let species = context.species(mon_type);
        let mut types = Vec::from_iter([species.primary_type]);
        if let Some(secondary) = species.secondary_type {
            types.push(secondary);
        }
        context.mon_mut(mon_type).types = types;
    }
}

fn field_effects(field: &Field) -> HashSet<String> {
    let mut effects = HashSet::default();
    if let Some(weather) = &field.weather {
        effects.insert(format!("weather:{weather}"));
    }
    if let Some(terrain) = &field.terrain {
        effects.insert(format!("terrain:{terrain}"));
    }
    for condition in &field.conditions {
        effects.insert(format!("condition:{condition}"));
    }
    effects
}

fn side_effects(side: &Side) -> HashSet<String> {
    let mut effects = HashSet::default();
    for condition in &side.conditions {
        effects.insert(format!("condition:{condition}"));
    }
    effects
}

fn mon_effects(mon: &Mon) -> HashSet<String> {
    let mut effects = HashSet::default();
    if let Some(ability) = &mon.ability {
        effects.insert(format!("ability:{ability}"));
    }
    if let Some(item) = &mon.item {
        effects.insert(format!("item:{item}"));
    }
    if let Some(status) = &mon.status {
        effects.insert(format!("status:{status}"));
    }
    for condition in &mon.conditions {
        effects.insert(format!("condition:{condition}"));
    }
    effects.insert("mon".to_owned());
    effects
}

fn modify_effects_for_focus(
    effects: HashSet<String>,
    mon_type: MonType,
    focus: Option<MonType>,
) -> HashSet<String> {
    match focus {
        Some(focus) => {
            if focus == mon_type {
                effects
            } else {
                effects
                    .into_iter()
                    .map(|effect| format!("{effect}:opposite"))
                    .collect()
            }
        }
        None => {
            let mut effects = effects;
            effects.extend(
                effects
                    .clone()
                    .iter()
                    .map(|effect| format!("{effect}:{mon_type}")),
            );
            effects
        }
    }
}

fn effects_for_source(
    context: &DamageContext,
    source: EffectSource,
    focus: Option<MonType>,
) -> HashSet<String> {
    match source {
        EffectSource::Field => field_effects(&context.field),
        EffectSource::Side(mon_type) => {
            let effects = side_effects(context.side(mon_type));
            modify_effects_for_focus(effects, mon_type, focus)
        }
        EffectSource::Mon(mon_type) => {
            let mut effects = mon_effects(context.mon(mon_type));

            // Weather and terrain can apply on Mons individually.
            if let Some(weather) = &context.field.weather
                && !context.mon_properties(mon_type).weather_suppressed
            {
                effects.insert(format!("weather:{weather}"));
            }
            if let Some(terrain) = &context.field.terrain {
                effects.insert(format!("terrain:{terrain}"));
            }

            modify_effects_for_focus(effects, mon_type, focus)
        }
    }
}

fn all_effects(context: &DamageContext, focus: Option<MonType>) -> HashSet<String> {
    let mut effects = HashSet::default();
    effects.extend(effects_for_source(&context, EffectSource::Field, focus));
    effects.extend(effects_for_source(
        &context,
        EffectSource::Side(MonType::Attacker),
        focus,
    ));
    effects.extend(effects_for_source(
        &context,
        EffectSource::Side(MonType::Defender),
        focus,
    ));
    effects.extend(effects_for_source(
        &context,
        EffectSource::Mon(MonType::Attacker),
        focus,
    ));
    effects.extend(effects_for_source(
        &context,
        EffectSource::Mon(MonType::Defender),
        focus,
    ));
    effects.insert(format!("move:{}", context.mov.name));
    effects
}

fn effect_type_and_name(name: &str) -> (&str, &str) {
    let mut split = name.splitn(3, ':');
    let effect_type = split.next();
    let effect_name = split.next();
    match (effect_type, effect_name) {
        (Some(effect_type), Some(effect_name)) => (effect_type, effect_name),
        _ => ("condition", name),
    }
}

#[derive(Debug, Clone, Copy)]
enum EffectSource {
    Field,
    Mon(MonType),
    Side(MonType),
}

fn effect_still_applies(context: &DamageContext, name: &str, on: EffectSource) -> bool {
    let (effect_type, name) = effect_type_and_name(name);
    match (effect_type, on) {
        ("ability", EffectSource::Mon(mon_type)) => context.mon(mon_type).has_ability([name]),
        ("condition", EffectSource::Field) => context.field.has_condition([name]),
        ("condition", EffectSource::Mon(mon_type)) => context.mon(mon_type).has_condition([name]),
        ("condition", EffectSource::Side(mon_type)) => context.side(mon_type).has_condition([name]),
        ("item", EffectSource::Mon(mon_type)) => context.mon(mon_type).has_item([name]),
        ("status", EffectSource::Mon(mon_type)) => context.mon(mon_type).has_status([name]),
        ("terrain", EffectSource::Field) => context.field.has_terrain([name]),
        ("weather", EffectSource::Field) => context.field.has_weather([name]),
        _ => true,
    }
}

fn get_ordered_hooks_by_effects<'h, S, T>(
    effects: &HashSet<S>,
    hooks: &'h IndexMap<&str, T>,
) -> Vec<(&'h str, &'h T)>
where
    S: Borrow<str> + Eq + Hash,
{
    hooks
        .iter()
        .filter(|(name, _)| effects.contains(name))
        .map(|(name, hook)| (*name, hook))
        .collect()
}

fn modify_state_from_field(context: &mut DamageContext) {
    let effects = effects_for_source(&context, EffectSource::Field, None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_STATE_FROM_FIELD_HOOKS);
    for (name, hook) in hooks {
        if !effect_still_applies(context, name, EffectSource::Field) {
            continue;
        }
        hook(context);
    }
}

fn modify_state_from_side(context: &mut DamageContext, mon_type: MonType) {
    let effects = effects_for_source(&context, EffectSource::Side(mon_type), None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_STATE_FROM_SIDE_HOOKS);
    for (name, hook) in hooks {
        if !effect_still_applies(context, name, EffectSource::Side(mon_type)) {
            continue;
        }
        hook(context, mon_type);
    }

    // Also consider the Mon on this side.
    let effects = effects_for_source(&context, EffectSource::Mon(mon_type), None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_STATE_FROM_SIDE_HOOKS);
    for (name, hook) in hooks {
        if !effect_still_applies(context, name, EffectSource::Mon(mon_type)) {
            continue;
        }
        hook(context, mon_type);
    }
}

fn modify_state_from_mon(context: &mut DamageContext, mon_type: MonType) {
    let effects = effects_for_source(&context, EffectSource::Mon(mon_type), None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_STATE_FROM_MON_HOOKS);
    for (name, hook) in hooks {
        if !effect_still_applies(context, name, EffectSource::Mon(mon_type)) {
            continue;
        }
        hook(context, mon_type);
    }
}

fn modify_move(context: &mut DamageContext) {
    let effects = all_effects(context, None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_MOVE_HOOKS);
    for (_, hook) in hooks {
        hook(context);
    }
}

fn fail_move_before_hit(context: &mut DamageContext) -> Option<String> {
    let effects = all_effects(context, None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::FAIL_MOVE_BEFORE_HIT_HOOKS);
    for (name, hook) in hooks {
        if hook(context) {
            let (_, name) = effect_type_and_name(name);
            return Some(name.to_owned());
        }
    }
    None
}

fn check_mon_state(
    context: &DamageContext,
    mon_type: MonType,
    hooks: &IndexMap<&str, hooks::CheckMonState>,
) -> Option<bool> {
    let effects = all_effects(context, Some(mon_type));
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks);
    for (_, hook) in hooks {
        if let Some(val) = hook(context, mon_type) {
            return Some(val);
        }
    }
    None
}

fn apply_fixed_damage(context: &DamageContext) -> Option<Range<u64>> {
    let effects = all_effects(context, None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::APPLY_FIXED_DAMAGE_HOOKS);
    for (_, hook) in hooks {
        if let Some(val) = hook(context) {
            return Some(val);
        }
    }
    None
}

fn modify_move_data(context: &mut DamageContext) {
    let effects = all_effects(context, None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_MOVE_DATA_HOOKS);
    for (_, hook) in hooks {
        hook(context);
    }
}

fn modify_base_power(context: &mut DamageContext, base_power: &mut Output<Fraction<u64>>) {
    let effects = all_effects(context, None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_BASE_POWER_HOOKS);
    for (_, hook) in hooks {
        hook(context, base_power);
    }
}

fn modify_stat(
    context: &DamageContext,
    stat: Stat,
    mon_type: MonType,
    value: &mut Output<Range<Fraction<u64>>>,
) {
    let empty = IndexMap::default();
    let effects = all_effects(context, Some(mon_type));
    let hooks = get_ordered_hooks_by_effects(
        &effects,
        match stat {
            Stat::HP => &empty,
            Stat::Atk => &hooks::MODIFY_ATK_STAT_HOOKS,
            Stat::Def => &hooks::MODIFY_DEF_STAT_HOOKS,
            Stat::SpAtk => &hooks::MODIFY_SPA_STAT_HOOKS,
            Stat::SpDef => &hooks::MODIFY_SPD_STAT_HOOKS,
            Stat::Spe => &hooks::MODIFY_SPE_STAT_HOOKS,
        },
    );
    for (_, hook) in hooks {
        hook(context, mon_type, value);
    }
}

fn modify_damage_from_weather(context: &DamageContext, damage: &mut Output<Range<Fraction<u64>>>) {
    let effects = all_effects(context, None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_DAMAGE_FROM_WEATHER_HOOKS);
    for (_, hook) in hooks {
        hook(context, damage);
    }
}

fn modify_type_effectiveness(context: &DamageContext, effectiveness: &mut Output<Fraction<u64>>) {
    let effects = all_effects(context, None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_TYPE_EFFECTIVENESS_HOOKS);
    for (_, hook) in hooks {
        hook(context, effectiveness);
    }
}

fn modify_damage(
    context: &mut DamageContext,
    damage: &mut Output<RangeDistribution<Fraction<u64>>>,
) {
    let effects = all_effects(context, None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_DAMAGE_HOOKS);
    for (_, hook) in hooks {
        hook(context, damage);
    }
}

fn modify_state_after_hit(context: &mut DamageContext) {
    let effects = all_effects(context, None);
    let hooks = get_ordered_hooks_by_effects(&effects, &hooks::MODIFY_STATE_AFTER_HIT_HOOKS);
    for (_, hook) in hooks {
        hook(context);
    }
}

#[cfg(test)]
mod damage_test {
    use ahash::HashSet;
    use battler_data::{
        BoostTable,
        Fraction,
        LocalDataStore,
        Nature,
        Stat,
        StatTable,
    };

    use crate::{
        common::{
            Output,
            Range,
            RangeDistribution,
        },
        damage::{
            DamageCalculatorInput,
            DamageOutput,
            MultiHitDamageOutput,
            calculate_damage,
        },
        state::{
            Field,
            Mon,
            Move,
        },
    };

    fn max_ivs() -> StatTable {
        StatTable {
            hp: 31,
            atk: 31,
            def: 31,
            spa: 31,
            spd: 31,
            spe: 31,
        }
    }

    fn empty_evs() -> StatTable {
        StatTable::default()
    }

    #[test]
    fn fixed_damage() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Bulbasaur".to_owned(),
                level: 5,
                ..Default::default()
            },
            defender: Mon {
                name: "Charmander".to_owned(),
                level: 5,
                ..Default::default()
            },
            mov: Move {
                name: "Seismic Toss".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            pretty_assertions::assert_eq!(output, MultiHitDamageOutput {
                hits: Vec::from_iter([
                    DamageOutput {
                        hp: Range::new(18, 23),
                        damage: Output::new(RangeDistribution::from_iter([Range::new(5, 5)]), ["=[[5,5]] - fixed"]),
                        ..Default::default()
                    },
                ]),
            });
        });
    }

    #[test]
    fn basic_tackle_with_high_stat_variance() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                ..Default::default()
            },
            mov: Move {
                name: "Tackle".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            pretty_assertions::assert_eq!(output, MultiHitDamageOutput {
                hits: Vec::from_iter([
                    DamageOutput {
                        base_power: Some(Output::new(40u64, ["[mapped] - floor"])),
                        attack: Some((Stat::Atk, Output::new(Range::new(152, 289), ["[mapped] - floor"]))),
                        defense: Some((Stat::Def, Output::new(Range::new(144, 280), ["[mapped] - floor"]))),
                        type_effectiveness: Some(Output::new::<_, _, &str>(Fraction::from(1u64), [])),
                        hp: Range::new(266, 360),
                        damage: Output::new(RangeDistribution::from_iter([
                            Range::new(20, 69),
                            Range::new(19, 68),
                            Range::new(19, 67),
                            Range::new(19, 66),
                            Range::new(19, 66),
                            Range::new(19, 65),
                            Range::new(18, 64),
                            Range::new(18, 64),
                            Range::new(18, 63),
                            Range::new(18, 62),
                            Range::new(18, 62),
                            Range::new(17, 61),
                            Range::new(17, 60),
                            Range::new(17, 60),
                            Range::new(17, 59),
                            Range::new(17, 58),
                        ]), [
                            "=[152,289] - attack",
                            "x42 - attacker level",
                            "x40 - base power",
                            "\u{00F7}[144,280] - defense",
                            "\u{00F7}50 - constant",
                            "+2 - constant",
                            "[mapped] - fraction",
                            "[mapped] - randomize",
                            "x1 - type effectiveness",
                            "[mapped] - floor",
                        ]),
                    },
                ]),
            });
        });
    }

    #[test]
    fn basic_tackle_with_no_stat_variance() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                nature: Some(Nature::Serious),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Timid),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Tackle".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            assert_matches::assert_matches!(output.hits[0].damage.value().min(), Some(31));
            assert_matches::assert_matches!(output.hits[0].damage.value().max(), Some(37));

            pretty_assertions::assert_eq!(output, MultiHitDamageOutput {
                hits: Vec::from_iter([
                    DamageOutput {
                        base_power: Some(Output::new(40u64, ["[mapped] - floor"])),
                        attack: Some((Stat::Atk, Output::new(Range::new(200, 200), ["[mapped] - floor"]))),
                        defense: Some((Stat::Def, Output::new(Range::new(192, 192), ["[mapped] - floor"]))),
                        type_effectiveness: Some(Output::new::<_, _, &str>(Fraction::from(1u64), [])),
                        hp: Range::new(297, 297),
                        damage: Output::new(RangeDistribution::from_iter([
                            Range::new(37, 37),
                            Range::new(36, 36),
                            Range::new(36, 36),
                            Range::new(35, 35),
                            Range::new(35, 35),
                            Range::new(35, 35),
                            Range::new(34, 34),
                            Range::new(34, 34),
                            Range::new(34, 34),
                            Range::new(33, 33),
                            Range::new(33, 33),
                            Range::new(32, 32),
                            Range::new(32, 32),
                            Range::new(32, 32),
                            Range::new(31, 31),
                            Range::new(31, 31),
                        ]), [
                            "=[200,200] - attack",
                            "x42 - attacker level",
                            "x40 - base power",
                            "\u{00F7}[192,192] - defense",
                            "\u{00F7}50 - constant",
                            "+2 - constant",
                            "[mapped] - fraction",
                            "[mapped] - randomize",
                            "x1 - type effectiveness",
                            "[mapped] - floor",
                        ]),
                    },
                ]),
            });
        });
    }

    #[test]
    fn multiple_hits() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                nature: Some(Nature::Serious),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Timid),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Fury Attack".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            pretty_assertions::assert_eq!(
                output.hits.iter().map(|output| output.damage.value().min_max_range()).collect::<Vec<_>>(),
                Vec::from_iter([
                    Some(Range::new(12, 15)),
                    Some(Range::new(12, 15)),
                    Some(Range::new(12, 15)),
                ])
            );
        });
    }

    #[test]
    fn spread_modifier() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                nature: Some(Nature::Serious),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Timid),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Surf".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(!damage.description().contains(&"x3/4 - spread".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(149, 176)));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                nature: Some(Nature::Serious),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Timid),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Surf".to_owned(),
                spread: true,
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x3/4 - spread".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(112, 132)));
        });
    }

    #[test]
    fn critical_hit() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                nature: Some(Nature::Serious),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                boosts: BoostTable {
                    spa: -3,
                    ..Default::default()
                },
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Timid),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                boosts: BoostTable {
                    spd: 6,
                    ..Default::default()
                },
                ..Default::default()
            },
            mov: Move {
                name: "Air Slash".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let attack = &output.hits[0].attack.as_ref().unwrap().1;
            pretty_assertions::assert_eq!(attack, &Output::new(Range::new(94, 94), [
                "x2/5 - drop",
                "[mapped] - floor",
            ]));
            let defense = &output.hits[0].defense.as_ref().unwrap().1;
            pretty_assertions::assert_eq!(defense, &Output::new(Range::new(824, 824), [
                "x4 - boost",
                "[mapped] - floor",
            ]));
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(7, 9)));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                nature: Some(Nature::Serious),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                boosts: BoostTable {
                    spa: -3,
                    ..Default::default()
                },
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Timid),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                boosts: BoostTable {
                    spd: 6,
                    ..Default::default()
                },
                ..Default::default()
            },
            mov: Move {
                name: "Air Slash".to_owned(),
                crit: true,
                ..Default::default()
            },
        }), Ok(output) => {
            let attack = &output.hits[0].attack.as_ref().unwrap().1;
            pretty_assertions::assert_eq!(attack, &Output::new(Range::new(236, 236), [
                "[mapped] - floor",
            ]));
            let defense = &output.hits[0].defense.as_ref().unwrap().1;
            pretty_assertions::assert_eq!(defense, &Output::new(Range::new(206, 206), [
                "[mapped] - floor",
            ]));
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x3/2 - crit".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(94, 111)));
        });
    }

    #[test]
    fn stab() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Pikachu".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Tackle".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(51, 60)));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Pikachu".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Aqua Jet".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x3/2 - stab".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(76, 90)));
        });
    }

    #[test]
    fn immune() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Pidgeot".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Gengar".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Tackle".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            pretty_assertions::assert_eq!(output, MultiHitDamageOutput {
                hits: Vec::from_iter([
                    DamageOutput {
                        hp: Range::new(261, 261),
                        damage: Output::new(RangeDistribution::from_iter([
                            Range::new(0, 0),
                        ]), [
                            "=[[0,0]] - immune",
                        ]),
                        ..Default::default()
                    }
                ]),
            });
        });
    }

    #[test]
    fn type_effectiveness() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Pikachu".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Pidgeot".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Thunderbolt".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let type_effectiveness = output.hits[0].type_effectiveness.as_ref().unwrap();
            pretty_assertions::assert_eq!(type_effectiveness, &Output::new(Fraction::from(2u64), [
                "x2 - super effective against Flying",
            ]));
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x2 - type effectiveness".to_owned()));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Pikachu".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Gyarados".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Thunderbolt".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let type_effectiveness = output.hits[0].type_effectiveness.as_ref().unwrap();
            pretty_assertions::assert_eq!(type_effectiveness, &Output::new(Fraction::from(4u64), [
                "x2 - super effective against Water",
                "x2 - super effective against Flying",
            ]));
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x4 - type effectiveness".to_owned()));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Ludicolo".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Gyarados".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Surf".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let type_effectiveness = output.hits[0].type_effectiveness.as_ref().unwrap();
            pretty_assertions::assert_eq!(type_effectiveness, &Output::new(Fraction::new(1u64, 2u64), [
                "x1/2 - not very effective against Water",
            ]));
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x1/2 - type effectiveness".to_owned()));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Ludicolo".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Ludicolo".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Surf".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let type_effectiveness = output.hits[0].type_effectiveness.as_ref().unwrap();
            pretty_assertions::assert_eq!(type_effectiveness, &Output::new(Fraction::new(1u64, 4u64), [
                "x1/2 - not very effective against Water",
                "x1/2 - not very effective against Grass",
            ]));
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x1/4 - type effectiveness".to_owned()));
        });
    }

    #[test]
    fn levitate_immunity() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Gengar".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Levitate".to_owned()),
                ..Default::default()
            },
            mov: Move {
                name: "Earthquake".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(0, 0)));
        });
    }

    #[test]
    fn grounded_due_to_ingrain_overrides_levitate() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Gengar".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Levitate".to_owned()),
                conditions: HashSet::from_iter(["Ingrain".to_owned()]),
                ..Default::default()
            },
            mov: Move {
                name: "Earthquake".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(185, 218)));
        });
    }

    #[test]
    fn grounded_due_to_ingrain_negates_immunity() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Golem".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Pidgeot".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                conditions: HashSet::from_iter(["Ingrain".to_owned()]),
                ..Default::default()
            },
            mov: Move {
                name: "Earthquake".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(160, 189)));
        });
    }

    #[test]
    fn burn() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                status: Some("Burn".to_owned()),
                ..Default::default()
            },
            mov: Move {
                name: "Skull Bash".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(98, 116)));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                status: Some("Burn".to_owned()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Skull Bash".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(49, 58)));
        });
    }

    #[test]
    fn rain() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field {
                weather: Some("Rain".to_owned()),
                ..Default::default()
            },
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Water Gun".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x3/2 - Rain".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(133, 157)));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field {
                weather: Some("Rain".to_owned()),
                ..Default::default()
            },
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Flamethrower".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x1/2 - Rain".to_owned()), "{damage:?}");
        });
    }

    #[test]
    fn utility_umbrella_suppresses_rain() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field {
                weather: Some("Rain".to_owned()),
                ..Default::default()
            },
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                item: Some("Utility Umbrella".to_owned()),
                ..Default::default()
            },
            mov: Move {
                name: "Water Gun".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(!damage.description().contains(&"x3/2 - Rain".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(89, 105)));
        });
    }

    #[test]
    fn embargo_suppresses_item() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field {
                weather: Some("Rain".to_owned()),
                ..Default::default()
            },
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                item: Some("Utility Umbrella".to_owned()),
                conditions: HashSet::from_iter(["Embargo".to_owned()]),
                ..Default::default()
            },
            mov: Move {
                name: "Water Gun".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x3/2 - Rain".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(133, 157)));
        });
    }

    #[test]
    fn air_lock_suppresses_rain() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field {
                weather: Some("Rain".to_owned()),
                ..Default::default()
            },
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Air Lock".to_owned()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Water Gun".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(!damage.description().contains(&"x3/2 - Rain".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(89, 105)));
        });
    }

    #[test]
    fn huge_power() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Tackle".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let attack = &output.hits[0].attack.as_ref().unwrap().1;
            assert_eq!(attack.value(), &Range::new(202, 202));
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(31, 37)));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Huge Power".to_owned()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Tackle".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let attack = &output.hits[0].attack.as_ref().unwrap().1;
            assert!(attack.description().contains(&"x2 - Huge Power".to_owned()), "{attack:?}");
            assert_eq!(attack.value(), &Range::new(404, 404));
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(61, 72)));
        });
    }

    #[test]
    fn ohko() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Golem".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Blastoise".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Fissure".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"=[[299,299]] - ohko".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(299, 299)));
        });
    }

    #[test]
    fn nature_power() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Ludicolo".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Nature Power".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let base_power = output.hits[0].base_power.as_ref().unwrap();
            assert_eq!(*base_power.value(), 80);
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(61, 72)));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field {
                environment: Some("Water".to_owned()),
                ..Default::default()
            },
            attacker: Mon {
                name: "Ludicolo".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Nature Power".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let base_power = output.hits[0].base_power.as_ref().unwrap();
            assert_eq!(*base_power.value(), 110);
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x2 - type effectiveness".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(249, 294)));
        });
    }

    #[test]
    fn psychic_terrain() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field {
                terrain: Some("Psychic Terrain".to_owned()),
                ..Default::default()
            },
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Quick Attack".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"=[[0,0]] - Psychic Terrain".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(0, 0)));
        });
    }

    #[test]
    fn volt_absorb() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Volt Absorb".to_owned()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Thunderbolt".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(161, 190)));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Volt Absorb".to_owned()),
                ..Default::default()
            },
            mov: Move {
                name: "Thunderbolt".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"=[[0,0]] - Volt Absorb".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(0, 0)));
        });
    }

    #[test]
    fn wonder_guard() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Shedinja".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Wonder Guard".to_owned()),
                ..Default::default()
            },
            mov: Move {
                name: "Water Gun".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"=[[0,0]] - Wonder Guard".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(0, 0)));
        });
    }

    #[test]
    fn soundproof() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Soundproof".to_owned()),
                ..Default::default()
            },
            mov: Move {
                name: "Uproar".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"=[[0,0]] - Soundproof".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(0, 0)));
        });
    }

    #[test]
    fn endeavor() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Endeavor".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(0, 0)));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                health: Some(Fraction::new(50, 100)),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Endeavor".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(148, 148)));
            assert_eq!(output.hits[0].hp, Range::new(297, 297));
        });
    }

    #[test]
    fn weather_ball() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Castform".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Weather Ball".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert_eq!(damage.value().min_max_range(), Some(Range::new(47, 55)));
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field {
                weather: Some("Sandstorm".to_owned()),
                ..Default::default()
            },
            attacker: Mon {
                name: "Castform".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Weather Ball".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x4 - type effectiveness".to_owned()), "{damage:?}");
            assert!(damage.description().contains(&"x100 - base power".to_owned()), "{damage:?}");
            assert_eq!(damage.value().min_max_range(), Some(Range::new(248, 292)));
        });
    }

    #[test]
    fn triple_kick() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Machamp".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Triple Kick".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            pretty_assertions::assert_eq!(
                output.hits.iter().map(|output| output.damage.value().min_max_range()).collect::<Vec<_>>(),
                Vec::from_iter([
                    Some(Range::new(8, 10)),
                    Some(Range::new(17, 20)),
                    Some(Range::new(25, 30)),
                ])
            );
        });
    }

    #[test]
    fn gem() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                item: Some("Flying Gem".to_owned()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Air Slash".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let base_power = output.hits[0].base_power.as_ref().unwrap();
            assert!(base_power.description().contains(&"x13/10 - Flying Gem".to_owned()), "{base_power:?}");
        });
    }

    #[test]
    fn type_powering_item() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                item: Some("Dragon Fang".to_owned()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Dragon Claw".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let base_power = output.hits[0].base_power.as_ref().unwrap();
            assert!(base_power.description().contains(&"x6/5 - Dragon Fang".to_owned()), "{base_power:?}");
        });
    }

    #[test]
    fn type_powering_ability() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Blaze".to_owned()),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Flamethrower".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let attack = &output.hits[0].attack.as_ref().unwrap().1;
            assert!(!attack.description().contains(&"x3/2 - Blaze".to_owned()), "{attack:?}");
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Blaze".to_owned()),
                health: Some(Fraction::new(1, 4)),
                ..Default::default()
            },
            defender: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            mov: Move {
                name: "Flamethrower".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let attack = &output.hits[0].attack.as_ref().unwrap().1;
            assert!(attack.description().contains(&"x3/2 - Blaze".to_owned()), "{attack:?}");
        });
    }

    #[test]
    fn thick_fat() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Mamoswine".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ability: Some("Thick Fat".to_owned()),
                ..Default::default()
            },
            mov: Move {
                name: "Flamethrower".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let attack = &output.hits[0].attack.as_ref().unwrap().1;
            assert!(attack.description().contains(&"x1/2 - Thick Fat".to_owned()), "{attack:?}");
        });
    }

    #[test]
    fn damage_reducing_berry() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                item: Some("Occa Berry".to_owned()),
                ..Default::default()
            },
            mov: Move {
                name: "Flamethrower".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x1/2 - Occa Berry".to_owned()), "{damage:?}");
        });
        assert_matches::assert_matches!(calculate_damage(DamageCalculatorInput {
            data: &data,
            field: Field::default(),
            attacker: Mon {
                name: "Charizard".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                ..Default::default()
            },
            defender: Mon {
                name: "Venusaur".to_owned(),
                level: 100,
                nature: Some(Nature::Hardy),
                ivs: Some(max_ivs()),
                evs: Some(empty_evs()),
                item: Some("Occa Berry".to_owned()),
                ability: Some("Ripen".to_owned()),
                ..Default::default()
            },
            mov: Move {
                name: "Flamethrower".to_owned(),
                ..Default::default()
            },
        }), Ok(output) => {
            let damage = &output.hits[0].damage;
            assert!(damage.description().contains(&"x1/2 - Occa Berry".to_owned()), "{damage:?}");
            assert!(damage.description().contains(&"x1/2 - Ripen".to_owned()), "{damage:?}");
        });
    }
}
