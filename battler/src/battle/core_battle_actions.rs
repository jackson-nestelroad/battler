use std::ops::Deref;

use lazy_static::lazy_static;

use crate::{
    battle::{
        core_battle_effects,
        core_battle_logs,
        modify_32,
        ActiveMoveContext,
        ActiveTargetContext,
        ApplyingEffectContext,
        BoostMapInOrderIterator,
        CoreBattle,
        EffectContext,
        Mon,
        MonContext,
        MonHandle,
        MoveHandle,
        MoveOutcome,
        MoveOutcomeOnTarget,
        PartialBoostTable,
        Player,
        PlayerContext,
        Side,
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
    effect::{
        fxlang,
        EffectHandle,
        EffectType,
    },
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
    outcome: MoveOutcomeOnTarget,
}

impl HitTargetState {
    pub fn new(handle: MonHandle, outcome: MoveOutcomeOnTarget) -> Self {
        Self { handle, outcome }
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

    core_battle_effects::run_event_for_mon(context, fxlang::BattleEvent::SwitchIn);

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
    source_effect: Option<EffectHandle>,
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

    // BeforeMove event handlers can prevent the move from being used.
    if !core_battle_effects::run_event_for_applying_effect(
        &mut context
            .active_move_context()?
            .user_applying_effect_context()?,
        fxlang::BattleEvent::BeforeMove,
        fxlang::VariableInput::default(),
    ) {
        core_battle_effects::run_event_for_applying_effect(
            &mut context
                .active_move_context()?
                .user_applying_effect_context()?,
            fxlang::BattleEvent::MoveAborted,
            fxlang::VariableInput::default(),
        );
        CoreBattle::clear_active_move(context.as_battle_context_mut())?;
        context.mon_mut().move_this_turn_outcome = Some(MoveOutcome::Failed);
        return Ok(());
    }

    // External moves do not have PP deducted.
    if !external {
        let locked_move = Mon::locked_move(context)?;
        let move_id = context.active_move()?.id();
        // SAFETY: move_id is only used for lookup.
        let move_id = unsafe { move_id.unsafely_detach_borrow() };
        if locked_move.is_none()
            && !context.mon_mut().deduct_pp(move_id, 1)
            && !move_id.eq("struggle")
        {
            // No PP, so this move action cannot be carried through.
            let move_name = &context.active_move()?.data.name;
            // SAFETY: Logging does not change the active move.
            let move_name = unsafe { move_name.unsafely_detach_borrow() };
            core_battle_logs::cant(context, "nopp", Some(move_name))?;
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
    use_move(context, &move_id, target, source_effect)?;

    // TODO: AfterMove event.

    CoreBattle::faint_messages(context.as_battle_context_mut())?;
    CoreBattle::check_win(context.as_battle_context_mut())?;

    Ok(())
}

pub fn use_move(
    context: &mut MonContext,
    move_id: &Id,
    target: Option<MonHandle>,
    source_effect: Option<EffectHandle>,
) -> Result<bool, Error> {
    context.mon_mut().move_this_turn_outcome = None;
    let outcome = use_move_internal(context, move_id, target, source_effect)?;
    context.mon_mut().move_this_turn_outcome = Some(outcome);
    Ok(outcome.into())
}

fn use_move_internal(
    context: &mut MonContext,
    move_id: &Id,
    mut target: Option<MonHandle>,
    source_effect: Option<EffectHandle>,
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

    context.active_move_mut().source_effect = source_effect;

    // Target may have been modified, so update the battle and context.
    let active_move_handle = context.active_move_handle();
    context
        .mon_mut()
        .set_active_move(active_move_handle, target);

    let mut context = context.active_move_context()?;

    // TODO: ModifyType on the move.
    core_battle_effects::run_active_move_event_expecting_void(
        &mut context,
        fxlang::BattleEvent::UseMove,
    );

    // The target changed, so it must be adjusted here.
    if base_target != context.active_move().data.target {
        target = CoreBattle::random_target(context.as_battle_context_mut(), mon_handle, move_id)?;
    }

    // TODO: ModifyType events on the Mon.
    core_battle_effects::run_event_for_applying_effect(
        &mut context.user_applying_effect_context()?,
        fxlang::BattleEvent::UseMove,
        fxlang::VariableInput::default(),
    );

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
    core_battle_effects::run_active_move_event_expecting_void(
        &mut context,
        fxlang::BattleEvent::UseMoveMessage,
    );

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
    let targets = targets
        .into_iter()
        .filter(|target| {
            context
                .as_battle_context()
                .mon(*target)
                .is_ok_and(|target| target.hp > 0)
        })
        .collect();
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

    let move_event_result = core_battle_effects::run_active_move_event_expecting_move_event_result(
        context,
        fxlang::BattleEvent::TryUseMove,
    );
    if !move_event_result.advance() {
        if move_event_result.failed() {
            core_battle_logs::fail(context.as_mon_context_mut())?;
            core_battle_logs::do_not_animate_last_move(context.as_battle_context_mut());
            return Ok(MoveOutcome::Failed);
        }
        return Ok(MoveOutcome::Success);
    }

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
    if targets.into_iter().all(|target| target.outcome.failed()) {
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
        .map(|target| HitTargetState::new(*target, MoveOutcomeOnTarget::Success))
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
        if target.outcome.failed() {
            if !context.is_secondary() && !context.is_self() {
                core_battle_logs::fail_target(&mut context.target_context(target.handle)?)?;
            }
        }
    }

    // Apply damage for the move to all targets.
    let mon_handle = context.mon_handle();
    apply_spread_damage(&mut context.effect_context()?, Some(mon_handle), targets)?;

    // Apply all other move effects that occur when a target is hit.
    apply_move_effects(context, targets)?;

    // Apply the effects against the user of the move.
    if !context.is_self() {
        apply_user_effect(context, targets)?;
    }

    if !context.is_secondary() && !context.is_self() {
        // Apply secondary effects.
        apply_secondary_effects(context, targets)?;
    }

    // Force switch out targets that were hit, as necessary.
    force_switch(context, targets)?;

    for target in targets.iter().filter(|target| target.outcome.damage() > 0) {
        core_battle_effects::run_event_for_applying_effect(
            &mut context.applying_effect_context_for_target(target.handle)?,
            fxlang::BattleEvent::DamagingHit,
            fxlang::VariableInput::from_iter([fxlang::Value::U16(target.outcome.damage())]),
        );
    }

    // TODO: Post-damage events.

    Ok(())
}

fn calculate_spread_damage(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    for target in targets {
        if target.outcome.failed() {
            continue;
        }
        target.outcome = MoveOutcomeOnTarget::Success;
        // Secondary or effects on the user cannot deal damage.
        //
        // Note that this is different from moves that target the user.
        if context.is_secondary() || context.is_self() {
            continue;
        }
        CoreBattle::set_active_target(context.as_battle_context_mut(), Some(target.handle))?;
        let mut context = context.active_target_context()?;
        target.outcome = calculate_damage(&mut context)?;
    }
    Ok(())
}

fn calculate_damage(context: &mut ActiveTargetContext) -> Result<MoveOutcomeOnTarget, Error> {
    let target_mon_handle = context.target_mon_handle();
    // Type immunity.
    let move_type = context.active_move().data.primary_type;
    let ignore_immunity = context.active_move().data.ignore_immunity();
    if !ignore_immunity && Mon::is_immune(&mut context.target_mon_context()?, move_type)? {
        return Ok(MoveOutcomeOnTarget::Failure);
    }

    // OHKO.
    if context.active_move().data.ohko_type.is_some() {
        return Ok(MoveOutcomeOnTarget::Damage(context.target_mon().max_hp));
    }

    // TODO: Damage callback for moves that have special rules for damage calculation.

    // Static damage.
    match context.active_move().data.damage {
        Some(DamageType::Level) => {
            return Ok(MoveOutcomeOnTarget::Damage(context.mon().level as u16))
        }
        Some(DamageType::Set(damage)) => return Ok(MoveOutcomeOnTarget::Damage(damage)),
        _ => (),
    }

    let base_power = context.active_move().data.base_power;
    // TODO: Base power callback for moves that have special rules for base power calculation.

    // If base power is explicitly 0, no damage should be dealt.
    //
    // Status moves stop here.
    if base_power == 0 {
        return Ok(MoveOutcomeOnTarget::Success);
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

    let ignore_offensive = context.active_move().data.ignore_offensive
        || context.active_move_mut().hit_data(target_mon_handle).crit;
    let ignore_defensive = context.active_move().data.ignore_defensive
        || context.active_move_mut().hit_data(target_mon_handle).crit;

    if ignore_offensive {
        attack_boosts = 0;
    }
    if ignore_defensive {
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
) -> Result<MoveOutcomeOnTarget, Error> {
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

    base_damage = core_battle_effects::run_event_for_applying_effect_expecting_u32(
        &mut context.user_applying_effect_context()?,
        fxlang::BattleEvent::ModifyDamage,
        base_damage,
    );

    let base_damage = base_damage as u16;
    let base_damage = base_damage.max(1);
    Ok(MoveOutcomeOnTarget::Damage(base_damage))
}

fn calculate_recoil_damage(context: &ActiveMoveContext) -> u64 {
    let damage_dealt = context.active_move().total_damage;
    match context.active_move().data.recoil_percent {
        Some(recoil_percent) if damage_dealt > 0 => {
            let recoil_base = if context.active_move().data.recoil_from_user_hp {
                context.as_mon_context().mon().max_hp as u64
            } else {
                damage_dealt
            };
            (recoil_percent.convert() * recoil_base).round().max(1)
        }
        _ => 0,
    }
}

mod direct_move_step {
    use std::ops::Mul;

    use crate::{
        battle::{
            core_battle_actions,
            core_battle_effects,
            core_battle_logs,
            ActiveMoveContext,
            ActiveTargetContext,
            CoreBattle,
            Mon,
            MonHandle,
            MoveOutcome,
            MoveOutcomeOnTarget,
        },
        common::{
            Error,
            Fraction,
            Id,
            WrapResultError,
        },
        effect::{
            fxlang,
            EffectHandle,
        },
        moves::{
            Accuracy,
            MoveCategory,
            MoveFlags,
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
            let is_powder = context
                .active_move()
                .data
                .flags
                .contains(&MoveFlags::Powder);

            let mut context = context.target_mon_context(target.handle)?;
            let types = Mon::types(&mut context)?;
            let immune = is_powder
                && context
                    .battle()
                    .check_multiple_type_immunity_against_effect(&types, &Id::from_known("powder"));
            // TODO: TryImmunity event.
            // TODO: Prankster immunity.

            if immune {
                core_battle_logs::immune(&mut context)?;
                target.outcome = MoveOutcome::Failed;
            }
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
            let mut immune = context.mon().level < context.target_mon().level;
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
                if context.mon().level >= context.target_mon().level {
                    let user_has_ohko_type = match ohko {
                        OhkoType::Always => true,
                        OhkoType::Type(typ) => Mon::has_type(context.as_mon_context(), typ)?,
                    };
                    if user_has_ohko_type {
                        *accuracy += context.mon().level - context.target_mon().level;
                    }
                }
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
        let hit = match accuracy {
            Accuracy::Chance(accuracy) => {
                rand_util::chance(context.battle_mut().prng.as_mut(), accuracy as u64, 100)
            }
            _ => true,
        };
        if !hit {
            core_battle_logs::miss(context)?;
        }
        Ok(hit)
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
            if targets.iter().all(|target| {
                context
                    .target_context(target.handle)
                    .map(|context| context.target_mon().hp)
                    .unwrap_or(0)
                    == 0
            }) {
                break;
            }

            // Of all the eligible targets, determine which ones we will actually hit.
            for target in targets.iter_mut().filter(|target| target.outcome.success()) {
                let mut context = context.target_context(target.handle)?;
                if context.active_move().data.multiaccuracy && hit > 0 {
                    if !accuracy_check(&mut context)? {
                        target.outcome = MoveOutcome::Failed;
                        continue;
                    }
                }
            }

            let mut hit_targets = targets
                .iter()
                .filter_map(|target| target.outcome.success().then_some(target.handle))
                .collect::<Vec<_>>();
            let hit_targets_state =
                core_battle_actions::move_hit(context, hit_targets.as_mut_slice())?;

            if hit_targets_state
                .iter()
                .all(|target| target.outcome.failed())
            {
                break;
            }

            // Record number of hits.
            context.active_move_mut().hit = hit + 1;
            context.active_move_mut().total_damage += hit_targets_state
                .iter()
                .filter_map(|target| {
                    if let MoveOutcomeOnTarget::Damage(damage) = target.outcome {
                        Some(damage as u64)
                    } else {
                        None
                    }
                })
                .sum::<u64>();

            // TODO: Update event for everything on the field, like items.
        }

        // Log OHKOs.
        for target in targets.iter() {
            let mut context = context.target_context(target.handle)?;
            if context.active_move().data.ohko_type.is_some() && context.target_mon().hp == 0 {
                core_battle_logs::ohko(&mut context)?;
            }
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
            core_battle_actions::damage(
                &mut context.as_mon_context_mut(),
                recoil_damage,
                Some(mon_handle),
                Some(&EffectHandle::Condition(Id::from_known("recoil"))),
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

        for target in targets.iter().filter(|target| target.outcome.success()) {
            core_battle_effects::run_active_move_event_expecting_void(
                context,
                fxlang::BattleEvent::AfterMoveSecondaryEffects,
            );
            core_battle_effects::run_event_for_applying_effect(
                &mut context.applying_effect_context_for_target(target.handle)?,
                fxlang::BattleEvent::AfterMoveSecondaryEffects,
                fxlang::VariableInput::default(),
            );
        }

        // TODO: Record which Mon attacked which, and how many times.

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

pub fn damage(
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
    let mut targets = [HitTargetState::new(
        target,
        MoveOutcomeOnTarget::Damage(damage),
    )];
    apply_spread_damage(&mut context, source, &mut targets)
}

fn apply_spread_damage(
    context: &mut EffectContext,
    source: Option<MonHandle>,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    for target in targets {
        let mut context = context.applying_effect_context(source, target.handle)?;
        let damage = match &mut target.outcome {
            MoveOutcomeOnTarget::Failure
            | MoveOutcomeOnTarget::Success
            | MoveOutcomeOnTarget::Damage(0) => continue,
            MoveOutcomeOnTarget::Damage(damage) => damage,
        };
        if context.target().hp == 0 {
            target.outcome = MoveOutcomeOnTarget::Damage(0);
            continue;
        }
        if !context.target().active {
            target.outcome = MoveOutcomeOnTarget::Failure;
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
                    false,
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
    log_failure: bool,
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
    if healed > 0 {
        core_battle_logs::heal(context, source, effect)?;
    } else if log_failure {
        core_battle_logs::fail_heal(context)?;
    }
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
    // We have two goals in this function:
    //  1. Apply all properties on the context's HitEffect object.
    //  2. Determine if the move did anything in this hit.

    let is_secondary = context.is_secondary();
    let is_self = context.is_self();

    let mut did_anything_to_user = MoveOutcomeOnTarget::Failure;

    for target in targets.iter_mut() {
        if target.outcome.failed() {
            continue;
        }
        let mut target_context = context.target_context(target.handle)?;
        if target_context.target_mon().fainted {
            continue;
        }

        // HitEffect is optional. If it is unspecified, we exit early.
        //
        // We clone to avoid holding onto the reference, which prevents us from creating new
        // contexts.
        let hit_effect = match target_context.hit_effect() {
            None => break,
            Some(hit_effect) => hit_effect.clone(),
        };

        let source_handle = target_context.mon_handle();
        let effect_handle = EffectHandle::ActiveMove(target_context.active_move_handle());

        let mut hit_effect_outcome: Option<MoveOutcomeOnTarget> = None;

        if let Some(boosts) = hit_effect.boosts {
            let outcome = boost(
                &mut target_context.target_mon_context()?,
                boosts,
                Some(source_handle),
                Some(&effect_handle),
                is_secondary,
                is_self,
            )?;
            hit_effect_outcome = Some(hit_effect_outcome.unwrap_or_default().combine(outcome));
        }

        if let Some(heal_percent) = hit_effect.heal_percent {
            let damage = heal_percent * target_context.mon().max_hp;
            let damage = damage.round();
            let damage = heal(
                &mut target_context.target_mon_context()?,
                damage,
                Some(source_handle),
                Some(&effect_handle),
                true,
            )?;
            let outcome = if damage == 0 {
                MoveOutcomeOnTarget::Failure
            } else {
                MoveOutcomeOnTarget::Success
            };
            hit_effect_outcome = Some(hit_effect_outcome.unwrap_or_default().combine(outcome));
        }

        if let Some(status) = hit_effect.status {
            let set_status = try_set_status(
                &mut target_context.applying_effect_context()?,
                Some(Id::from(status)),
                !is_secondary && !is_self,
            )?;
            let outcome = if set_status {
                MoveOutcomeOnTarget::Success
            } else {
                MoveOutcomeOnTarget::Failure
            };
            hit_effect_outcome = Some(hit_effect_outcome.unwrap_or_default().combine(outcome));
        }

        if let Some(volatile_status) = hit_effect.volatile_status {
            let set_status = try_add_volatile(
                &mut target_context.applying_effect_context()?,
                &Id::from(volatile_status),
                !is_secondary && !is_self,
            )?;
            let outcome = if set_status {
                MoveOutcomeOnTarget::Success
            } else {
                MoveOutcomeOnTarget::Failure
            };
            hit_effect_outcome = Some(hit_effect_outcome.unwrap_or_default().combine(outcome));
        }

        if let Some(side_condition) = hit_effect.side_condition {
            // TODO: Add side condition.
        }

        if let Some(slot_condition) = hit_effect.slot_condition {
            // TODO: Add slot condition.
        }

        if let Some(weather) = hit_effect.weather {
            // TODO: Set weather.
        }

        if let Some(terrain) = hit_effect.terrain {
            // TODO: Set terrain.
        }

        if let Some(pseudo_weather) = hit_effect.pseudo_weather {
            // TODO: Add pseudo weather.
        }

        if hit_effect.force_switch {
            let outcome = if Player::can_switch(target_context.as_player_context()) {
                MoveOutcomeOnTarget::Success
            } else {
                MoveOutcomeOnTarget::Failure
            };
            hit_effect_outcome = Some(hit_effect_outcome.unwrap_or_default().combine(outcome));
        }

        // TODO: Hit event for field, side, or target.

        // Some move effects function like HitEffect properties, but don't make much sense to be
        // generic.
        //
        // If we are checking the primary hit event on the targets, we should check these as well.
        if !is_secondary && !is_self {
            if let Some(self_destruct_type) = &context.active_move().data.self_destruct {
                did_anything_to_user = MoveOutcomeOnTarget::Success;

                // At this point, we know we have hit the target, so we self-destruct the user now.
                if self_destruct_type == &SelfDestructType::IfHit {
                    if target.outcome.hit() {
                        faint(
                            context.as_mon_context_mut(),
                            Some(source_handle),
                            Some(&effect_handle),
                        )?;
                    }
                }
            }
            if context.active_move().data.user_switch.is_some() {
                let outcome = if Player::can_switch(context.as_player_context()) {
                    MoveOutcomeOnTarget::Success
                } else {
                    MoveOutcomeOnTarget::Failure
                };
                hit_effect_outcome = Some(hit_effect_outcome.unwrap_or_default().combine(outcome));
            }
        }

        // Move did not try to do anything, so it trivially succeeds.
        let hit_effect_outcome = match hit_effect_outcome {
            Some(hit_effect_outcome) => hit_effect_outcome,
            None => MoveOutcomeOnTarget::Success,
        };

        // The target's outcome is affected by the outcome here.
        target.outcome = target.outcome.combine(hit_effect_outcome);
    }

    // Did the move do anything to its targets?
    let did_anything_to_targets = targets
        .iter()
        .map(|target| target.outcome)
        .reduce(|acc, outcome| acc.combine(outcome))
        .unwrap_or(MoveOutcomeOnTarget::Success);

    // Did the move do anything at all, to the targets or the user?
    let did_anything = did_anything_to_user.combine(did_anything_to_targets);

    if did_anything.failed() {
        // This is the primary hit of the move, and it failed to do anything, so the move failed as
        // a whole.
        if !is_self && !is_secondary {
            core_battle_logs::do_not_animate_last_move(context.as_battle_context_mut());
            core_battle_logs::fail(context.as_mon_context_mut())?;
        }
    } else if context.active_move().data.user_switch.is_some() && context.mon().hp > 0 {
        context.mon_mut().needs_switch = true;
    }

    Ok(())
}

fn boost(
    context: &mut MonContext,
    boosts: PartialBoostTable,
    source: Option<MonHandle>,
    effect: Option<&EffectHandle>,
    is_secondary: bool,
    is_self: bool,
) -> Result<MoveOutcomeOnTarget, Error> {
    if context.mon().hp == 0
        || !context.mon().active
        || Side::mons_left(context.as_side_context()) == 0
    {
        return Ok(MoveOutcomeOnTarget::Failure);
    }
    // TODO: ChangeBoost event.
    let capped_boosts = Mon::cap_boosts(context, boosts.clone());
    // TODO: TryBoost event.

    let mut success = false;
    for (boost, value) in BoostMapInOrderIterator::new(&capped_boosts) {
        let original_delta = *boosts.get(boost).unwrap_or(&0);
        let delta = Mon::boost_stat(context, *boost, *value);
        success = success && delta != 0;
        if delta != 0 || (!is_secondary && !is_self) {
            core_battle_logs::boost(context, *boost, delta, original_delta)?;
        } else if let Some(effect) = effect {
            let effect_context = context.as_battle_context_mut().effect_context(effect)?;
            let effect_type = effect_context.effect().effect_type();
            if effect_type == EffectType::Ability {
                core_battle_logs::boost(context, *boost, delta, original_delta)?;
            }
        }
    }

    // TODO: AfterBoost event.
    if success {
        if boosts.values().any(|val| val > &0) {
            context.mon_mut().stats_raised_this_turn = true;
        }
        if boosts.values().any(|val| val < &0) {
            context.mon_mut().stats_lowered_this_turn = true;
        }
        Ok(MoveOutcomeOnTarget::Success)
    } else {
        Ok(MoveOutcomeOnTarget::Failure)
    }
}

fn apply_user_effect(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    let mon_handle = context.mon_handle();
    let mut context = context.hit_self_active_move_context();
    if context.hit_effect().is_none() {
        return Ok(());
    }

    for target in targets {
        if target.outcome.failed() {
            continue;
        }

        // A move hits its targets multiple times for multihit moves. However, it is undesirable for
        // non-idempotent effects on the user (specifically stat drops) to run once for each hit.
        //
        // Thus, we keep track of whether the primary HitEffect against the user has been applied.
        // Note that this only makes an impact on multihit moves (since single hit moves
        // will trivially run through here once).
        //
        // This also only impacts the primary user effect. Secondary user effects can run multiple
        // times (since there is a little bit more control over how secondary effects run,
        // since there can be any number of them and they can be guarded behind a chance).
        if !context.is_secondary() && !context.active_move().primary_user_effect_applied {
            if context.hit_effect().wrap_error()?.boosts.is_some() {
                let chance = context
                    .active_move()
                    .data
                    .user_effect_chance
                    .unwrap_or(Fraction::from(1));
                let user_effect_roll = rand_util::chance(
                    context.battle_mut().prng.as_mut(),
                    chance.numerator() as u64,
                    chance.denominator() as u64,
                );
                if user_effect_roll {
                    move_hit(&mut context, &[mon_handle])?;
                }
                if context.active_move().data.multihit.is_some() {
                    context.active_move_mut().primary_user_effect_applied = true;
                }
            }
        } else {
            move_hit(&mut context, &[mon_handle])?;
        }
    }

    Ok(())
}

fn apply_secondary_effects(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    if context.active_move().data.secondary_effects.is_empty() {
        return Ok(());
    }

    for target in targets {
        if target.outcome.failed() {
            continue;
        }
        // TODO: ModifySecondaries event.
        for i in 0..context.active_move().data.secondary_effects.len() {
            let secondary_effect = match context.active_move().data.secondary_effects.get(i) {
                None => break,
                Some(secondary_effect) => secondary_effect,
            };
            let chance = secondary_effect.chance.unwrap_or(Fraction::from(1));
            let secondary_roll = rand_util::chance(
                context.battle_mut().prng.as_mut(),
                chance.numerator() as u64,
                chance.denominator() as u64,
            );
            if secondary_roll {
                let mut context = context.secondary_active_move_context(i);
                move_hit(&mut context, &[target.handle])?;
            }
        }
    }

    Ok(())
}

fn force_switch(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    if !context
        .hit_effect()
        .is_some_and(|hit_effect| hit_effect.force_switch)
    {
        return Ok(());
    }

    for target in targets {
        let mut context = context.target_context(target.handle)?;
        if target.outcome.failed()
            || context.target_mon().hp == 0
            || context.mon().hp == 0
            || !Player::can_switch(context.target_mon_context()?.as_player_context())
        {
            continue;
        }
        // TODO: DragOut event.
        context.target_mon_mut().force_switch = true;
    }

    Ok(())
}

pub fn try_set_status(
    context: &mut ApplyingEffectContext,
    status: Option<Id>,
    is_primary_move_effect: bool,
) -> Result<bool, Error> {
    if context.target().hp == 0 {
        return Ok(false);
    }

    // A Mon may only have one status set at a time.
    match (&status, &context.target().status) {
        (Some(_), Some(_)) => {
            if is_primary_move_effect {
                if let Some(mut source_context) = context.source_context()? {
                    core_battle_logs::fail(&mut source_context)?;
                }
            }
            return Ok(false);
        }
        _ => (),
    }

    // Cure the current status and return early.
    let status = match status {
        Some(status) => status,
        None => {
            context.target_mut().status = status;
            context.target_mut().status_state = fxlang::EffectState::new();
            return Ok(true);
        }
    };

    let status_effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(&status)?
        .clone();
    let status = status_effect_handle
        .try_id()
        .wrap_error_with_message("status must have an id")?
        .clone();

    if check_status_immunity(&mut context.target_context()?, &status)? {
        if is_primary_move_effect {
            core_battle_logs::immune(&mut context.target_context()?)?;
        }
        return Ok(false);
    }

    // Save the previous status in case an effect callback cancels the status.
    let previous_status = context.target().status.clone();
    let previous_status_state = context.target().status_state.clone();

    if !core_battle_effects::run_event_for_applying_effect(
        context,
        fxlang::BattleEvent::SetStatus,
        fxlang::VariableInput::from_iter([fxlang::Value::Effect(status_effect_handle.clone())]),
    ) {
        return Ok(false);
    }

    // Set the status so that the following effects can use it.
    context.target_mut().status = Some(status);
    context.target_mut().status_state = fxlang::EffectState::new();

    if let Some(condition) =
        CoreBattle::get_effect_by_handle(context.as_battle_context_mut(), &status_effect_handle)?
            .fxlang_condition()
    {
        if let Some(duration) = condition.duration {
            context.target_mut().status_state.set_duration(duration);
        }

        if let Some(duration) = core_battle_effects::run_mon_status_event_expecting_u8(
            context,
            fxlang::BattleEvent::Duration,
        ) {
            context.target_mut().status_state.set_duration(duration);
        }
    }

    if core_battle_effects::run_mon_status_event_expecting_bool(context, fxlang::BattleEvent::Start)
        .is_some_and(|result| !result)
    {
        context.target_mut().status = previous_status;
        context.target_mut().status_state = previous_status_state;
    }

    if !core_battle_effects::run_event_for_applying_effect(
        context,
        fxlang::BattleEvent::AfterSetStatus,
        fxlang::VariableInput::from_iter([fxlang::Value::Effect(status_effect_handle)]),
    ) {
        return Ok(false);
    }

    Ok(true)
}

fn check_status_immunity(context: &mut MonContext, status: &Id) -> Result<bool, Error> {
    if context.mon().hp == 0 {
        return Ok(true);
    }

    let types = Mon::types(context)?;
    if context
        .battle_mut()
        .check_multiple_type_immunity_against_effect(&types, status)
    {
        return Ok(true);
    }

    // TODO: Immunity event.

    Ok(false)
}

pub fn clear_status(
    context: &mut ApplyingEffectContext,
    is_primary_move_effect: bool,
) -> Result<bool, Error> {
    if context.target().hp == 0 || context.target().status.is_none() {
        return Ok(false);
    }
    try_set_status(context, None, is_primary_move_effect)
}

pub fn cure_status(context: &mut ApplyingEffectContext, log_effect: bool) -> Result<bool, Error> {
    if context.target().hp == 0 {
        return Ok(false);
    }
    match context.target().status.clone() {
        None => return Ok(false),
        Some(status) => {
            let status_name =
                CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &status)?
                    .name()
                    .to_owned();
            core_battle_logs::cure_status(context, &status_name, log_effect)?;
        }
    }
    try_set_status(context, None, false)
}

pub fn try_add_volatile(
    context: &mut ApplyingEffectContext,
    status: &Id,
    is_primary_move_effect: bool,
) -> Result<bool, Error> {
    if context.target().hp == 0 {
        return Ok(false);
    }

    let volatile_effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(status)?
        .clone();
    let status = volatile_effect_handle
        .try_id()
        .wrap_error_with_message("volatile must have an id")?
        .clone();

    if context.target().volatiles.contains_key(&status) {
        return Ok(core_battle_effects::run_mon_volatile_event_expecting_bool(
            context,
            fxlang::BattleEvent::Restart,
            &status,
        )
        .unwrap_or(true));
    }

    if check_status_immunity(&mut context.target_context()?, &status)? {
        if is_primary_move_effect {
            core_battle_logs::immune(&mut context.target_context()?)?;
        }
        return Ok(false);
    }
    if !core_battle_effects::run_event_for_applying_effect(
        context,
        fxlang::BattleEvent::AddVolatile,
        fxlang::VariableInput::from_iter([fxlang::Value::Effect(volatile_effect_handle.clone())]),
    ) {
        return Ok(false);
    }

    context
        .target_mut()
        .volatiles
        .insert(status.clone(), fxlang::EffectState::new());

    if let Some(condition) =
        CoreBattle::get_effect_by_handle(context.as_battle_context_mut(), &volatile_effect_handle)?
            .fxlang_condition()
    {
        if let Some(duration) = condition.duration {
            context
                .target_mut()
                .volatiles
                .get_mut(&status)
                .wrap_error_with_message("expected volatile state to exist")?
                .set_duration(duration);
        }

        if let Some(duration) = core_battle_effects::run_mon_volatile_event_expecting_u8(
            context,
            fxlang::BattleEvent::Duration,
            &status,
        ) {
            context
                .target_mut()
                .volatiles
                .get_mut(&status)
                .wrap_error_with_message("expected volatile state to exist")?
                .set_duration(duration);
        }
    }

    if core_battle_effects::run_mon_volatile_event_expecting_bool(
        context,
        fxlang::BattleEvent::Start,
        &status,
    )
    .is_some_and(|result| !result)
    {
        context.target_mut().volatiles.remove(&status);
    }

    let volatile_name = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &status)?
        .name()
        .to_owned();
    core_battle_logs::add_volatile(context, &volatile_name)?;

    Ok(true)
}

pub fn remove_volatile(context: &mut ApplyingEffectContext, status: &Id) -> Result<bool, Error> {
    if context.target().hp == 0 {
        return Ok(false);
    }

    let volatile_effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(&status)?
        .clone();
    let status = volatile_effect_handle
        .try_id()
        .wrap_error_with_message("volatile must have an id")?
        .clone();

    if !context.target().volatiles.contains_key(&status) {
        return Ok(false);
    }

    core_battle_effects::run_mon_volatile_event(context, fxlang::BattleEvent::End, &status);
    context.target_mut().volatiles.remove(&status);

    let volatile_name = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &status)?
        .name()
        .to_owned();
    core_battle_logs::remove_volatile(context, &volatile_name)?;

    Ok(true)
}
