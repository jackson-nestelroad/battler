use std::ops::Deref;

use lazy_static::lazy_static;

use crate::{
    battle::{
        core_battle_logs,
        modify_32,
        ActiveMoveContext,
        ActiveTargetContext,
        CoreBattle,
        EffectContext,
        Mon,
        MonContext,
        MonHandle,
        MoveDamage,
        MoveHandle,
        MoveOutcome,
        Player,
        PlayerContext,
    },
    battler_error,
    common::{
        Error,
        Fraction,
        Id,
        Identifiable,
        UnsafelyDetachBorrow,
        WrapResultError,
    },
    effect::EffectHandle,
    mons::Stat,
    moves::{
        DamageType,
        MoveCategory,
        MoveTarget,
        SelfDestructType,
    },
    rng::rand_util,
};

struct HitTargetState {
    handle: MonHandle,
    damage: MoveDamage,
}

impl HitTargetState {
    pub fn new(handle: MonHandle, damage: MoveDamage) -> Self {
        Self { handle, damage }
    }
}

/// Switches a Mon into the given position.
pub fn switch_in(context: &mut MonContext, position: usize) -> Result<(), Error> {
    if context.mon_mut().active {
        core_battle_logs::hint(
            context.as_battle_context_mut(),
            "A switch failed because the Mon trying to switch in is already in.",
        )?;
        return Ok(());
    }

    let active_len = context.player().active.len();
    if position >= active_len {
        return Err(battler_error!(
            "invalid switch position {position} / {active_len}"
        ));
    }

    let prev = context
        .player()
        .active
        .get(position)
        .cloned()
        .wrap_error_with_format(format_args!(
            "expected {position} to be a valid index to active Mons"
        ))?;
    if let Some(mon) = prev {
        let mut context = context.as_battle_context_mut().mon_context(mon)?;
        context.mon_mut().switch_out();
    }
    Mon::switch_in(context, position);
    context.player_mut().active[position] = Some(context.mon_handle());

    core_battle_logs::switch(context)?;

    Ok(())
}

fn register_active_move(
    context: &mut MonContext,
    move_id: &Id,
    target: Option<MonHandle>,
) -> Result<MoveHandle, Error> {
    let mut active_move = context
        .battle_mut()
        .dex
        .moves
        .get_by_id(move_id)?
        .deref()
        .clone();
    active_move.used_by = Some(context.mon_handle());
    let active_move_handle = context.battle().register_move(active_move);
    let mon_handle = context.mon_handle();
    CoreBattle::set_active_move(
        context.as_battle_context_mut(),
        active_move_handle,
        mon_handle,
        target,
    )?;
    Ok(active_move_handle)
}

/// Executes the given move by a Mon.
pub fn do_move(
    context: &mut MonContext,
    move_id: &Id,
    target_location: Option<isize>,
    original_target: Option<MonHandle>,
    external: bool,
) -> Result<(), Error> {
    context.mon_mut().active_move_actions += 1;
    let mon_handle = context.mon_handle();
    let target = CoreBattle::get_target(
        context.as_battle_context_mut(),
        mon_handle,
        move_id,
        target_location,
        original_target,
    )?;

    // Make a copy of the move so we can work with it and modify it for the turn.
    let active_move_handle = register_active_move(context, move_id, target)?;
    context.active_move_mut()?.external = external;

    // TODO: Run BeforeMove checks.
    // TODO: Abort move if requested.

    // External moves do not have PP deducted.
    if !external {
        // TODO: Check for locked move, which will not deduct PP (think Uproar).
        let move_id = context.active_move()?.id();
        // SAFETY: move_id is only used for lookup.
        let move_id = unsafe { move_id.unsafely_detach_borrow() };
        if !context.mon_mut().deduct_pp(move_id, 1) && !move_id.eq("struggle") {
            // No PP, so this move action cannot be carried through.
            let move_name = &context.active_move()?.data.name;
            // SAFETY: Logging does not change the active move.
            let move_name = unsafe { move_name.unsafely_detach_borrow() };
            core_battle_logs::cant(context, "nopp", move_name)?;
            CoreBattle::clear_active_move(context.as_battle_context_mut())?;
            return Ok(());
        }

        // At this point, the move will be attempted, so we should remember it.
        context.mon_mut().last_move_selected = Some(active_move_handle);
        context.mon_mut().last_move_target = target_location;
    }

    // Use the move.
    let move_id = context.active_move()?.id().clone();
    let target = context.mon().active_target;
    use_move(context, &move_id, target)?;

    // TODO: AfterMove event.

    CoreBattle::faint_messages(context.as_battle_context_mut())?;
    CoreBattle::check_win(context.as_battle_context_mut())?;

    Ok(())
}

