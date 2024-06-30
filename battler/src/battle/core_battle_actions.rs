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
        BoostTable,
        Context,
        CoreBattle,
        EffectContext,
        Mon,
        MonContext,
        MonHandle,
        MoveEventResult,
        MoveHandle,
        MoveOutcome,
        MoveOutcomeOnTarget,
        Player,
        PlayerContext,
        Side,
        SideEffectContext,
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
        Move,
        MoveCategory,
        MoveFlags,
        MoveTarget,
        SelfDestructType,
        SwitchType,
    },
    rng::rand_util,
};

#[derive(Clone)]
struct HitTargetState {
    handle: MonHandle,
    outcome: MoveOutcomeOnTarget,
}

impl HitTargetState {
    pub fn new(handle: MonHandle, outcome: MoveOutcomeOnTarget) -> Self {
        Self { handle, outcome }
    }
}

fn hit_targets_state_from_targets<I>(targets: I) -> Vec<HitTargetState>
where
    I: IntoIterator<Item = MonHandle>,
{
    targets
        .into_iter()
        .map(|handle| HitTargetState::new(handle, MoveOutcomeOnTarget::Success))
        .collect()
}

/// Switches a Mon into the given position.
pub fn switch_in(
    context: &mut MonContext,
    position: usize,
    mut switch_type: Option<SwitchType>,
) -> Result<bool, Error> {
    if context.mon_mut().active {
        core_battle_logs::hint(
            context.as_battle_context_mut(),
            "A switch failed because the Mon trying to switch in is already in.",
        )?;
        return Ok(false);
    }

    let active_len = context.player().active.len();
    if position >= active_len {
        return Err(battler_error!(
            "invalid switch position {position} / {active_len}"
        ));
    }

    let previous_mon = context
        .player()
        .active
        .get(position)
        .cloned()
        .wrap_error_with_format(format_args!(
            "expected {position} to be a valid index to active Mons"
        ))?;
    if let Some(previous_mon) = previous_mon {
        let mut context = context.as_battle_context_mut().mon_context(previous_mon)?;
        if context.mon().hp > 0 {
            if let Some(previous_mon_switch_type) = context.mon().needs_switch {
                switch_type = Some(previous_mon_switch_type);
            }

            context.mon_mut().being_called_back = true;

            if !context.mon().skip_before_switch_out {
                // TODO: BeforeSwitchOut event.
                // TODO: Update event.
            }
            context.mon_mut().skip_before_switch_out = false;

            // TODO: SwitchOut event.

            // The Mon could faint here, which cancels the switch (Pursuit).
            if context.mon().hp == 0 {
                return Ok(false);
            }

            // TODO: Ability End event.

            if let Some(SwitchType::CopyVolatile) = switch_type {
                // TODO: Copy volatiles to the new Mon.
            }

            Mon::clear_volatile(&mut context, true)?;
        }

        context.mon_mut().active = false;
        context.mon_mut().needs_switch = None;
        context.mon_mut().stats_raised_this_turn = false;
        context.mon_mut().stats_lowered_this_turn = false;
    }

    Mon::switch_in(context, position);
    context.player_mut().active[position] = Some(context.mon_handle());

    core_battle_logs::switch(context)?;

    run_switch_in_events(context)
}

pub fn run_switch_in_events(context: &mut MonContext) -> Result<bool, Error> {
    core_battle_effects::run_event_for_mon(context, fxlang::BattleEvent::SwitchIn);

    // TODO: EntryHazard event.

    if context.mon().hp == 0 {
        return Ok(false);
    }
    if !context.mon().fainted {
        // TODO: Ability Start event.
        // TODO: Item Start event.
    }

    Ok(true)
}

fn register_active_move_by_id(context: &mut Context, move_id: &Id) -> Result<MoveHandle, Error> {
    let mut active_move = context
        .battle_mut()
        .dex
        .moves
        .get_by_id(move_id)?
        .deref()
        .clone();
    register_active_move(context, active_move)
}

pub fn register_active_move(
    context: &mut Context,
    mut active_move: Move,
) -> Result<MoveHandle, Error> {
    let active_move_handle = context.battle_mut().register_move(active_move);
    Ok(active_move_handle)
}

fn mon_is_charging(context: &mut ActiveMoveContext) -> Result<bool, Error> {
    Ok(context
        .active_move()
        .data
        .flags
        .contains(&MoveFlags::Charge)
        && Mon::has_volatile(context.as_mon_context_mut(), &Id::from_known("twoturnmove"))?)
}

/// Executes the given move by a Mon.
pub fn do_move(
    context: &mut MonContext,
    move_id: &Id,
    target_location: Option<isize>,
    original_target: Option<MonHandle>,
    source_effect: Option<&EffectHandle>,
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
    let active_move_handle = register_active_move_by_id(context.as_battle_context_mut(), move_id)?;

    context.mon_mut().set_active_move(active_move_handle);

    let mut context = context.active_move_context()?;

    // BeforeMove event handlers can prevent the move from being used.
    if !core_battle_effects::run_event_for_applying_effect(
        &mut context.user_applying_effect_context(None)?,
        fxlang::BattleEvent::BeforeMove,
        fxlang::VariableInput::default(),
    ) {
        core_battle_effects::run_event_for_applying_effect(
            &mut context.user_applying_effect_context(None)?,
            fxlang::BattleEvent::MoveAborted,
            fxlang::VariableInput::default(),
        );
        context.mon_mut().move_this_turn_outcome = Some(MoveOutcome::Failed);
        return Ok(());
    }

    let locked_move_before = Mon::locked_move(context.as_mon_context_mut())?;

    // Check that move has enough PP to be used.
    if !external {
        let move_id = context.active_move().id();
        // SAFETY: move_id is only used for lookup.
        let move_id = unsafe { move_id.unsafely_detach_borrow() };
        if locked_move_before.is_none()
            && !context.mon_mut().check_pp(move_id, 1)
            && !move_id.eq("struggle")
        {
            // No PP, so this move action cannot be carried through.
            let move_name = &context.active_move().data.name;
            // SAFETY: Logging does not change the active move.
            let move_name = unsafe { move_name.unsafely_detach_borrow() };
            core_battle_logs::cant(context.as_mon_context_mut(), "nopp", Some(move_name))?;
            return Ok(());
        }

        context.mon_mut().last_move_target = target_location;
    }

    // Use the move.
    use_active_move(
        context.as_mon_context_mut(),
        active_move_handle,
        target,
        source_effect,
        external,
    )?;

    // Deduct PP if the move was successful.
    //
    // Charging moves do not have their PP deducted.
    if !external && !mon_is_charging(&mut context)? {
        let move_id = context.active_move().id();
        // SAFETY: move_id is only used for lookup.
        let move_id = unsafe { move_id.unsafely_detach_borrow() };
        context.mon_mut().deduct_pp(move_id, 1);

        // At this point, the move was used, so we should remember it.
        context.mon_mut().last_move = Some(active_move_handle);
    }

    core_battle_effects::run_active_move_event_expecting_void(
        &mut context,
        fxlang::BattleEvent::AfterMove,
        core_battle_effects::MoveTargetForEvent::User,
    );
    core_battle_effects::run_event_for_applying_effect(
        &mut context.user_applying_effect_context(None)?,
        fxlang::BattleEvent::AfterMove,
        fxlang::VariableInput::default(),
    );

    context.mon_mut().clear_active_move();

    CoreBattle::faint_messages(context.as_battle_context_mut())?;
    CoreBattle::check_win(context.as_battle_context_mut())?;

    Ok(())
}