pub fn use_move(
    context: &mut MonContext,
    move_id: &Id,
    target: Option<MonHandle>,
) -> Result<bool, Error> {
    context.mon_mut().move_this_turn_outcome = None;
    let outcome = use_move_internal(context, move_id, target)?;
    context.mon_mut().move_this_turn_outcome = Some(outcome);
    Ok(outcome.into())
}

fn use_move_internal(
    context: &mut MonContext,
    move_id: &Id,
    mut target: Option<MonHandle>,
) -> Result<MoveOutcome, Error> {
    // This move becomes the active move.
    let active_mon_handle = register_active_move(context, move_id, target)?;
    let mut context = context.active_move_context()?;
    context.mon_mut().last_move_used = Some(active_mon_handle);
    let base_target = context.active_move().data.target.clone();
    // TODO: ModifyTarget event.
    let mon_handle = context.mon_handle();
    if target.is_none() && context.active_move().data.target.requires_target() {
        target = CoreBattle::random_target(context.as_battle_context_mut(), mon_handle, move_id)?;
    }
    // Target may have been modified, so update the battle and context.
    let active_move_handle = context.active_move_handle();
    context
        .mon_mut()
        .set_active_move(active_move_handle, target);

    let mut context = context.active_move_context()?;
    // TODO: ModifyType.
    // TODO: ModifyMove.

    // The target changed, so it must be adjusted here.
    if base_target != context.active_move().data.target {
        target = CoreBattle::random_target(context.as_battle_context_mut(), mon_handle, move_id)?;
    }

    // Mon fainted before this move could be made.
    if context.mon().fainted {
        return Ok(MoveOutcome::Failed);
    }

    // Log that the move is being used.
    let move_name = &context.active_move().data.name;
    // SAFETY: Logging does not change the active move.
    let move_name = unsafe { move_name.unsafely_detach_borrow() };
    core_battle_logs::use_move(context.as_mon_context_mut(), move_name, target)?;

    if context.mon().active_target.is_none() && context.active_move().data.target.requires_target()
    {
        core_battle_logs::last_move_had_no_target(context.as_battle_context_mut());
        core_battle_logs::fail(context.as_mon_context_mut())?;
        return Ok(MoveOutcome::Failed);
    }

    let targets = get_move_targets(&mut context, target)?;
    // TODO: DeductPP event (for Pressure).
    // TODO: Targeted event.
    // TODO: TryMove event.
    // TODO: UseMoveMessage event.

    if context.active_move().data.self_destruct == Some(SelfDestructType::Always) {
        let mon_handle = context.mon_handle();
        let effect_handle = context.effect_handle();
        faint(
            context.as_mon_context_mut(),
            Some(mon_handle),
            Some(&effect_handle),
        )?;
    }

    let outcome = if !context.active_move().data.target.affects_mons_directly() {
        todo!("moves that do not affect Mons directly are not implemented")
    } else {
        if targets.is_empty() {
            core_battle_logs::last_move_had_no_target(context.as_battle_context_mut());
            core_battle_logs::fail(context.as_mon_context_mut())?;
            return Ok(MoveOutcome::Failed);
        }
        try_direct_move(&mut context, &targets)?
    };

    // TODO: Move hit on self for boosts?

    if context.mon().hp == 0 {
        let mon_handle = context.mon_handle();
        let effect_handle = context.effect_handle();
        faint(
            context.as_mon_context_mut(),
            Some(mon_handle),
            Some(&effect_handle),
        )?;
    }

    // TODO: MoveFail event if outcome is Failed.

    Ok(outcome)
}

pub fn faint(
    context: &mut MonContext,
    source: Option<MonHandle>,
    effect: Option<&EffectHandle>,
) -> Result<(), Error> {
    Mon::faint(context, source, effect)
}

pub fn get_move_targets(
    context: &mut ActiveMoveContext,
    target: Option<MonHandle>,
) -> Result<Vec<MonHandle>, Error> {
    let mut targets = Vec::new();
    match context.active_move().data.target {
        MoveTarget::All => {
            targets.extend(
                context
                    .battle()
                    .active_mon_handles_on_side(context.side().index),
            );
            targets.extend(
                context
                    .battle()
                    .active_mon_handles_on_side(context.foe_side().index),
            );
        }
        MoveTarget::FoeSide => {
            targets.extend(
                context
                    .battle()
                    .active_mon_handles_on_side(context.foe_side().index),
            );
        }
        MoveTarget::AllySide => {
            targets.extend(
                context
                    .battle()
                    .active_mon_handles_on_side(context.side().index),
            );
        }
        MoveTarget::AllyTeam => {
            targets.extend(
                context
                    .battle()
                    .all_mon_handles_on_side(context.side().index),
            );
        }
        MoveTarget::AllAdjacent => {
            targets.extend(
                Mon::adjacent_allies(&mut context.as_mon_context_mut())?.filter_map(|mon| mon),
            );
            targets.extend(
                Mon::adjacent_foes(&mut context.as_mon_context_mut())?.filter_map(|mon| mon),
            );
        }
        MoveTarget::AllAdjacentFoes => {
            targets.extend(
                Mon::adjacent_foes(&mut context.as_mon_context_mut())?.filter_map(|mon| mon),
            );
        }
        MoveTarget::Allies => {
            targets.extend(
                Mon::adjacent_allies_and_self(&mut context.as_mon_context_mut())?
                    .filter_map(|mon| mon),
            );
        }
        _ => {
            let mut target = target;
            if let Some(possible_target) = target {
                let mon = context.mon_handle();
                let target_context = context.target_mon_context(possible_target)?;
                if target_context.mon().fainted
                    && !target_context
                        .mon()
                        .is_ally(target_context.as_battle_context().mon(mon)?)
                {
                    // A targeted Mon has fainted, so the move should retarget.
                    let mon = context.mon_handle();
                    let active_move = context.active_move().id().clone();
                    target = CoreBattle::random_target(
                        context.as_battle_context_mut(),
                        mon,
                        &active_move,
                    )?;
                }
            }

            if let Some(target) = target {
                targets.push(target);
            }
        }
    }
    Ok(targets)
}

fn try_direct_move(
    context: &mut ActiveMoveContext,
    targets: &[MonHandle],
) -> Result<MoveOutcome, Error> {
    if targets.len() > 1 && !context.active_move().data.smart_target {
        context.active_move_mut().spread_hit = true;
    }

    // TODO: Try event.
    // TODO: PrepareHit event.
    // TODO: Potentially fail move early.

    lazy_static! {
        static ref STEPS: Vec<direct_move_step::DirectMoveStep> = vec![
            direct_move_step::check_targets_invulnerability,
            direct_move_step::check_try_hit_event,
            direct_move_step::check_type_immunity,
            direct_move_step::check_general_immunity,
            direct_move_step::handle_accuracy,
            direct_move_step::break_protect,
            // TODO: Boost stealing would happen at this stage.
            direct_move_step::move_hit_loop,
        ];
    }

    // TODO: Try event for the move.
    // TODO: PrepareHit event.
    // TODO: Fail the move early if needed.

    let mut targets = targets
        .iter()
        .map(|target| direct_move_step::MoveStepTarget {
            handle: *target,
            outcome: MoveOutcome::Success,
        })
        .collect::<Vec<_>>();
    // We can lose targets without explicit failures.
    let mut at_least_one_failure = false;
    for step in &*STEPS {
        step(context, targets.as_mut_slice())?;
        at_least_one_failure = at_least_one_failure
            || targets
                .iter()
                .any(|target| target.outcome == MoveOutcome::Failed);
        targets = targets
            .into_iter()
            .filter(|target| target.outcome.success())
            .collect();
        if targets.is_empty() {
            break;
        }
    }

    let outcome = if targets.is_empty() {
        if at_least_one_failure {
            MoveOutcome::Failed
        } else {
            MoveOutcome::Skipped
        }
    } else {
        MoveOutcome::Success
    };

    if context.active_move().spread_hit {
        core_battle_logs::last_move_spread_targets(
            context.as_battle_context_mut(),
            targets.into_iter().map(|target| target.handle),
        )?;
    }

    Ok(outcome)
}