pub fn use_move(
    context: &mut MonContext,
    move_id: &Id,
    target: Option<MonHandle>,
    source_effect: Option<&EffectHandle>,
    external: bool,
) -> Result<bool, Error> {
    let active_move_handle = register_active_move_by_id(context.as_battle_context_mut(), move_id)?;
    use_active_move(context, active_move_handle, target, source_effect, external)
}

pub fn use_active_move(
    context: &mut MonContext,
    active_move_handle: MoveHandle,
    target: Option<MonHandle>,
    source_effect: Option<&EffectHandle>,
    external: bool,
) -> Result<bool, Error> {
    context.mon_mut().move_this_turn_outcome = None;

    context.mon_mut().set_active_move(active_move_handle);

    let mut context = context.active_move_context()?;
    context.active_move_mut().source_effect = source_effect.cloned();
    context.active_move_mut().used_by = Some(context.mon_handle());
    context.active_move_mut().external = external;

    let outcome = use_active_move_internal(&mut context, target)?;

    context.mon_mut().move_this_turn_outcome = Some(outcome);

    Ok(outcome.success())
}

fn use_active_move_internal(
    context: &mut ActiveMoveContext,
    mut target: Option<MonHandle>,
) -> Result<MoveOutcome, Error> {
    context.mon_mut().last_move_used = Some(context.active_move_handle());

    // TODO: ModifyType on the move.
    core_battle_effects::run_active_move_event_expecting_void(
        context,
        fxlang::BattleEvent::UseMove,
        core_battle_effects::MoveTargetForEvent::User,
    );

    // TODO: ModifyType events on the Mon.
    core_battle_effects::run_event_for_applying_effect(
        &mut context.user_applying_effect_context(None)?,
        fxlang::BattleEvent::UseMove,
        fxlang::VariableInput::default(),
    );

    // Mon fainted before this move could be made.
    if context.mon().fainted {
        return Ok(MoveOutcome::Failed);
    }

    let targets = get_move_targets(context, target)?;
    if context.active_move().data.target.has_single_target() {
        target = targets.first().cloned();
    }

    // Log that the move is being used.
    let move_name = context.active_move().data.name.clone();
    core_battle_logs::use_move(context.as_mon_context_mut(), &move_name, target)?;

    if context.active_move().data.target.requires_target() && target.is_none() {
        core_battle_logs::last_move_had_no_target(context.as_battle_context_mut());
        core_battle_logs::fail(context.as_mon_context_mut())?;
        return Ok(MoveOutcome::Failed);
    }

    // TODO: DeductPP event (for Pressure).
    // TODO: Targeted event.
    // TODO: TryMove event.
    core_battle_effects::run_active_move_event_expecting_void(
        context,
        fxlang::BattleEvent::UseMoveMessage,
        core_battle_effects::MoveTargetForEvent::User,
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
        try_indirect_move(context, &targets)?
    } else {
        if targets.is_empty() {
            core_battle_logs::last_move_had_no_target(context.as_battle_context_mut());
            core_battle_logs::fail(context.as_mon_context_mut())?;
            return Ok(MoveOutcome::Failed);
        }
        try_direct_move(context, &targets)?
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

    if outcome == MoveOutcome::Failed {
        core_battle_effects::run_active_move_event_expecting_void(
            context,
            fxlang::BattleEvent::MoveFailed,
            core_battle_effects::MoveTargetForEvent::User,
        );
    }

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
    selected_target: Option<MonHandle>,
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
            let mut target = match selected_target {
                Some(target) => {
                    let mon = context.mon_handle();
                    let target_context = context.target_mon_context(target)?;
                    if target_context.mon().fainted
                        && !target_context
                            .mon()
                            .is_ally(target_context.as_battle_context().mon(mon)?)
                    {
                        // The targeted Mon has fainted, so the move should retarget.
                        None
                    } else {
                        Some(target)
                    }
                }
                None => None,
            };

            if target.is_none() && !context.active_move().data.no_random_target {
                let mon = context.mon_handle();
                let move_target = context.active_move().data.target;
                target =
                    CoreBattle::random_target(context.as_battle_context_mut(), mon, move_target)?;
            }

            let mut target = match target {
                Some(target) => target,
                None => return Ok(Vec::new()),
            };

            if context.battle().max_side_length() > 1 && !context.active_move().data.tracks_target {
                if !mon_is_charging(context)? {
                    target = core_battle_effects::run_event_for_applying_effect_expecting_mon_quick_return(
                        &mut context.user_applying_effect_context(Some(target))?,
                        fxlang::BattleEvent::RedirectTarget,
                        fxlang::VariableInput::from_iter([fxlang::Value::Mon(target)]),
                    ).unwrap_or(target);
                }
            }

            targets.push(target);
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

fn run_try_use_move_events(context: &mut ActiveMoveContext) -> Result<Option<MoveOutcome>, Error> {
    let move_event_result = core_battle_effects::run_active_move_event_expecting_move_event_result(
        context,
        fxlang::BattleEvent::TryUseMove,
        core_battle_effects::MoveTargetForEvent::User,
    );

    let move_prepare_hit_result = core_battle_effects::run_active_move_event_expecting_bool(
        context,
        fxlang::BattleEvent::PrepareHit,
        core_battle_effects::MoveTargetForEvent::User,
    )
    .map(|value| MoveEventResult::from(value))
    .unwrap_or(MoveEventResult::Advance);

    let event_prepare_hit_result =
        MoveEventResult::from(core_battle_effects::run_event_for_applying_effect(
            &mut context.user_applying_effect_context(None)?,
            fxlang::BattleEvent::PrepareHit,
            fxlang::VariableInput::default(),
        ));

    let move_event_result = move_event_result
        .combine(move_prepare_hit_result)
        .combine(event_prepare_hit_result);

    if !move_event_result.advance() {
        if move_event_result.failed() {
            core_battle_logs::fail(context.as_mon_context_mut())?;
            core_battle_logs::do_not_animate_last_move(context.as_battle_context_mut());
            return Ok(Some(MoveOutcome::Failed));
        }
        return Ok(Some(MoveOutcome::Skipped));
    }
    return Ok(None);
}

fn try_indirect_move(
    context: &mut ActiveMoveContext,
    targets: &[MonHandle],
) -> Result<MoveOutcome, Error> {
    if let Some(try_use_move_outcome) = run_try_use_move_events(context)? {
        return Ok(try_use_move_outcome);
    }

    let move_target = context.active_move().data.target;
    let try_move_result = match move_target {
        MoveTarget::All => core_battle_effects::run_event_for_field_effect_expecting_move_event_result(
            &mut context.field_effect_context()?,
            fxlang::BattleEvent::TryHitField,
            fxlang::VariableInput::default(),
        ),
        MoveTarget::AllySide | MoveTarget::AllyTeam => {
            core_battle_effects::run_event_for_side_effect_expecting_move_event_result(
                &mut context.side_effect_context(context.side().index)?,
                fxlang::BattleEvent::TryHitSide,
                fxlang::VariableInput::default(),
            )
        }
        MoveTarget::FoeSide => {
            core_battle_effects::run_event_for_side_effect_expecting_move_event_result(
                &mut context.side_effect_context(context.side().index)?,
                fxlang::BattleEvent::TryHitSide,
                fxlang::VariableInput::default(),
            )
        }
        _ => return Err(battler_error!("move against target {move_target} applied indirectly, but it should directly hit target mons"))
    };

    if !try_move_result.advance() {
        if try_move_result.failed() {
            core_battle_logs::fail(context.as_mon_context_mut())?;
            core_battle_logs::do_not_animate_last_move(context.as_battle_context_mut());
        }
        return Ok(MoveOutcome::Failed);
    }

    // Hit the first target, as a representative of the side.
    //
    // This lets us share the core move hit code between all types of moves.
    //
    // Note that if a move breaks its promise and actually applies a Mon effect, it will get applied
    // to this Mon. This should be viewed as undefined behavior.
    move_hit_determine_success(context, &targets[0..1])
}

fn try_direct_move(
    context: &mut ActiveMoveContext,
    targets: &[MonHandle],
) -> Result<MoveOutcome, Error> {
    if targets.len() > 1 && !context.active_move().data.smart_target {
        context.active_move_mut().spread_hit = true;
    }

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

    if let Some(try_use_move_outcome) = run_try_use_move_events(context)? {
        return Ok(try_use_move_outcome);
    }

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
        at_least_one_failure =
            at_least_one_failure || targets.iter().any(|target| target.outcome.failed());
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

    if context.active_move().spread_hit && !outcome.failed() {
        core_battle_logs::last_move_spread_targets(
            context.as_battle_context_mut(),
            targets.into_iter().map(|target| target.handle),
        )?;
    }

    Ok(outcome)
}

fn move_hit_determine_success(
    context: &mut ActiveMoveContext,
    targets: &[MonHandle],
) -> Result<MoveOutcome, Error> {
    let targets = move_hit(
        context,
        hit_targets_state_from_targets(targets.iter().cloned()),
    )?;
    if targets.into_iter().all(|target| target.outcome.failed()) {
        Ok(MoveOutcome::Failed)
    } else {
        Ok(MoveOutcome::Success)
    }
}

fn move_hit(
    context: &mut ActiveMoveContext,
    mut hit_targets_state: Vec<HitTargetState>,
) -> Result<Vec<HitTargetState>, Error> {
    hit_targets(context, hit_targets_state.as_mut_slice())?;
    Ok(hit_targets_state)
}

fn hit_targets(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    let move_target = context.active_move().data.target.clone();
    let try_move_result = match move_target {
        MoveTarget::All => core_battle_effects::run_active_move_event_expecting_move_event_result(
            context,
            fxlang::BattleEvent::TryHitField,
            core_battle_effects::MoveTargetForEvent::None,
        ),
        MoveTarget::AllySide | MoveTarget::AllyTeam => {
            core_battle_effects::run_active_move_event_expecting_move_event_result(
                context,
                fxlang::BattleEvent::TryHitField,
                core_battle_effects::MoveTargetForEvent::Side(context.side().index),
            )
        }
        MoveTarget::FoeSide => {
            core_battle_effects::run_active_move_event_expecting_move_event_result(
                context,
                fxlang::BattleEvent::TryHitField,
                core_battle_effects::MoveTargetForEvent::Side(context.foe_side().index),
            )
        }
        _ => {
            let mut result = MoveEventResult::Advance;
            for target in targets.iter() {
                let next_result =
                    core_battle_effects::run_active_move_event_expecting_move_event_result(
                        context,
                        fxlang::BattleEvent::TryHit,
                        core_battle_effects::MoveTargetForEvent::Mon(target.handle),
                    );
                result = result.combine(next_result);
                if !result.advance() {
                    break;
                }
            }
            result
        }
    };

    if !try_move_result.advance() {
        if try_move_result.failed() {
            core_battle_logs::fail(context.as_mon_context_mut())?;
            core_battle_logs::do_not_animate_last_move(context.as_battle_context_mut());
        }
        for target in targets {
            target.outcome = MoveOutcomeOnTarget::Failure;
        }
        return Ok(());
    }

    // First, check for substitute.
    if !context.is_secondary() && !context.is_self() && move_target.affects_mons_directly() {
        try_primary_hit(context, targets)?;
    }

    // Hitting a substitute produces 0 damage, mark those.
    for target in targets.as_mut() {
        if target.outcome == MoveOutcomeOnTarget::Damage(0) {
            target.outcome = MoveOutcomeOnTarget::HitSubstitute;
        }
    }

    // Calculate damage for each target.
    calculate_spread_damage(context, targets)?;
    for target in targets.iter_mut() {
        if target.outcome.failed() {
            if !context.is_secondary() && !context.is_self() {
                core_battle_logs::fail_target(&mut context.target_mon_context(target.handle)?)?;
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
            fxlang::VariableInput::from_iter([fxlang::Value::U64(target.outcome.damage() as u64)]),
        );
    }

    Ok(())
}

fn try_primary_hit(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    for target in targets {
        if target.outcome.failed() {
            continue;
        }
        target.outcome =
            core_battle_effects::run_event_for_applying_effect_expecting_move_outcome_on_target(
                &mut context.applying_effect_context_for_target(target.handle)?,
                fxlang::BattleEvent::TryPrimaryHit,
            )
            .unwrap_or(MoveOutcomeOnTarget::Success);
    }
    Ok(())
}

fn calculate_spread_damage(
    context: &mut ActiveMoveContext,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    for target in targets {
        if !target.outcome.hit_target() {
            continue;
        }
        target.outcome = MoveOutcomeOnTarget::Unknown;
        // Secondary or effects on the user cannot deal damage.
        //
        // Note that this is different from moves that target the user.
        if context.is_secondary() || context.is_self() {
            continue;
        }
        let mut context = context.target_context(target.handle)?;
        target.outcome = calculate_damage(&mut context)?;
    }
    Ok(())
}

pub fn calculate_damage(context: &mut ActiveTargetContext) -> Result<MoveOutcomeOnTarget, Error> {
    let target_mon_handle = context.target_mon_handle();
    // Type immunity.
    let move_type = context.active_move().data.primary_type;
    let ignore_immunity =
        context.active_move().data.ignore_immunity() || context.active_move().data.typeless;
    if !ignore_immunity && Mon::is_immune(&mut context.target_mon_context()?, move_type)? {
        return Ok(MoveOutcomeOnTarget::Failure);
    }

    // OHKO.
    if context.active_move().data.ohko_type.is_some() {
        return Ok(MoveOutcomeOnTarget::Damage(context.target_mon().max_hp));
    }

    let target_handle = context.target_mon_handle();
    if let Some(damage) = core_battle_effects::run_active_move_event_expecting_u16(
        context.as_active_move_context_mut(),
        fxlang::BattleEvent::Damage,
        core_battle_effects::MoveTargetForEvent::Mon(target_handle),
    ) {
        return Ok(MoveOutcomeOnTarget::Damage(damage));
    }

    // Static damage.
    if let Some(damage) = context.active_move().data.damage {
        return Ok(MoveOutcomeOnTarget::Damage(damage));
    }

    let mut base_power = context.active_move().data.base_power;
    if let Some(dynamic_base_power) = core_battle_effects::run_active_move_event_expecting_u32(
        context.as_active_move_context_mut(),
        fxlang::BattleEvent::BasePower,
        core_battle_effects::MoveTargetForEvent::Mon(target_handle),
    ) {
        base_power = dynamic_base_power;
    }

    // If base power is explicitly 0, no damage should be dealt.
    //
    // Status moves stop here.
    if base_power == 0 {
        return Ok(MoveOutcomeOnTarget::Unknown);
    }

    // Critical hit.
    let crit_ratio = context.active_move().data.crit_ratio.unwrap_or(0);
    let crit_ratio = core_battle_effects::run_event_for_applying_effect_expecting_u8(
        &mut context.user_applying_effect_context()?,
        fxlang::BattleEvent::ModifyCritRatio,
        crit_ratio,
    );
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
    let stab = !context.active_move().data.typeless
        && Mon::has_type(context.as_mon_context_mut(), move_type)?;
    if stab {
        let stab_modifier = context
            .active_move()
            .clone()
            .stab_modifier
            .unwrap_or(Fraction::new(3, 2));
        base_damage = modify_32(base_damage, stab_modifier);
    }

    // Type effectiveness.
    let type_modifier = if context.active_move().data.typeless {
        0
    } else {
        Mon::type_effectiveness(&mut context.target_mon_context()?, move_type)?
    };
    let type_modifier = type_modifier.max(-6).min(6);
    context
        .active_move_mut()
        .hit_data(target_mon_handle)
        .type_modifier = type_modifier;
    if type_modifier > 0 {
        core_battle_logs::super_effective(&mut context.target_mon_context()?)?;
        for _ in 0..type_modifier {
            base_damage *= 2;
        }
    } else if type_modifier < 0 {
        core_battle_logs::resisted(&mut context.target_mon_context()?)?;
        for _ in 0..-type_modifier {
            base_damage /= 2;
        }
    }

    if crit {
        core_battle_logs::critical_hit(&mut context.target_mon_context()?)?;
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

fn calculate_recoil_damage(context: &ActiveMoveContext, damage_dealt: u64) -> u64 {
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

pub fn apply_recoil_damage(
    context: &mut ActiveMoveContext,
    damage_dealt: u64,
) -> Result<(), Error> {
    let recoil_damage = calculate_recoil_damage(context, damage_dealt);
    if recoil_damage > 0 {
        let recoil_damage = recoil_damage.min(u16::MAX as u64) as u16;
        let mon_handle = context.mon_handle();
        damage(
            &mut context
                .user_applying_effect_context(None)?
                .forward_applying_effect_context(EffectHandle::Condition(Id::from_known(
                    "recoil",
                )))?,
            recoil_damage,
        )?;
    }

    if context.active_move().data.struggle_recoil {
        let recoil_damage = Fraction::new(context.mon().max_hp, 4).round();
        let mon_handle = context.mon_handle();
        direct_damage(
            &mut context.as_mon_context_mut(),
            recoil_damage,
            Some(mon_handle),
            Some(&EffectHandle::Condition(Id::from_known("strugglerecoil"))),
        )?;
    }

    Ok(())
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
        effect::fxlang,
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
            if !core_battle_effects::run_event_for_applying_effect(
                &mut context.applying_effect_context_for_target(target.handle)?,
                fxlang::BattleEvent::Invulnerability,
                fxlang::VariableInput::default(),
            ) {
                target.outcome = MoveOutcome::Failed;
                core_battle_logs::miss(&mut context.target_mon_context(target.handle)?)?;
            }
        }
        Ok(())
    }

    pub fn check_try_hit_event(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        for target in targets.iter_mut() {
            if !core_battle_effects::run_event_for_applying_effect(
                &mut context.applying_effect_context_for_target(target.handle)?,
                fxlang::BattleEvent::TryHit,
                fxlang::VariableInput::default(),
            ) {
                target.outcome = MoveOutcome::Failed;
            }
        }
        if targets.iter().all(|target| target.outcome.failed()) {
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

            let types = Mon::types(&mut context.target_mon_context(target.handle)?)?;
            let immune = (is_powder
                && context
                    .battle()
                    .check_multiple_type_immunity_against_effect(
                        &types,
                        &Id::from_known("powder"),
                    ))
                || !core_battle_effects::run_active_move_event_expecting_bool(
                    context,
                    fxlang::BattleEvent::TryImmunity,
                    core_battle_effects::MoveTargetForEvent::Mon(target.handle),
                )
                .unwrap_or(true);

            // TODO: Prankster immunity.

            if immune {
                core_battle_logs::immune(&mut context.target_mon_context(target.handle)?)?;
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
                if Mon::has_type(&mut context.target_mon_context()?, typ)? {
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
                        OhkoType::Type(typ) => Mon::has_type(context.as_mon_context_mut(), typ)?,
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
            // TODO: If also not semi-invulnerable, accuracy is exempt.
        } else {
            if core_battle_effects::run_event_for_applying_effect_expecting_bool(
                &mut context.applying_effect_context()?,
                fxlang::BattleEvent::AccuracyExempt,
            )
            .is_some()
            {
                accuracy = Accuracy::Exempt;
            }
        }

        let hit = match accuracy {
            Accuracy::Chance(accuracy) => {
                rand_util::chance(context.battle_mut().prng.as_mut(), accuracy as u64, 100)
            }
            _ => true,
        };
        if !hit {
            core_battle_logs::miss(&mut context.target_mon_context()?)?;
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

            // Record number of hits.
            //
            // We do this now so that damage and base power callbacks can use this value.
            context.active_move_mut().hit = hit + 1;

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

            let hit_targets = targets
                .iter()
                .enumerate()
                .filter_map(|(i, target)| target.outcome.success().then_some((i, target.handle)))
                .collect::<Vec<_>>();
            let hit_targets_state = core_battle_actions::move_hit(
                context,
                core_battle_actions::hit_targets_state_from_targets(
                    hit_targets
                        .iter()
                        .map(|(_, target_handle)| target_handle)
                        .cloned(),
                ),
            )?;

            // Update the outcome for the target as soon as possible.
            for (i, _) in hit_targets {
                let new_outcome = MoveOutcome::from(
                    !hit_targets_state
                        .get(i)
                        .wrap_error_with_format(format_args!(
                            "expected target hit state at index {i}"
                        ))?
                        .outcome
                        .failed(),
                );
                targets
                    .get_mut(i)
                    .wrap_error_with_format(format_args!("expected target at index {i}"))?
                    .outcome = new_outcome;
            }

            if hit_targets_state
                .iter()
                .all(|target| target.outcome.failed())
            {
                // This hit failed.
                context.active_move_mut().hit -= 1;
                break;
            }

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
                core_battle_logs::ohko(&mut context.target_mon_context()?)?;
            }
        }

        CoreBattle::faint_messages(context.as_battle_context_mut())?;

        let hits = context.active_move().hit;
        if context.active_move().data.multihit.is_some() {
            core_battle_logs::hit_count(context, hits)?;
        }

        core_battle_actions::apply_recoil_damage(context, context.active_move().total_damage)?;

        for target in targets.iter().filter(|target| target.outcome.success()) {
            core_battle_effects::run_active_move_event_expecting_void(
                context,
                fxlang::BattleEvent::AfterMoveSecondaryEffects,
                core_battle_effects::MoveTargetForEvent::Mon(target.handle),
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

pub fn direct_damage(
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

pub fn damage(context: &mut ApplyingEffectContext, damage: u16) -> Result<u16, Error> {
    let target = context.target_handle();
    let source = context.source_handle();
    let mut targets = [HitTargetState::new(
        target,
        MoveOutcomeOnTarget::Damage(damage),
    )];
    apply_spread_damage(context.as_effect_context_mut(), source, &mut targets)?;
    Ok(targets
        .get(0)
        .wrap_error_with_message("expected target result to exist after applying spread damage")?
        .outcome
        .damage())
}

fn apply_spread_damage(
    context: &mut EffectContext,
    source: Option<MonHandle>,
    targets: &mut [HitTargetState],
) -> Result<(), Error> {
    for target in targets {
        let mut context = context.applying_effect_context(source, target.handle)?;
        let damage = match &mut target.outcome {
            MoveOutcomeOnTarget::Damage(damage) => damage,
            _ => continue,
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
        let effect_handle = context.effect_handle().clone();
        *damage = Mon::damage(
            &mut context.target_context()?,
            *damage,
            source_handle,
            Some(&effect_handle),
        )?;
        context.target_mut().hurt_this_turn = context.target().hp;

        core_battle_effects::run_event_for_applying_effect(
            &mut context,
            fxlang::BattleEvent::DamageReceived,
            fxlang::VariableInput::from_iter([fxlang::Value::U64(*damage as u64)]),
        );

        core_battle_logs::damage(
            &mut context.target_context()?,
            source_handle,
            Some(&effect_handle),
        )?;

        apply_drain(&mut context, *damage)?;
    }
    Ok(())
}

pub fn apply_drain(context: &mut ApplyingEffectContext, damage: u16) -> Result<(), Error> {
    if let Some(Some(drain_percent)) = context
        .effect()
        .active_move()
        .map(|active_move| active_move.data.drain_percent)
    {
        let target_handle = context.target_handle();
        if let Some(mut context) = context.source_context()? {
            let amount = drain_percent * damage;
            let amount = amount.round();
            heal(
                &mut context,
                amount,
                Some(target_handle),
                Some(&EffectHandle::Condition(Id::from_known("drain"))),
            )?;
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
    if healed > 0 {
        core_battle_logs::heal(context, source, effect)?;
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
    let switch_type = context.mon().force_switch.unwrap_or_default();
    switch_in(&mut context, position, Some(switch_type))?;
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
    let mut log_failure = !is_self && !is_secondary;

    let mut did_anything = targets
        .iter()
        .map(|target| target.outcome)
        .reduce(|acc, outcome| acc.combine(outcome))
        .unwrap_or(MoveOutcomeOnTarget::Unknown);

    for target in targets.iter_mut() {
        if target.outcome.failed() {
            continue;
        }

        let mut target_context = context.target_context(target.handle)?;

        let source_handle = target_context.mon_handle();
        let effect_handle = target_context.as_active_move_context().effect_handle();

        let mut hit_effect_outcome = MoveOutcomeOnTarget::Unknown;

        if let Some(hit_effect) = target_context.hit_effect().cloned() {
            // Only run the following effects if we hit the target.
            //
            // In other words, this check skips applying effects to substitutes.
            if target.outcome.hit_target() {
                if !target_context.target_mon().fainted {
                    if let Some(boosts) = hit_effect.boosts {
                        let outcome = boost(
                            &mut target_context.target_mon_context()?,
                            boosts,
                            Some(source_handle),
                            Some(&effect_handle),
                            is_secondary,
                            is_self,
                        )?;
                        let outcome = MoveOutcomeOnTarget::from(outcome);
                        hit_effect_outcome = hit_effect_outcome.combine(outcome);
                    }

                    if let Some(heal_percent) = hit_effect.heal_percent {
                        if target_context.target_mon().hp >= target_context.target_mon().max_hp {
                            core_battle_logs::fail_heal(&mut target_context.target_mon_context()?)?;
                            core_battle_logs::do_not_animate_last_move(
                                target_context.as_battle_context_mut(),
                            );
                        } else {
                            let damage = heal_percent * target_context.mon().max_hp;
                            let damage = damage.round();
                            heal(
                                &mut target_context.target_mon_context()?,
                                damage,
                                Some(source_handle),
                                Some(&effect_handle),
                            )?;
                            hit_effect_outcome = MoveOutcomeOnTarget::Success;
                        }
                    }
                }

                if let Some(status) = hit_effect.status {
                    let set_status = try_set_status(
                        &mut target_context.applying_effect_context()?,
                        Some(Id::from(status)),
                        !is_secondary && !is_self,
                    )?;
                    match set_status {
                        ApplyMoveEffectResult::Immune => log_failure = false,
                        _ => (),
                    }
                    let outcome = MoveOutcomeOnTarget::from(set_status.success());
                    hit_effect_outcome = hit_effect_outcome.combine(outcome);
                }

                if let Some(volatile_status) = hit_effect.volatile_status {
                    let added_volatile = try_add_volatile(
                        &mut target_context.applying_effect_context()?,
                        &Id::from(volatile_status),
                        !is_secondary && !is_self,
                    )?;
                    let outcome = MoveOutcomeOnTarget::from(added_volatile);
                    hit_effect_outcome = hit_effect_outcome.combine(outcome);
                }

                if let Some(side_condition) = hit_effect.side_condition {
                    let added_side_condition = add_side_condition(
                        &mut target_context
                            .applying_effect_context()?
                            .side_effect_context()?,
                        &Id::from(side_condition),
                    )?;
                    let outcome = MoveOutcomeOnTarget::from(added_side_condition);
                    hit_effect_outcome = hit_effect_outcome.combine(outcome);
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
                    hit_effect_outcome = hit_effect_outcome.combine(outcome);
                }
            }
        }

        if !target_context.is_self() {
            // These event callbacks run regardless of if there is a hit effect defined.
            let move_target = target_context.active_move().data.target;
            let target_handle = target_context.target_mon_handle();
            match move_target {
                MoveTarget::All => {
                    if let Some(hit_result) =
                        core_battle_effects::run_active_move_event_expecting_bool(
                            target_context.as_active_move_context_mut(),
                            fxlang::BattleEvent::HitField,
                            core_battle_effects::MoveTargetForEvent::Field,
                        )
                    {
                        let outcome = MoveOutcomeOnTarget::from(hit_result);
                        hit_effect_outcome = hit_effect_outcome.combine(outcome);
                    }
                }
                MoveTarget::AllySide | MoveTarget::AllyTeam => {
                    let side = target_context.side().index;
                    if let Some(hit_result) =
                        core_battle_effects::run_active_move_event_expecting_bool(
                            target_context.as_active_move_context_mut(),
                            fxlang::BattleEvent::HitSide,
                            core_battle_effects::MoveTargetForEvent::Side(side),
                        )
                    {
                        let outcome = MoveOutcomeOnTarget::from(hit_result);
                        hit_effect_outcome = hit_effect_outcome.combine(outcome);
                    }
                }
                MoveTarget::FoeSide => {
                    let side = target_context.foe_side().index;
                    if let Some(hit_result) =
                        core_battle_effects::run_active_move_event_expecting_bool(
                            target_context.as_active_move_context_mut(),
                            fxlang::BattleEvent::HitSide,
                            core_battle_effects::MoveTargetForEvent::Side(side),
                        )
                    {
                        let outcome = MoveOutcomeOnTarget::from(hit_result);
                        hit_effect_outcome = hit_effect_outcome.combine(outcome);
                    }
                }
                _ => {
                    if let Some(hit_result) =
                        core_battle_effects::run_active_move_event_expecting_bool(
                            target_context.as_active_move_context_mut(),
                            fxlang::BattleEvent::Hit,
                            core_battle_effects::MoveTargetForEvent::Mon(target_handle),
                        )
                    {
                        let outcome = MoveOutcomeOnTarget::from(hit_result);
                        hit_effect_outcome = hit_effect_outcome.combine(outcome);
                    }

                    // Run the event for other effects only once.
                    if !target_context.is_secondary() {
                        core_battle_effects::run_event_for_applying_effect(
                            &mut target_context.applying_effect_context()?,
                            fxlang::BattleEvent::Hit,
                            fxlang::VariableInput::default(),
                        );
                    }
                }
            }
        }

        // Some move effects function like HitEffect properties, but don't make much sense to be
        // generic.
        //
        // If we are checking the primary hit event on the targets, we should check these as well.
        if !is_secondary && !is_self {
            if let Some(self_destruct_type) = &context.active_move().data.self_destruct {
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
                hit_effect_outcome = hit_effect_outcome.combine(outcome);
            }
        }

        // The target's outcome is affected by the outcome here.
        target.outcome = target.outcome.combine(hit_effect_outcome);
        did_anything = did_anything.combine(hit_effect_outcome);
    }

    if did_anything.failed() {
        if !is_self && !is_secondary {
            // This is the primary hit of the move, and it failed to do anything, so the move failed
            // as a whole.
            core_battle_logs::do_not_animate_last_move(context.as_battle_context_mut());
            if log_failure {
                core_battle_logs::fail(context.as_mon_context_mut())?;
            }
        }
    } else if context.active_move().data.user_switch.is_some() && context.mon().hp > 0 {
        context.mon_mut().needs_switch = context.active_move().data.user_switch;
    }

    Ok(())
}

pub fn boost(
    context: &mut MonContext,
    original_boosts: BoostTable,
    source: Option<MonHandle>,
    effect: Option<&EffectHandle>,
    is_secondary: bool,
    is_self: bool,
) -> Result<bool, Error> {
    if context.mon().hp == 0
        || !context.mon().active
        || Side::mons_left(context.as_side_context()) == 0
    {
        return Ok(false);
    }
    // TODO: ChangeBoost event.
    let capped_boosts = Mon::cap_boosts(context, original_boosts.clone());
    let boosts = match effect {
        Some(effect_handle) => {
            core_battle_effects::run_event_for_applying_effect_expecting_boost_table(
                &mut context.applying_effect_context(effect_handle.clone(), source, None)?,
                fxlang::BattleEvent::TryBoost,
                capped_boosts.clone(),
            )
        }
        None => core_battle_effects::run_event_for_mon_expecting_boost_table(
            context,
            fxlang::BattleEvent::TryBoost,
            capped_boosts.clone(),
        ),
    };

    let mut success = false;
    for (boost, value) in BoostMapInOrderIterator::new(&boosts) {
        let original_delta = original_boosts.get(boost);
        let user_intended = original_delta != 0;
        let capped = original_delta != 0 && capped_boosts.get(boost) == 0;
        let suppressed = value == 0;

        let delta = Mon::boost_stat(context, boost, value);
        success = success || delta != 0;

        // We are careful to only log stat changes that should be visible to the user.
        if delta != 0 || capped || (!is_secondary && !is_self && user_intended && !suppressed) {
            core_battle_logs::boost(context, boost, delta, original_delta)?;
        } else if let Some(effect) = effect {
            let effect_context = context
                .as_battle_context_mut()
                .effect_context(effect.clone(), None)?;
            let effect_type = effect_context.effect().effect_type();
            if effect_type == EffectType::Ability {
                core_battle_logs::boost(context, boost, delta, original_delta)?;
            }
        }
    }

    // TODO: AfterBoost event.
    if success {
        if boosts.values().any(|val| val > 0) {
            context.mon_mut().stats_raised_this_turn = true;
        }
        if boosts.values().any(|val| val < 0) {
            context.mon_mut().stats_lowered_this_turn = true;
        }
        Ok(true)
    } else {
        Ok(false)
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
                    move_hit(&mut context, hit_targets_state_from_targets([mon_handle]))?;
                }
                if context.active_move().data.multihit.is_some() {
                    context.active_move_mut().primary_user_effect_applied = true;
                }
            } else {
                move_hit(&mut context, hit_targets_state_from_targets([mon_handle]))?;
            }
        } else {
            move_hit(&mut context, hit_targets_state_from_targets([mon_handle]))?;
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
                move_hit(&mut context, Vec::from_iter([target.clone()]))?;
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
        context.target_mon_mut().force_switch = Some(SwitchType::Normal);
    }

    Ok(())
}

fn initial_effect_state(
    context: &mut EffectContext,
    source: Option<MonHandle>,
) -> Result<fxlang::EffectState, Error> {
    let mut effect_state = fxlang::EffectState::new();
    effect_state.set_source_effect(
        context
            .effect_handle()
            .stable_effect_handle(context.as_battle_context())?,
    );
    if let Some(source_handle) = source {
        effect_state.set_source(source_handle);
        let mut context = context.as_battle_context_mut().mon_context(source_handle)?;
        effect_state.set_source_side(context.mon().side);
        if let Ok(source_position) = Mon::position_on_side(&mut context) {
            effect_state.set_source_position(source_position);
        }
    }
    Ok(effect_state)
}

#[derive(Clone, PartialEq, Eq)]
pub enum ApplyMoveEffectResult {
    Failed,
    Success,
    Immune,
}

impl ApplyMoveEffectResult {
    pub fn success(&self) -> bool {
        match self {
            Self::Success => true,
            _ => false,
        }
    }
}

pub fn try_set_status(
    context: &mut ApplyingEffectContext,
    status: Option<Id>,
    is_primary_move_effect: bool,
) -> Result<ApplyMoveEffectResult, Error> {
    if context.target().hp == 0 {
        return Ok(ApplyMoveEffectResult::Failed);
    }

    // A Mon may only have one status set at a time.
    match (&status, &context.target().status) {
        (Some(_), Some(_)) => {
            return Ok(ApplyMoveEffectResult::Failed);
        }
        _ => (),
    }

    // Cure the current status and return early.
    let status = match status {
        Some(status) => status,
        None => {
            context.target_mut().status = status;
            context.target_mut().status_state = fxlang::EffectState::new();
            return Ok(ApplyMoveEffectResult::Success);
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
        return Ok(ApplyMoveEffectResult::Immune);
    }

    // Save the previous status in case an effect callback cancels the status.
    let previous_status = context.target().status.clone();
    let previous_status_state = context.target().status_state.clone();

    if !core_battle_effects::run_event_for_applying_effect(
        context,
        fxlang::BattleEvent::SetStatus,
        fxlang::VariableInput::from_iter([fxlang::Value::Effect(status_effect_handle.clone())]),
    ) {
        return Ok(ApplyMoveEffectResult::Failed);
    }

    // Set the status so that the following effects can use it.
    context.target_mut().status = Some(status);

    let source_handle = context.source_handle();
    context.target_mut().status_state =
        initial_effect_state(context.as_effect_context_mut(), source_handle)?;

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
        return Ok(ApplyMoveEffectResult::Failed);
    }

    Ok(ApplyMoveEffectResult::Success)
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
) -> Result<ApplyMoveEffectResult, Error> {
    if context.target().hp == 0 || context.target().status.is_none() {
        return Ok(ApplyMoveEffectResult::Failed);
    }
    try_set_status(context, None, is_primary_move_effect)
}

pub fn cure_status(
    context: &mut ApplyingEffectContext,
    log_effect: bool,
) -> Result<ApplyMoveEffectResult, Error> {
    if context.target().hp == 0 {
        return Ok(ApplyMoveEffectResult::Failed);
    }
    match context.target().status.clone() {
        None => return Ok(ApplyMoveEffectResult::Failed),
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
        .unwrap_or(false));
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

    let source_handle = context.source_handle();
    let effect_state = initial_effect_state(context.as_effect_context_mut(), source_handle)?;
    context
        .target_mut()
        .volatiles
        .insert(status.clone(), effect_state);

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

pub fn remove_volatile(
    context: &mut ApplyingEffectContext,
    status: &Id,
    no_events: bool,
) -> Result<bool, Error> {
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

    if no_events {
        context.target_mut().volatiles.remove(&status);
        return Ok(true);
    }

    core_battle_effects::run_mon_volatile_event(context, fxlang::BattleEvent::End, &status);
    context.target_mut().volatiles.remove(&status);

    let volatile_name = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &status)?
        .name()
        .to_owned();
    core_battle_logs::remove_volatile(context, &volatile_name)?;

    Ok(true)
}

pub fn trap_mon(context: &mut MonContext) -> Result<(), Error> {
    if check_status_immunity(context, &Id::from_known("trapped"))? {
        return Ok(());
    }
    if context.mon().trapped {
        return Ok(());
    }
    context.mon_mut().trapped = true;

    Ok(())
}

pub fn calculate_confusion_damage(context: &mut MonContext, base_power: u32) -> Result<u16, Error> {
    let attack_stat = Stat::Atk;
    let defense_stat = Stat::Def;
    let attack_boosts = context.mon().boosts.get(attack_stat.try_into()?);
    let defense_boosts = context.mon().boosts.get(defense_stat.try_into()?);
    let attack = Mon::calculate_stat(
        context,
        attack_stat,
        attack_boosts,
        Fraction::from(1),
        context.mon_handle(),
        context.mon_handle(),
    )?;
    let defense = Mon::calculate_stat(
        context,
        defense_stat,
        defense_boosts,
        Fraction::from(1),
        context.mon_handle(),
        context.mon_handle(),
    )?;
    let level = context.mon().level as u32;
    let base_damage = 2 * level / 5 + 2;
    let base_damage = base_damage * base_power;
    let base_damage = base_damage * attack as u32;
    let base_damage = base_damage / defense as u32;
    let base_damage = base_damage / 50;
    let base_damage = base_damage + 2;
    let base_damage = context.battle_mut().randomize_base_damage(base_damage);
    Ok((base_damage as u16).max(1))
}

pub fn add_side_condition(context: &mut SideEffectContext, condition: &Id) -> Result<bool, Error> {
    let side_condition_handle = context
        .battle_mut()
        .get_effect_handle_by_id(condition)?
        .clone();
    let condition = side_condition_handle
        .try_id()
        .wrap_error_with_message("volatile must have an id")?
        .clone();

    if context.side().conditions.contains_key(&condition) {
        return Ok(
            core_battle_effects::run_side_condition_event_expecting_bool(
                context,
                fxlang::BattleEvent::SideRestart,
                &condition,
            )
            .unwrap_or(false),
        );
    }

    let effect_state = initial_effect_state(context.as_effect_context_mut(), None)?;
    context
        .side_mut()
        .conditions
        .insert(condition.clone(), effect_state);

    if let Some(side_condition) =
        CoreBattle::get_effect_by_handle(context.as_battle_context_mut(), &side_condition_handle)?
            .fxlang_condition()
    {
        if let Some(duration) = side_condition.duration {
            context
                .side_mut()
                .conditions
                .get_mut(&condition)
                .wrap_error_with_message("expected side condition state to exist")?
                .set_duration(duration);
        }

        if let Some(duration) = core_battle_effects::run_side_condition_event_expecting_u8(
            context,
            fxlang::BattleEvent::Duration,
            &condition,
        ) {
            context
                .side_mut()
                .conditions
                .get_mut(&condition)
                .wrap_error_with_message("expected side condition state to exist")?
                .set_duration(duration);
        }
    }

    if core_battle_effects::run_side_condition_event_expecting_bool(
        context,
        fxlang::BattleEvent::SideStart,
        &condition,
    )
    .is_some_and(|result| !result)
    {
        context.side_mut().conditions.remove(&condition);
    }

    let side_condition_name =
        CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &condition)?
            .name()
            .to_owned();
    core_battle_logs::add_side_condition(context, &side_condition_name)?;

    core_battle_effects::run_event_for_side_effect(
        context,
        fxlang::BattleEvent::SideConditionStart,
        fxlang::VariableInput::from_iter([fxlang::Value::Effect(side_condition_handle.clone())]),
    );

    Ok(true)
}

pub fn remove_side_condition(
    context: &mut SideEffectContext,
    condition: &Id,
) -> Result<bool, Error> {
    let side_condition_handle = context
        .battle_mut()
        .get_effect_handle_by_id(condition)?
        .clone();
    let condition = side_condition_handle
        .try_id()
        .wrap_error_with_message("volatile must have an id")?
        .clone();

    if !context.side().conditions.contains_key(&condition) {
        return Ok(false);
    }

    core_battle_effects::run_side_condition_event(
        context,
        fxlang::BattleEvent::SideEnd,
        &condition,
    );
    context.side_mut().conditions.remove(&condition);

    let condition_name = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &condition)?
        .name()
        .to_owned();
    core_battle_logs::remove_side_conditions(context, &condition_name)?;

    Ok(true)
}