fn move_hit_loose_success(
    context: &mut ActiveMoveContext,
    targets: &[MonHandle],
) -> Result<MoveOutcome, Error> {
    let targets = move_hit(context, targets)?;
    if targets.into_iter().all(|target| target.damage.failed()) {
        Ok(MoveOutcome::Skipped)
    } else {
        Ok(MoveOutcome::Success)
    }
}

fn move_hit(
    context: &mut ActiveMoveContext,
    targets: &[MonHandle],
) -> Result<Vec<HitTargetState>, Error> {
    let mut hit_targets_state = targets
        .iter()
        .map(|target| HitTargetState::new(*target, MoveDamage::None))
        .collect::<Vec<_>>();
    hit_targets(context, hit_targets_state.as_mut_slice())?;
    Ok(hit_targets_state)
}

fn hit_targets(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    let move_target = context.active_move().data.target.clone();
    if move_target == MoveTarget::All && !context.is_self() {
        // TODO: TryHitField event for the HitEffect.
    } else if (move_target == MoveTarget::FoeSide
        || move_target == MoveTarget::AllySide
        || move_target == MoveTarget::AllyTeam)
        && !context.is_self()
    {
        // TODO: TryHitSide event for the HitEffect.
    } else {
        // TODO: TryHit event for the HitEffect.
    }

    // TODO: If any of the above events fail, the move should fail.
    // TODO: If we run multiple TryHit events for multiple targets, the targets hit should be
    // filtered.

    // First, check for substitute.
    if !context.is_secondary() && !context.is_self() && move_target.affects_mons_directly() {
        // TODO: TryPrimaryHit event, which should catch substitutes.
    }

    // TODO: If we hit a substitute, filter those targets out.

    // Calculate damage for each target.
    calculate_spread_damage(context, targets)?;
    for target in targets.iter_mut() {
        if target.damage.failed() {
            if !context.is_secondary() && !context.is_self() {
                core_battle_logs::fail_target(&mut context.target_context(target.handle)?)?;
            }
        }
    }

    let mon_handle = context.mon_handle();
    apply_spread_damage(&mut context.effect_context()?, Some(mon_handle), targets)?;

    // TODO: Run move effects.

    // TODO: Self drops.

    // TODO: Secondary effects.

    // TODO: Force switch.

    // TODO: Post-damage events.

    Ok(())
}

fn calculate_spread_damage(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    for target in targets {
        if target.damage.failed() {
            continue;
        }
        target.damage = MoveDamage::None;
        // Secondary or effects on the user cannot deal damage.
        //
        // Note that this is different from moves that target the user.
        if context.is_secondary() || context.is_self() {
            continue;
        }
        CoreBattle::set_active_target(context.as_battle_context_mut(), Some(target.handle))?;
        let mut context = context.active_target_context()?;
        target.damage = calculate_damage(&mut context)?;
    }
    Ok(())
}

fn calculate_damage(context: &mut ActiveTargetContext) -> Result<MoveDamage, Error> {
    let target_mon_handle = context.target_mon_handle();
    // Type immunity.
    let move_type = context.active_move().data.primary_type;
    let ignore_immunity = context.active_move().data.ignore_immunity();
    if !ignore_immunity && Mon::is_immune(&mut context.target_mon_context()?, move_type)? {
        return Ok(MoveDamage::Failure);
    }

    // OHKO.
    if context.active_move().data.ohko_type.is_some() {
        return Ok(MoveDamage::Damage(context.target_mon().max_hp));
    }

    // TODO: Damage callback for moves that have special rules for damage calculation.

    // Static damage.
    match context.active_move().data.damage {
        Some(DamageType::Level) => return Ok(MoveDamage::Damage(context.mon().level as u16)),
        Some(DamageType::Set(damage)) => return Ok(MoveDamage::Damage(damage)),
        _ => (),
    }

    let base_power = context.active_move().data.base_power;
    // TODO: Base power callback for moves that have special rules for base power calculation.

    // If base power is explicitly 0, no damage should be dealt.
    //
    // Status moves stop here.
    if base_power == 0 {
        return Ok(MoveDamage::None);
    }
    let base_power = context.active_move().data.base_power.max(1);

    // Critical hit.
    // TODO: ModifyCritRatio event.
    let crit_ratio = context.active_move().data.crit_ratio.unwrap_or(0);
    let crit_ratio = crit_ratio.max(0).min(4);
    let crit_mult = [0, 24, 8, 2, 1];
    context.active_move_mut().hit_data(target_mon_handle).crit =
        context.active_move().data.will_crit
            || (crit_ratio > 0
                && rand_util::chance(
                    context.battle_mut().prng.as_mut(),
                    1,
                    crit_mult[crit_ratio as usize],
                ));

    if context.active_move_mut().hit_data(target_mon_handle).crit {
        // TODO: CriticalHit event.
    }

    // TODO: BasePower event, which happens after crit calculation.

    let level = context.mon().level;
    let move_category = context.active_move().data.category.clone();
    let is_physical = move_category == MoveCategory::Physical;
    let attack_stat = context
        .active_move()
        .data
        .override_offensive_stat
        .unwrap_or(if is_physical { Stat::Atk } else { Stat::SpAtk });
    let defense_stat = context
        .active_move()
        .data
        .override_defensive_stat
        .unwrap_or(if is_physical { Stat::Def } else { Stat::SpDef });

    let mut attack_boosts = context
        .attacker_context()?
        .mon()
        .boosts
        .get(attack_stat.try_into()?);
    let mut defense_boosts = context
        .defender_context()?
        .mon()
        .boosts
        .get(defense_stat.try_into()?);

    if context.active_move().data.ignore_offensive {
        attack_boosts = 0;
    }
    if context.active_move().data.ignore_defensive {
        defense_boosts = 0;
    }

    let move_user = context.mon_handle();
    let move_target = context.target_mon_handle();
    let attack = Mon::calculate_stat(
        &mut context.attacker_context()?,
        attack_stat,
        attack_boosts,
        Fraction::from(1),
        move_user,
        move_target,
    )?;
    let defense = Mon::calculate_stat(
        &mut context.defender_context()?,
        defense_stat,
        defense_boosts,
        Fraction::from(1),
        move_target,
        move_user,
    )?;

    let base_damage = 2 * (level as u32) / 5 + 2;
    let base_damage = base_damage * base_power * (attack as u32);
    let base_damage = base_damage / (defense as u32);
    let base_damage = base_damage / 50;

    // Damage modifiers.
    modify_damage(context, base_damage)
}

fn modify_damage(
    context: &mut ActiveTargetContext,
    mut base_damage: u32,
) -> Result<MoveDamage, Error> {
    base_damage += 2;
    if context.active_move().spread_hit {
        let spread_modifier = Fraction::new(3, 4);
        base_damage = modify_32(base_damage, spread_modifier);
    }

    // TODO: WeatherModifyDamage event.

    // Critical hit.
    let target_mon_handle = context.target_mon_handle();
    let crit = context.active_move_mut().hit_data(target_mon_handle).crit;
    if crit {
        let crit_modifier = Fraction::new(3, 2);
        base_damage = modify_32(base_damage, crit_modifier);
    }

    // Randomize damage.
    base_damage = context.battle_mut().randomize_base_damage(base_damage);

    // STAB.
    let move_type = context.active_move().data.primary_type;
    let stab = Mon::has_type(&context.as_mon_context(), move_type)?;
    if stab {
        let stab_modifier = context
            .active_move()
            .clone()
            .stab_modifier
            .unwrap_or(Fraction::new(3, 2));
        base_damage = modify_32(base_damage, stab_modifier);
    }

    // Type effectiveness.
    let type_modifier = Mon::type_effectiveness(&mut context.target_mon_context()?, move_type)?;
    let type_modifier = type_modifier.max(-6).min(6);
    context
        .active_move_mut()
        .hit_data(target_mon_handle)
        .type_modifier = type_modifier;
    if type_modifier > 0 {
        core_battle_logs::super_effective(context)?;
        for _ in 0..type_modifier {
            base_damage *= 2;
        }
    } else if type_modifier < 0 {
        core_battle_logs::resisted(context)?;
        for _ in 0..-type_modifier {
            base_damage /= 2;
        }
    }

    if crit {
        core_battle_logs::critical_hit(context)?;
    }

    // TODO: StatusModifyDamage event (Burn).

    // TODO: ModifyDamage event.

    let base_damage = base_damage as u16;
    let base_damage = base_damage.max(1);
    Ok(MoveDamage::Damage(base_damage))
}

fn calculate_recoil_damage(context: &ActiveMoveContext) -> u64 {
    let damage_dealt = context.active_move().total_damage;
    match context.active_move().data.recoil_percent {
        Some(recoil_percent) if damage_dealt > 0 => {
            (recoil_percent.convert() * damage_dealt).round().max(1)
        }
        _ => 0,
    }
}

mod direct_move_step {
    use std::ops::Mul;

    use crate::{
        battle::{
            core_battle_actions,
            core_battle_logs,
            ActiveMoveContext,
            ActiveTargetContext,
            CoreBattle,
            Mon,
            MonHandle,
            MoveDamage,
            MoveOutcome,
        },
        common::{
            Error,
            Fraction,
            Id,
            WrapResultError,
        },
        effect::EffectHandle,
        moves::{
            Accuracy,
            MoveCategory,
            MoveTarget,
            MultihitType,
            OhkoType,
        },
        rng::rand_util,
    };

    pub struct MoveStepTarget {
        pub handle: MonHandle,
        pub outcome: MoveOutcome,
    }

    pub type DirectMoveStep =
        fn(&mut ActiveMoveContext, &mut [MoveStepTarget]) -> Result<(), Error>;

    pub fn check_targets_invulnerability(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        for target in targets {
            // TODO: Invulnerability event.
            // should_continue = false if invulnerable.
        }
        Ok(())
    }

    pub fn check_try_hit_event(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        for target in targets.iter_mut() {
            // TODO: TryHit event.
            // should_continue = false if failed.
        }
        if targets
            .iter()
            .all(|target| target.outcome == MoveOutcome::Failed)
        {
            core_battle_logs::do_not_animate_last_move(context.as_battle_context_mut());
            core_battle_logs::fail(context.as_mon_context_mut())?;
        }
        Ok(())
    }

    pub fn check_type_immunity(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        let move_type = context.active_move().data.primary_type;
        for target in targets {
            // TODO: IgnoreImmunity event for the move (Thousand Arrows has a special rule).
            let immune = !context.active_move().data.ignore_immunity();
            let mut target_context = context.target_mon_context(target.handle)?;
            let immune = immune && Mon::is_immune(&mut target_context, move_type)?;
            if immune {
                core_battle_logs::immune(&mut target_context)?;
                target.outcome = MoveOutcome::Failed;
            }
        }
        Ok(())
    }

    pub fn check_general_immunity(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        for target in targets {
            // TODO: Check for powder immunity.
            // TODO: TryImmunity event.
            // TODO: Prankster immunity.
        }
        Ok(())
    }

    pub fn handle_accuracy(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        for target in targets {
            let mut context = context.target_context(target.handle)?;
            if !accuracy_check(&mut context)? {
                if !context.active_move().spread_hit {
                    core_battle_logs::last_move_had_no_target(context.as_battle_context_mut());
                }
                core_battle_logs::miss(&mut context)?;
                target.outcome = MoveOutcome::Failed;
                // TODO: AccuracyFailure event.
            }
        }
        Ok(())
    }

    fn accuracy_check(context: &mut ActiveTargetContext) -> Result<bool, Error> {
        let mut accuracy = context.active_move().data.accuracy;
        // OHKO moves bypass accuracy modifiers.
        if let Some(ohko) = context.active_move().data.ohko_type.clone() {
            // TODO: Skip if target is semi-invulnerable.
            let mut immune = context.mon().level >= context.target_mon().level;
            if let OhkoType::Type(typ) = ohko {
                if Mon::has_type(&context.target_mon_context()?, typ)? {
                    immune = true;
                }
            }

            if immune {
                core_battle_logs::immune(&mut context.target_mon_context()?)?;
                return Ok(false);
            }

            if let Accuracy::Chance(accuracy) = &mut accuracy {
                *accuracy += context.mon().level - context.target_mon().level;
            }
        } else {
            // TODO: ModifyAccuracy event.
            if let Accuracy::Chance(accuracy) = &mut accuracy {
                let mut boost = 0;
                if !context.active_move().data.ignore_accuracy {
                    // TODO: ModifyBoost event.
                    boost = context.mon().boosts.acc.max(-6).min(6);
                }
                if !context.active_move().data.ignore_evasion {
                    // TODO: ModifyBoost event.
                    boost = (boost - context.target_mon_context()?.mon().boosts.eva)
                        .max(-6)
                        .min(6);
                }
                let multiplier = if boost > 0 {
                    Fraction::new((3 + boost) as u16, 3)
                } else {
                    Fraction::new(3, (3 - boost) as u16)
                };
                *accuracy = multiplier.mul(*accuracy as u16).floor() as u8;
            }
        }

        if context.active_move().data.target == MoveTarget::User
            && context.active_move().data.category == MoveCategory::Status
        {
            // TODO: If also not semi-invulnerable, accuracy is always.
        }

        // TODO: Accuracy event.
        match accuracy {
            Accuracy::Chance(accuracy) => Ok(rand_util::chance(
                context.battle_mut().prng.as_mut(),
                accuracy as u64,
                100,
            )),
            _ => Ok(true),
        }
    }

    pub fn break_protect(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        if context.active_move().data.breaks_protect {
            for target in targets {
                // TODO: Break protect volatile conditions.
            }
        }
        Ok(())
    }

    pub fn move_hit_loop(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        context.active_move_mut().total_damage = 0;

        let hits = match context.active_move().data.multihit {
            None => 1,
            Some(MultihitType::Static(hits)) => hits,
            Some(MultihitType::Range(min, max)) => {
                if min == 2 && max == 5 {
                    // 35-35-15-15 for 2-3-4-5 hits.
                    *rand_util::sample_slice(
                        context.battle_mut().prng.as_mut(),
                        &[2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 5, 5, 5],
                    )
                    .wrap_error()?
                } else {
                    rand_util::range(context.battle_mut().prng.as_mut(), min as u64, max as u64)
                        as u8
                }
            }
        };

        // TODO: Consider Loaded Dice item.

        for hit in 0..hits {
            // No more targets.
            if targets.iter().all(|target| !target.outcome.success()) {
                break;
            }

            // Record number of hits.
            context.active_move_mut().hit = hit + 1;

            // Of all the eligible targets, determine which ones we will actually hit.
            let mut next_hit_targets = Vec::with_capacity(targets.len());
            for target in targets.iter_mut().filter(|target| target.outcome.success()) {
                let mut context = context.target_context(target.handle)?;
                if context.active_move().data.multiaccuracy && hit > 1 {
                    if !accuracy_check(&mut context)? {
                        target.outcome = MoveOutcome::Failed;
                        continue;
                    }
                }

                // If we made it this far, the target is eligible for another hit.
                next_hit_targets.push(target);
            }

            let mut hit_targets = targets
                .iter()
                .filter_map(|target| target.outcome.success().then_some(target.handle))
                .collect::<Vec<_>>();
            let hit_targets_state =
                core_battle_actions::move_hit(context, hit_targets.as_mut_slice())?;

            if hit_targets_state
                .iter()
                .all(|target| target.damage.failed())
            {
                break;
            }

            context.active_move_mut().total_damage += hit_targets_state
                .iter()
                .filter_map(|target| {
                    if let MoveDamage::Damage(damage) = target.damage {
                        Some(damage as u64)
                    } else {
                        None
                    }
                })
                .sum::<u64>();

            // TODO: Update event for everything on the field, like items.
        }

        CoreBattle::faint_messages(context.as_battle_context_mut())?;

        let hits = context.active_move().hit;
        if context.active_move().data.multihit.is_some() {
            core_battle_logs::hit_count(context, hits)?;
        }

        let recoil_damage = core_battle_actions::calculate_recoil_damage(context);
        if recoil_damage > 0 {
            let recoil_damage = recoil_damage.min(u16::MAX as u64) as u16;
            let mon_handle = context.mon_handle();
            let effect_handle = context.effect_handle();
            core_battle_actions::damage(
                &mut context.as_mon_context_mut(),
                recoil_damage,
                Some(mon_handle),
                Some(&effect_handle),
            )?;
        }

        if context.active_move().data.struggle_recoil {
            let recoil_damage = Fraction::new(context.mon().max_hp, 4).round();
            let mon_handle = context.mon_handle();
            core_battle_actions::direct_damage(
                &mut context.as_mon_context_mut(),
                recoil_damage,
                Some(mon_handle),
                Some(&EffectHandle::Condition(Id::from_known("strugglerecoil"))),
            )?;
        }

        // TODO: Record which Mon attacked which, and how many times.

        // Log OHKOs.
        for target in targets.iter() {
            let mut context = context.target_context(target.handle)?;
            if context.active_move().data.ohko_type.is_some() && context.target_mon().hp == 0 {
                core_battle_logs::ohko(&mut context)?;
            }
        }

        // At this point, all hits have been applied.
        Ok(())
    }
}

fn direct_damage(
    context: &mut MonContext,
    damage: u16,
    source: Option<MonHandle>,
    effect: Option<&EffectHandle>,
) -> Result<u16, Error> {
    if context.mon().hp == 0 || damage == 0 {
        return Ok(0);
    }
    let damage = damage.max(1);
    let damage = Mon::damage(context, damage, source, effect)?;
    core_battle_logs::damage(context, source, effect)?;
    if context.mon().fainted {
        faint(context, source, effect)?;
    }
    Ok(damage)
}

fn damage(
    context: &mut MonContext,
    damage: u16,
    source: Option<MonHandle>,
    effect: Option<&EffectHandle>,
) -> Result<(), Error> {
    let target = context.mon_handle();
    let mut context = match effect {
        None => return Err(battler_error!("damage dealt must be tied to some effect")),
        Some(effect) => context.as_battle_context_mut().effect_context(effect)?,
    };
    let mut targets = [HitTargetState::new(target, MoveDamage::Damage(damage))];
    apply_spread_damage(&mut context, source, &mut targets)
}

fn apply_spread_damage(
    context: &mut EffectContext,
    source: Option<MonHandle>,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    for target in targets {
        let mut context = context.applying_effect_context(source, target.handle)?;
        let damage = match &mut target.damage {
            MoveDamage::Failure | MoveDamage::None | MoveDamage::Damage(0) => continue,
            MoveDamage::Damage(damage) => damage,
        };
        if context.target().hp == 0 {
            target.damage = MoveDamage::Damage(0);
            continue;
        }
        if !context.target().active {
            target.damage = MoveDamage::Failure;
            continue;
        }
        // TODO: Struggle recoil should not be affected by effects.
        // TODO: Check weather immunity.
        // TODO: Run Damage event, which can cause damage to fail.

        let source_handle = context.source_handle();
        let effect_handle = context.effect_handle();
        *damage = Mon::damage(
            &mut context.target_context()?,
            *damage,
            source_handle,
            Some(&effect_handle),
        )?;
        context.target_mut().hurt_this_turn = *damage;

        let source_handle = context.source_handle();
        let effect_handle = context.effect_handle();
        core_battle_logs::damage(
            &mut context.target_context()?,
            source_handle,
            Some(&effect_handle),
        )?;

        if let Some(Some(drain_percent)) = context
            .effect()
            .active_move()
            .map(|active_move| active_move.data.drain_percent)
        {
            let target_handle = context.target_handle();
            if let Some(mut context) = context.source_context()? {
                let amount = drain_percent * *damage;
                let amount = amount.round();
                heal(
                    &mut context,
                    amount,
                    Some(target_handle),
                    Some(&EffectHandle::Condition(Id::from_known("drain"))),
                )?;
            }
        }
    }
    Ok(())
}

pub fn heal(
    context: &mut MonContext,
    damage: u16,
    source: Option<MonHandle>,
    effect: Option<&EffectHandle>,
) -> Result<u16, Error> {
    // TODO: TryHeal event.
    if damage == 0
        || context.mon().hp == 0
        || !context.mon().active
        || context.mon().hp > context.mon().max_hp
    {
        return Ok(0);
    }

    let healed = Mon::heal(context, damage)?;
    core_battle_logs::heal(context, source, effect)?;
    // TODO: Heal event.
    Ok(healed)
}

pub fn drag_in(context: &mut PlayerContext, position: usize) -> Result<bool, Error> {
    let old = Player::active_mon_handle(context, position);

    let old_context = match old {
        None => return Err(battler_error!("nothing to drag out")),
        Some(old) => context.mon_context(old)?,
    };
    if old_context.mon().hp == 0 {
        return Ok(false);
    }
    // TODO: DragOut event.

    let player = context.player().index;
    let mon = CoreBattle::random_switchable(context.as_battle_context_mut(), player)?;
    let mut context = match mon {
        None => return Ok(false),
        Some(mon) => context.mon_context(mon)?,
    };
    if context.mon().active {
        return Ok(false);
    }
    switch_in(&mut context, position)?;
    Ok(true)
}

fn apply_move_effects(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    Ok(())
}

fn apply_self_effect(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    Ok(())
}

fn apply_secondary_effects(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    Ok(())
}

fn force_switch(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    Ok(())
}
