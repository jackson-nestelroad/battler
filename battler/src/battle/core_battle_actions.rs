use std::ops::Deref;

use lazy_static::lazy_static;

use crate::{
    battle::{
        core_battle_effects,
        core_battle_logs,
        modify_32,
        AbilitySlot,
        Action,
        ActiveMoveContext,
        ActiveTargetContext,
        ApplyingEffectContext,
        BattleQueue,
        BoostTable,
        BoostTableEntries,
        Context,
        CoreBattle,
        EffectContext,
        ExperienceAction,
        FieldEffectContext,
        ItemSlot,
        LevelUpAction,
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
        SwitchEventsAction,
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
    conditions::ConditionType,
    effect::{
        fxlang,
        EffectHandle,
        EffectType,
    },
    mons::{
        MoveSource,
        Stat,
        Type,
    },
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

/// The state of a move hit against a target.
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

fn switch_out(context: &mut MonContext, run_switch_out_events: bool) -> Result<bool, Error> {
    if context.mon().hp > 0 {
        if run_switch_out_events {
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
        }

        core_battle_effects::run_mon_ability_event(
            &mut context.applying_effect_context(
                EffectHandle::Condition(Id::from_known("switchout")),
                None,
                None,
            )?,
            fxlang::BattleEvent::End,
        );

        Mon::clear_volatile(context, true)?;

        core_battle_effects::run_event_for_mon(
            context,
            fxlang::BattleEvent::Exit,
            fxlang::VariableInput::default(),
        );
    }

    Mon::switch_out(context)?;
    Ok(true)
}

/// Switches a Mon into the given position.
pub fn switch_in(
    context: &mut MonContext,
    position: usize,
    mut switch_type: Option<SwitchType>,
    is_drag: bool,
) -> Result<bool, Error> {
    if context.mon_mut().active {
        core_battle_logs::hint(
            context.as_battle_context_mut(),
            "A switch failed because the Mon trying to switch in is already in.",
        )?;
        return Ok(false);
    }

    let active_len = context.player().total_active_positions();
    if position >= active_len {
        return Err(battler_error!(
            "invalid switch position {position} / {active_len}"
        ));
    }

    if let Some(previous_mon) = context.player().active_mon_handle(position) {
        let mut context = context.as_battle_context_mut().mon_context(previous_mon)?;
        if context.mon().hp > 0 {
            if let Some(previous_mon_switch_type) = context.mon().needs_switch {
                switch_type = Some(previous_mon_switch_type);
            }

            context.mon_mut().being_called_back = true;

            if !switch_out(&mut context, true)? {
                return Ok(false);
            }

            if let Some(SwitchType::CopyVolatile) = switch_type {
                // TODO: Copy volatiles to the new Mon.
            }
        }
    }

    Mon::switch_in(context, position)?;

    core_battle_logs::switch(context, is_drag)?;

    if is_drag {
        // The Mon was dragged in, so all events run immediately, potentially in the context of a
        // running move.
        run_switch_in_events(context)?;
    } else {
        // Otherwise, run events later, as a separate part of the battle.
        let mon_handle = context.mon_handle();
        BattleQueue::insert_action_into_sorted_position(
            context.as_battle_context_mut(),
            Action::SwitchEvents(SwitchEventsAction::new(mon_handle)),
        )?;
    }

    Ok(true)
}

/// Runs events corresponding to a Mon switching into battle.
pub fn run_switch_in_events(context: &mut MonContext) -> Result<bool, Error> {
    core_battle_effects::run_event_for_mon(
        context,
        fxlang::BattleEvent::SwitchIn,
        fxlang::VariableInput::default(),
    );

    core_battle_effects::run_event_for_mon(
        context,
        fxlang::BattleEvent::EntryHazard,
        fxlang::VariableInput::default(),
    );

    if context.mon().hp == 0 {
        return Ok(false);
    }
    if !context.mon().fainted {
        core_battle_effects::run_mon_ability_event(
            &mut context.applying_effect_context(
                EffectHandle::Condition(Id::from_known("switchin")),
                None,
                None,
            )?,
            fxlang::BattleEvent::Start,
        );
        core_battle_effects::run_mon_item_event(
            &mut context.applying_effect_context(
                EffectHandle::Condition(Id::from_known("switchin")),
                None,
                None,
            )?,
            fxlang::BattleEvent::Start,
        );
    }

    Ok(true)
}

fn register_active_move_by_id(context: &mut Context, move_id: &Id) -> Result<MoveHandle, Error> {
    let active_move = context
        .battle_mut()
        .dex
        .moves
        .get_by_id(move_id)?
        .deref()
        .clone();
    register_active_move(context, active_move)
}

/// Registers a new active move.
pub fn register_active_move(context: &mut Context, active_move: Move) -> Result<MoveHandle, Error> {
    let active_move_handle = context.battle_mut().register_move(active_move);
    Ok(active_move_handle)
}

/// Executes the given move selected by a Mon.
pub fn do_move(
    context: &mut MonContext,
    move_id: &Id,
    target_location: Option<isize>,
    original_target: Option<MonHandle>,
) -> Result<(), Error> {
    context.mon_mut().active_move_actions += 1;

    do_move_internal(context, move_id, target_location, original_target)?;

    context.mon_mut().clear_active_move();

    Ok(())
}

fn do_move_internal(
    context: &mut MonContext,
    move_id: &Id,
    target_location: Option<isize>,
    original_target: Option<MonHandle>,
) -> Result<(), Error> {
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
    let mut context = context.active_move_context(active_move_handle)?;

    let locked_move_before = Mon::locked_move(context.as_mon_context_mut())?;

    // Check that move has enough PP to be used.
    let move_id = context.active_move().id().clone();
    if locked_move_before.is_none()
        && !context.mon_mut().check_pp(&move_id, 1)
        && !move_id.eq("struggle")
    {
        // No PP, so this move action cannot be carried through.
        let move_name = &context.active_move().data.name;
        // SAFETY: Logging does not change the active move.
        let move_name = unsafe { move_name.unsafely_detach_borrow() };
        core_battle_logs::cant(context.as_mon_context_mut(), "nopp", Some(move_name))?;
        return Ok(());
    }

    // The move is going to be used, so remember the choices made here. This is important for
    // locking moves.
    if locked_move_before.is_none() {
        context.mon_mut().last_move_selected = Some(move_id.clone());
        context.mon_mut().last_move_target_location = target_location;
    }

    // Use the move.
    use_active_move(
        context.as_mon_context_mut(),
        active_move_handle,
        target,
        None,
        false,
        true,
    )?;

    let this_move_is_the_last_selected = context
        .mon()
        .last_move_selected
        .as_ref()
        .is_some_and(|move_id| move_id == context.active_move().id());

    // Set the last move of the user only if they selected it for use.
    //
    // Locked moves (like Razor Wind) can be used externally by other moves (like Mirror Move). The
    // use of the locked move on the next turn will go through this logic, even though the user does
    // not own the move. This check prevents this.
    //
    // Without this check, there is a discrepency between how `last_move` is set for external moves:
    // moves that require multple turns get set as the last move while single-turn moves do not.
    //
    // If you really want the last move used regardless of selection, you should use
    // `last_move_used`, which is set for all external moves on any turn with no preconditions.
    if this_move_is_the_last_selected {
        // Some moves, like charging moves, do not count as the last move until the last turn.
        let set_last_move = core_battle_effects::run_event_for_mon(
            context.as_mon_context_mut(),
            fxlang::BattleEvent::SetLastMove,
            fxlang::VariableInput::default(),
        );
        if set_last_move {
            context.mon_mut().last_move = Some(active_move_handle);
        }
    }

    // Deduct PP if the Mon selected this move for use, and the Mon was not forced to use it, or if
    // the move is a charging move.
    //
    // Note that charging moves have their PP deducted on the last turn of use, as opposed to the
    // first (default). The effect of such a move should hook into this event to ensure PP is not
    // continually deducted every turn.
    if this_move_is_the_last_selected && locked_move_before.is_none()
        || context
            .active_move()
            .data
            .flags
            .contains(&MoveFlags::Charge)
    {
        let deduction = core_battle_effects::run_event_for_mon_expecting_u8(
            context.as_mon_context_mut(),
            fxlang::BattleEvent::DeductPp,
            1,
        );
        if deduction > 0 {
            // TODO: DeductPP event to interrupt this (or set deduction to 0).
            let move_id = context.active_move().id();
            // SAFETY: move_id is only used for lookup.
            let move_id = unsafe { move_id.unsafely_detach_borrow() };
            context.mon_mut().deduct_pp(move_id, 1);
        }
    }

    core_battle_effects::run_active_move_event_expecting_void(
        &mut context,
        fxlang::BattleEvent::AfterMove,
        core_battle_effects::MoveTargetForEvent::User,
        fxlang::VariableInput::default(),
    );
    core_battle_effects::run_event_for_applying_effect(
        &mut context.user_applying_effect_context(None)?,
        fxlang::BattleEvent::AfterMove,
        fxlang::VariableInput::default(),
    );

    Ok(())
}

/// Uses a move.
pub fn use_move(
    context: &mut MonContext,
    move_id: &Id,
    target: Option<MonHandle>,
    source_effect: Option<&EffectHandle>,
    external: bool,
) -> Result<bool, Error> {
    let active_move_handle = register_active_move_by_id(context.as_battle_context_mut(), move_id)?;
    use_active_move(
        context,
        active_move_handle,
        target,
        source_effect,
        external,
        true,
    )
}

/// Uses a move that was already registered as an active move.
pub fn use_active_move(
    context: &mut MonContext,
    active_move_handle: MoveHandle,
    target: Option<MonHandle>,
    source_effect: Option<&EffectHandle>,
    external: bool,
    directly_used: bool,
) -> Result<bool, Error> {
    if directly_used {
        context.mon_mut().move_this_turn_outcome = None;
        context.mon_mut().set_active_move(active_move_handle);
    }

    let mut context = context.active_move_context(active_move_handle)?;
    context.active_move_mut().source_effect = source_effect.cloned();
    context.active_move_mut().used_by = Some(context.mon_handle());
    context.active_move_mut().external = external;

    // BeforeMove event handlers can prevent the move from being used.
    if !external
        && directly_used
        && !core_battle_effects::run_event_for_applying_effect(
            &mut context.user_applying_effect_context(None)?,
            fxlang::BattleEvent::BeforeMove,
            fxlang::VariableInput::default(),
        )
    {
        core_battle_effects::run_event_for_applying_effect(
            &mut context.user_applying_effect_context(None)?,
            fxlang::BattleEvent::MoveAborted,
            fxlang::VariableInput::default(),
        );
        context.mon_mut().move_this_turn_outcome = Some(MoveOutcome::Failed);
        return Ok(false);
    }

    if directly_used {
        context.mon_mut().last_move_used = Some(context.active_move_handle());
    }

    let outcome = use_active_move_internal(&mut context, target, directly_used)?;

    if directly_used {
        context.mon_mut().move_this_turn_outcome =
            match (context.mon_mut().move_this_turn_outcome, outcome) {
                (
                    left @ Some(MoveOutcome::Success | MoveOutcome::Skipped),
                    MoveOutcome::Skipped | MoveOutcome::Failed,
                ) => left,
                _ => Some(outcome),
            };
    }

    Ok(outcome.success())
}

fn use_active_move_internal(
    context: &mut ActiveMoveContext,
    mut target: Option<MonHandle>,
    directly_used: bool,
) -> Result<MoveOutcome, Error> {
    core_battle_effects::run_active_move_event_expecting_void(
        context,
        fxlang::BattleEvent::ModifyType,
        core_battle_effects::MoveTargetForEvent::User,
        fxlang::VariableInput::default(),
    );
    let use_move_input = fxlang::VariableInput::from_iter([target
        .map(fxlang::Value::Mon)
        .unwrap_or(fxlang::Value::Undefined)]);
    core_battle_effects::run_active_move_event_expecting_void(
        context,
        fxlang::BattleEvent::UseMove,
        core_battle_effects::MoveTargetForEvent::User,
        use_move_input.clone(),
    );

    core_battle_effects::run_event_for_applying_effect(
        &mut context.user_applying_effect_context(None)?,
        fxlang::BattleEvent::ModifyType,
        fxlang::VariableInput::default(),
    );
    core_battle_effects::run_event_for_applying_effect(
        &mut context.user_applying_effect_context(None)?,
        fxlang::BattleEvent::UseMove,
        use_move_input,
    );

    // TODO: Prevent moves if Mon faints at this point? Would this ever happen?

    let targets = get_move_targets(context, target)?;
    if context.active_move().data.target.has_single_target() {
        target = targets.first().cloned();
    }

    // Log that the move is being used.
    let move_name = context.active_move().data.name.clone();
    core_battle_logs::use_move(
        context.as_mon_context_mut(),
        &move_name,
        target,
        !directly_used,
    )?;

    if context.active_move().data.target.requires_target() && target.is_none() {
        core_battle_logs::last_move_had_no_target(context.as_battle_context_mut());
        core_battle_logs::fail(context.as_mon_context_mut())?;
        return Ok(MoveOutcome::Failed);
    }

    // TODO: Targeted event.

    let try_move_result = core_battle_effects::run_active_move_event_expecting_move_event_result(
        context,
        fxlang::BattleEvent::TryMove,
        core_battle_effects::MoveTargetForEvent::UserWithTarget(target),
    );
    let try_move_result = if try_move_result.advance() {
        core_battle_effects::run_event_for_applying_effect_expecting_move_event_result(
            &mut context.user_applying_effect_context(target)?,
            fxlang::BattleEvent::TryMove,
        )
    } else {
        try_move_result
    };
    if !try_move_result.advance() {
        if try_move_result.failed() {
            core_battle_logs::do_not_animate_last_move(context.as_battle_context_mut());
            core_battle_logs::fail(context.as_mon_context_mut())?;
        }
        return Ok(MoveOutcome::Failed);
    }

    core_battle_effects::run_active_move_event_expecting_void(
        context,
        fxlang::BattleEvent::UseMoveMessage,
        core_battle_effects::MoveTargetForEvent::User,
        fxlang::VariableInput::default(),
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
        core_battle_logs::do_not_animate_last_move(context.as_battle_context_mut());

        core_battle_effects::run_active_move_event_expecting_void(
            context,
            fxlang::BattleEvent::MoveFailed,
            core_battle_effects::MoveTargetForEvent::User,
            fxlang::VariableInput::default(),
        );
    }

    Ok(outcome)
}

/// Faints a Mon.
pub fn faint(
    context: &mut MonContext,
    source: Option<MonHandle>,
    effect: Option<&EffectHandle>,
) -> Result<(), Error> {
    Mon::faint(context, source, effect)
}

/// Gets all of the targets of a move.
fn get_move_targets(
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
                target =
                    core_battle_effects::run_event_for_applying_effect_expecting_mon_quick_return(
                        &mut context.user_applying_effect_context(Some(target))?,
                        fxlang::BattleEvent::RedirectTarget,
                        fxlang::VariableInput::from_iter([fxlang::Value::Mon(target)]),
                    )
                    .unwrap_or(target);
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

/// Runs all events prior to a move hitting any targets.
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

/// Tries to use an indirect move against some aspect of the battle field, such as a side or the
/// field itself.
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

struct MoveStepResult {
    targets: Vec<direct_move_step::MoveStepTarget>,
    at_least_one_failure: bool,
}

fn apply_move_step_against_targets(
    context: &mut ActiveMoveContext,
    mut result: MoveStepResult,
    move_step: direct_move_step::DirectMoveStep,
) -> Result<MoveStepResult, Error> {
    move_step(context, result.targets.as_mut_slice())?;
    let at_least_one_failure = result.targets.iter().any(|target| target.outcome.failed());
    let targets = result
        .targets
        .into_iter()
        .filter(|target| target.outcome.success())
        .collect();
    Ok(MoveStepResult {
        targets,
        at_least_one_failure: result.at_least_one_failure | at_least_one_failure,
    })
}

fn prepare_direct_move_against_targets(
    context: &mut ActiveMoveContext,
    targets: &[MonHandle],
) -> Result<MoveStepResult, Error> {
    lazy_static! {
        static ref STEPS: Vec<direct_move_step::DirectMoveStep> = vec![
            direct_move_step::check_targets_invulnerability,
            direct_move_step::check_try_hit_event,
            direct_move_step::check_type_immunity,
            direct_move_step::check_general_immunity,
            direct_move_step::handle_accuracy,
            direct_move_step::break_protect,
            // TODO: Boost stealing would happen at this stage.
        ];
    }

    let mut result = MoveStepResult {
        targets: targets
            .iter()
            .map(|target| direct_move_step::MoveStepTarget {
                handle: *target,
                outcome: MoveOutcome::Success,
            })
            .collect(),
        at_least_one_failure: false,
    };
    for step in &*STEPS {
        result = apply_move_step_against_targets(context, result, *step)?;
        if result.targets.is_empty() {
            break;
        }
    }

    Ok(result)
}

/// Prepares to use a move directly against several target Mons.
pub fn prepare_direct_move(
    context: &mut ActiveMoveContext,
    targets: &[MonHandle],
) -> Result<Vec<MonHandle>, Error> {
    let result = prepare_direct_move_against_targets(context, targets)?;
    Ok(result
        .targets
        .into_iter()
        .map(|target| target.handle)
        .collect())
}

/// Tries to use a move directly against several target Mons.
fn try_direct_move(
    context: &mut ActiveMoveContext,
    targets: &[MonHandle],
) -> Result<MoveOutcome, Error> {
    if targets.len() > 1 && !context.active_move().data.smart_target {
        context.active_move_mut().spread_hit = true;
    }

    if let Some(try_use_move_outcome) = run_try_use_move_events(context)? {
        return Ok(try_use_move_outcome);
    }

    let prepare_result = prepare_direct_move_against_targets(context, targets)?;
    let result = if !prepare_result.targets.is_empty() {
        apply_move_step_against_targets(context, prepare_result, *&direct_move_step::move_hit_loop)?
    } else {
        prepare_result
    };

    let outcome = if result.targets.is_empty() {
        if result.at_least_one_failure {
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
            result.targets.into_iter().map(|target| target.handle),
        )?;
    }

    Ok(outcome)
}

/// Hits all targets and determines if the move was a success.
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

/// Hits the given targets with a move, recording the state of the hit.
fn move_hit(
    context: &mut ActiveMoveContext,
    mut hit_targets_state: Vec<HitTargetState>,
) -> Result<Vec<HitTargetState>, Error> {
    hit_targets(context, hit_targets_state.as_mut_slice())?;
    Ok(hit_targets_state)
}

/// Hits all targets with a move, recording the state of the hit.
///
/// This function will run once for each "hit effect" of the move.
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
            target.outcome = if try_move_result.failed() {
                MoveOutcomeOnTarget::Failure
            } else {
                MoveOutcomeOnTarget::Unknown
            };
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
            fxlang::VariableInput::from_iter([fxlang::Value::UFraction(
                target.outcome.damage().into(),
            )]),
        );
        core_battle_effects::run_active_move_event_expecting_void(
            context,
            fxlang::BattleEvent::AfterHit,
            core_battle_effects::MoveTargetForEvent::Mon(target.handle),
            fxlang::VariableInput::default(),
        );
    }

    Ok(())
}

/// Tries the primary hit of the move.
///
/// This event can be overridden for special moves like "Substitute." If the move hits a Substitute,
/// the target is invulnerable to the rest of the move's hit effects.
///
/// For practically every other move, this function is a no-op.
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

/// Calculates the damage a move will deal against multiple targets.
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

/// Calculates damage for an active move on a target.
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
        fxlang::BattleEvent::MoveDamage,
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
        fxlang::BattleEvent::MoveBasePower,
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
    let crit_chance = if crit_ratio > 0 {
        let crit_chance = [0u32, 24, 8, 2, 1][crit_ratio as usize];
        core_battle_effects::run_event_for_applying_effect_expecting_u32(
            &mut context.applying_effect_context()?,
            fxlang::BattleEvent::ModifyCritChance,
            crit_chance,
        )
    } else {
        0
    };

    context.active_move_mut().hit_data(target_mon_handle).crit =
        context.active_move().data.will_crit
            || (crit_chance > 0
                && rand_util::chance(context.battle_mut().prng.as_mut(), 1, crit_chance as u64));

    if context.active_move_mut().hit_data(target_mon_handle).crit {
        // TODO: CriticalHit event.
    }

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
        Fraction::from(1u16),
        move_user,
    )?;
    let defense = Mon::calculate_stat(
        &mut context.defender_context()?,
        defense_stat,
        defense_boosts,
        Fraction::from(1u16),
        move_target,
    )?;

    let base_damage = 2 * (level as u32) / 5 + 2;
    let base_damage = base_damage * base_power * (attack as u32);
    let base_damage = base_damage / (defense as u32);
    let base_damage = base_damage / 50;

    // Damage modifiers.
    modify_damage(context, base_damage)
}

fn type_effectiveness(context: &mut ActiveTargetContext) -> Result<i8, Error> {
    if context.active_move().data.typeless {
        return Ok(0);
    }

    let move_type = context.active_move().data.primary_type;
    let target_handle = context.target_mon_handle();
    let mut total = 0;
    for defense in Mon::types(&mut context.target_mon_context()?)? {
        let modifier = context
            .battle()
            .check_type_effectiveness(move_type, defense);
        let modifier = core_battle_effects::run_active_move_event_expecting_i8(
            context.as_active_move_context_mut(),
            fxlang::BattleEvent::Effectiveness,
            core_battle_effects::MoveTargetForEvent::Mon(target_handle),
            fxlang::VariableInput::from_iter([
                fxlang::Value::Fraction(modifier.into()),
                fxlang::Value::Type(defense),
            ]),
        )
        .unwrap_or(modifier);
        let modifier = core_battle_effects::run_event_for_applying_effect_expecting_i8(
            &mut context.applying_effect_context()?,
            fxlang::BattleEvent::Effectiveness,
            modifier,
            fxlang::VariableInput::from_iter([fxlang::Value::Type(defense)]),
        );
        total += modifier;
    }
    Ok(total)
}

/// Modifies the damage dealt against a target.
fn modify_damage(
    context: &mut ActiveTargetContext,
    mut base_damage: u32,
) -> Result<MoveOutcomeOnTarget, Error> {
    base_damage += 2;
    if context.active_move().spread_hit {
        let spread_modifier = Fraction::new(3, 4);
        base_damage = modify_32(base_damage, spread_modifier);
    }

    // Weather modifiers.
    base_damage = core_battle_effects::run_event_for_applying_effect_expecting_u32(
        &mut context.user_applying_effect_context()?,
        fxlang::BattleEvent::WeatherModifyDamage,
        base_damage,
    );

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
    let type_modifier = type_effectiveness(context)?;
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

/// Calculates recoil damage of a move against the user.
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

/// Applies recoil damage to the user of an active move.
pub fn apply_recoil_damage(
    context: &mut ActiveMoveContext,
    damage_dealt: u64,
) -> Result<(), Error> {
    let recoil_damage = calculate_recoil_damage(context, damage_dealt);
    if recoil_damage > 0 {
        let recoil_damage = recoil_damage.min(u16::MAX as u64) as u16;
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

    use super::check_immunity;
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
            WrapResultError,
        },
        effect::fxlang,
        moves::{
            Accuracy,
            MoveCategory,
            MoveTarget,
            MultihitType,
            OhkoType,
        },
        rng::rand_util,
    };

    /// The outcome of a move step against a target.
    pub struct MoveStepTarget {
        pub handle: MonHandle,
        pub outcome: MoveOutcome,
    }

    /// The interface for any direct move step.
    pub type DirectMoveStep =
        fn(&mut ActiveMoveContext, &mut [MoveStepTarget]) -> Result<(), Error>;

    /// Checks if targets are invulnerable.
    pub fn check_targets_invulnerability(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        for target in targets {
            if core_battle_effects::run_event_for_applying_effect_expecting_bool_quick_return(
                &mut context.applying_effect_context_for_target(target.handle)?,
                fxlang::BattleEvent::Invulnerability,
            ) {
                target.outcome = MoveOutcome::Failed;
                core_battle_logs::miss(&mut context.target_mon_context(target.handle)?)?;
            }
        }
        Ok(())
    }

    /// Checks the "TryHit" event for each target.
    pub fn check_try_hit_event(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        for target in targets.iter_mut() {
            let result =
                core_battle_effects::run_event_for_applying_effect_expecting_move_event_result(
                    &mut context.applying_effect_context_for_target(target.handle)?,
                    fxlang::BattleEvent::TryHit,
                );
            if !result.advance() {
                target.outcome = if result.failed() {
                    MoveOutcome::Failed
                } else {
                    MoveOutcome::Skipped
                }
            }
        }
        if targets.iter().all(|target| target.outcome.failed()) {
            core_battle_logs::do_not_animate_last_move(context.as_battle_context_mut());
            core_battle_logs::fail(context.as_mon_context_mut())?;
        }
        Ok(())
    }

    /// Checks for type immunity.
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

    /// Checks for general immunity, which is communicate through effect callbacks.
    ///
    /// Types have their own effect callbacks for special immunities (e.g., Grass types are immune
    /// to powder moves).
    pub fn check_general_immunity(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        for target in targets {
            let immune = !core_battle_effects::run_active_move_event_expecting_bool(
                context,
                fxlang::BattleEvent::TryImmunity,
                core_battle_effects::MoveTargetForEvent::Mon(target.handle),
            )
            .unwrap_or(true)
                || check_immunity(&mut context.applying_effect_context_for_target(target.handle)?)?;

            if immune {
                core_battle_logs::immune(&mut context.target_mon_context(target.handle)?)?;
                target.outcome = MoveOutcome::Failed;
            }
        }
        Ok(())
    }

    /// Applies an accuracy check to each target.
    pub fn handle_accuracy(
        context: &mut ActiveMoveContext,
        targets: &mut [MoveStepTarget],
    ) -> Result<(), Error> {
        for target in targets {
            let mut context = context.target_context(target.handle)?;
            if !accuracy_check(&mut context)? {
                target.outcome = MoveOutcome::Failed;
            }
        }
        Ok(())
    }

    /// Runs a single accuracy check of a move against a target.
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
            if let Accuracy::Chance(accuracy) = &mut accuracy {
                *accuracy = core_battle_effects::run_event_for_applying_effect_expecting_u8(
                    &mut context.applying_effect_context()?,
                    fxlang::BattleEvent::ModifyAccuracy,
                    *accuracy,
                );
                let mut boost = 0;
                if !context.active_move().data.ignore_accuracy {
                    let boosts = context.mon().boosts.clone();
                    let boosts = core_battle_effects::run_event_for_mon_expecting_boost_table(
                        context.as_mon_context_mut(),
                        fxlang::BattleEvent::ModifyBoosts,
                        boosts,
                    );
                    boost = boosts.acc.max(-6).min(6);
                }
                if !context.active_move().data.ignore_evasion {
                    let boosts = context.target_mon().boosts.clone();
                    let boosts = core_battle_effects::run_event_for_mon_expecting_boost_table(
                        &mut context.target_mon_context()?,
                        fxlang::BattleEvent::ModifyBoosts,
                        boosts,
                    );
                    boost = (boost - boosts.eva).max(-6).min(6);
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
            if core_battle_effects::run_event_for_applying_effect_expecting_bool_quick_return(
                &mut context.applying_effect_context()?,
                fxlang::BattleEvent::AccuracyExempt,
            ) {
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

    /// Breaks protect for each target.
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

    /// Hits each target for each hit of the move.
    ///
    /// Multi-hit moves hit each target multiple times.
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

            if context.active_move().hit > 1 {
                let target = if context.active_move().data.target.has_single_target() {
                    targets.first().map(|target| target.handle)
                } else {
                    None
                };
                let move_name = context.active_move().data.name.clone();
                core_battle_logs::use_move(context.as_mon_context_mut(), &move_name, target, true)?;
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
                fxlang::VariableInput::default(),
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

/// Applies direct damage to the Mon.
///
/// No events are run based on this damage. This type of damage should be an exception, for moves
/// like "Struggle."
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

/// Damages a Mon.
///
/// This is the normal path that damage should take.
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

/// Applies spread damage to multiple targets.
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

        if context.effect().id() != &Id::from("strugglerecoil") {
            if let Some(condition) = context.effect().condition() {
                if condition.data.condition_type == ConditionType::Weather {
                    if check_immunity(&mut context)? {
                        *damage = 0;
                        continue;
                    }
                }
            }
            *damage = core_battle_effects::run_event_for_applying_effect_expecting_u16(
                &mut context,
                fxlang::BattleEvent::Damage,
                *damage,
            );
            if *damage == 0 {
                continue;
            }
        }

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
            fxlang::VariableInput::from_iter([fxlang::Value::UFraction((*damage).into())]),
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

/// Applies the drain effect to the user of an effect.
pub fn apply_drain(context: &mut ApplyingEffectContext, damage: u16) -> Result<(), Error> {
    if let Some(Some(drain_percent)) = context
        .effect()
        .move_effect()
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

/// Heals a Mon.
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

/// Drags a random Mon into a player's position.
pub fn drag_in(context: &mut PlayerContext, position: usize) -> Result<bool, Error> {
    let old = context.player().active_mon_handle(position);

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
    switch_in(&mut context, position, Some(switch_type), true)?;
    Ok(true)
}

/// Applies the effects of a move's hit.
///
/// Run for each "hit effect" of a move.
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
                    let slot = Mon::position_on_side(&target_context.target_mon_context()?)?;
                    let added_slot_condition = add_slot_condition(
                        &mut target_context
                            .applying_effect_context()?
                            .side_effect_context()?,
                        slot,
                        &Id::from(slot_condition),
                    )?;
                    let outcome = MoveOutcomeOnTarget::from(added_slot_condition);
                    hit_effect_outcome = hit_effect_outcome.combine(outcome);
                }

                if let Some(weather) = hit_effect.weather {
                    let set_weather_success = set_weather(
                        &mut target_context
                            .applying_effect_context()?
                            .field_effect_context()?,
                        &Id::from(weather),
                    )?;
                    let outcome = MoveOutcomeOnTarget::from(set_weather_success);
                    hit_effect_outcome = hit_effect_outcome.combine(outcome);
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

/// Boosts the stats of a Mon.
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
    for (boost, value) in BoostTableEntries::new(&boosts) {
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

/// Applies an effect on the user of a move.
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
                    .unwrap_or(Fraction::from(1u16));
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

/// Applies all secondary effects of a move.
///
/// Secondary effects have some random chance connected to them and can have effects against targets
/// and the user.
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
            let chance = secondary_effect.chance.unwrap_or(Fraction::from(1u16));
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

/// Forces all targets of the move to switch out at the end of the move.
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
    target: Option<MonHandle>,
    source: Option<MonHandle>,
) -> Result<fxlang::EffectState, Error> {
    let mut effect_state = fxlang::EffectState::new();
    effect_state.set_source_effect(
        context
            .effect_handle()
            .stable_effect_handle(context.as_battle_context())?,
    );
    if let Some(target_handle) = target {
        effect_state.set_target(target_handle);
    }
    if let Some(source_handle) = source {
        effect_state.set_source(source_handle);
        let mut context = context.as_battle_context_mut().mon_context(source_handle)?;
        effect_state.set_source_side(context.mon().side);
        if let Ok(source_position) = Mon::position_on_side(&mut context) {
            effect_state.set_source_position(source_position)?;
        }
    }
    Ok(effect_state)
}

/// The result of applying a move effect.
///
/// Must be its own type because some effects handle immunity and failure differently.
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

/// Tries to set the status of a Mon.
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
            core_battle_effects::run_event_for_applying_effect(
                context,
                fxlang::BattleEvent::CureStatus,
                fxlang::VariableInput::default(),
            );
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

    if check_immunity(&mut context.forward_applying_effect_context(status_effect_handle.clone())?)?
    {
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

    let target_handle = context.target_handle();
    let source_handle = context.source_handle();
    context.target_mut().status_state = initial_effect_state(
        context.as_effect_context_mut(),
        Some(target_handle),
        source_handle,
    )?;

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
        return Ok(ApplyMoveEffectResult::Failed);
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

/// Checks the immunity of a Mon from an effect.
pub fn check_immunity(context: &mut ApplyingEffectContext) -> Result<bool, Error> {
    if context.target().hp == 0 {
        return Ok(true);
    }

    Ok(
        core_battle_effects::run_event_for_applying_effect_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::Immunity,
        ),
    )
}

/// Clears the status of a Mon.
///
/// Different from curing in that a message is not displayed.
pub fn clear_status(
    context: &mut ApplyingEffectContext,
    is_primary_move_effect: bool,
) -> Result<ApplyMoveEffectResult, Error> {
    if context.target().hp == 0 || context.target().status.is_none() {
        return Ok(ApplyMoveEffectResult::Failed);
    }
    try_set_status(context, None, is_primary_move_effect)
}

/// Cures the status of a Mon.
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

/// Tries to add the volatile effect to a Mon.
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

    if check_immunity(
        &mut context.forward_applying_effect_context(volatile_effect_handle.clone())?,
    )? {
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

    let target_handle = context.target_handle();
    let source_handle = context.source_handle();
    let effect_state = initial_effect_state(
        context.as_effect_context_mut(),
        Some(target_handle),
        source_handle,
    )?;
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
        return Ok(false);
    }

    let volatile_name = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &status)?
        .name()
        .to_owned();
    core_battle_logs::add_volatile(context, &volatile_name)?;

    Ok(true)
}

/// Removes a volatile effect from a Mon.
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

/// Tries to trap a Mon.
pub fn trap_mon(context: &mut MonContext) -> Result<(), Error> {
    let effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(&Id::from_known("trapped"))?
        .clone();
    if check_immunity(&mut context.applying_effect_context(effect_handle, None, None)?)? {
        return Ok(());
    }
    if context.mon().trapped {
        return Ok(());
    }
    context.mon_mut().trapped = true;

    Ok(())
}

/// Calculates confusion damage.
///
/// The games use a special damage formula for confusion damage, with less modifiers.
pub fn calculate_confusion_damage(context: &mut MonContext, base_power: u32) -> Result<u16, Error> {
    let attack_stat = Stat::Atk;
    let defense_stat = Stat::Def;
    let attack_boosts = context.mon().boosts.get(attack_stat.try_into()?);
    let defense_boosts = context.mon().boosts.get(defense_stat.try_into()?);
    let attack = Mon::calculate_stat(
        context,
        attack_stat,
        attack_boosts,
        Fraction::from(1u16),
        context.mon_handle(),
    )?;
    let defense = Mon::calculate_stat(
        context,
        defense_stat,
        defense_boosts,
        Fraction::from(1u16),
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

/// Adds a condition to a side.
pub fn add_side_condition(context: &mut SideEffectContext, condition: &Id) -> Result<bool, Error> {
    let side_condition_handle = context
        .battle_mut()
        .get_effect_handle_by_id(condition)?
        .clone();
    let condition = side_condition_handle
        .try_id()
        .wrap_error_with_message("side condition must have an id")?
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

    let source_handle = context.source_handle();
    let effect_state = initial_effect_state(context.as_effect_context_mut(), None, source_handle)?;
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
        return Ok(false);
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

/// Removes a condition from a side.
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
        .wrap_error_with_message("side condition must have an id")?
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
    core_battle_logs::remove_side_condition(context, &condition_name)?;

    Ok(true)
}

/// Sets the types of a Mon.
pub fn set_types(context: &mut ApplyingEffectContext, types: Vec<Type>) -> Result<bool, Error> {
    // TODO: SetTypes event (block Arceus and Silvally).
    if types.is_empty() {
        return Ok(false);
    }
    context.target_mut().types = types;
    let source = context.source_handle();
    let source_effect = context.source_effect_handle().cloned();
    let mut context = context.target_context()?;
    let types = &context.mon().types;
    // SAFETY: Types are not modified in the log statement.
    let types = unsafe { types.unsafely_detach_borrow() };
    core_battle_logs::type_change(&mut context, types, source, source_effect.as_ref())?;
    Ok(true)
}

/// Calculates the experience gained for the Mon for the given Mon fainting.
fn calculate_exp_gain(
    context: &mut MonContext,
    fainted_mon_handle: MonHandle,
) -> Result<u32, Error> {
    let mon_handle = context.mon_handle();
    let mon_level = context.mon().level as u32;
    let mon_happiness = context.mon().happiness;
    let fainted_mon = context.as_battle_context_mut().mon(fainted_mon_handle)?;
    let species = fainted_mon.base_species.clone();
    let target_level = fainted_mon.level as u32;
    let participated = fainted_mon.foes_fought_while_active.contains(&mon_handle);
    let base_exp_yield = context
        .battle()
        .dex
        .species
        .get(&species)
        .into_result()?
        .data
        .base_exp_yield;
    let exp = ((base_exp_yield as u32) * (target_level as u32)) / 5;
    let exp = if participated { exp } else { exp / 2 };
    let dynamic_scaling = Fraction::new(2 * target_level + 10, target_level + mon_level + 10);
    let dynamic_scaling = dynamic_scaling * dynamic_scaling * dynamic_scaling.sqrt();
    let exp = dynamic_scaling * exp;
    let exp = exp.floor() + 1;

    let exp = if mon_happiness > 220 {
        exp * 6 / 5
    } else {
        exp
    };

    let exp = if context.mon().different_original_trainer {
        exp * 3 / 2
    } else {
        exp
    };

    core_battle_effects::run_event_for_mon_expecting_u32(
        context,
        fxlang::BattleEvent::ModifyExperience,
        exp,
    );

    // TODO: Custom experience modifiers set on the battle itself, to simulate outside effects (like
    // Exp Charm).

    Ok(exp)
}

/// Levels up a Mon directly to the given level.
pub fn level_up(context: &mut MonContext, target_level: u8) -> Result<(), Error> {
    let old_level = context.mon().level;
    context.mon_mut().level = target_level;

    Mon::recalculate_stats(context)?;
    Mon::recalculate_base_stats(context)?;
    core_battle_logs::level_up(context)?;

    for level in old_level..target_level {
        context.mon_mut().happiness += match context.mon().happiness {
            0..=99 => 3,
            100..=199 => 2,
            200..=254 => 1,
            255 => 0,
        };
        learn_moves_at_level(context, level + 1)?;
    }
    return Ok(());
}

fn learn_moves_at_level(context: &mut MonContext, level: u8) -> Result<(), Error> {
    let mut learnable_moves_at_level = context
        .battle()
        .dex
        .species
        .get(&context.mon().species)
        .into_result()?
        .data
        .learnset
        .iter()
        .filter_map(|(id, methods)| {
            methods
                .contains(&MoveSource::Level(level))
                .then_some(Id::from(id.as_str()))
        })
        .collect::<Vec<_>>();
    // Sort for consistency.
    learnable_moves_at_level.sort();

    let max_move_count = context.battle().format.rules.numeric_rules.max_move_count as usize;
    let current_move_count = context.mon().base_move_slots.len();
    let instant_learn_count = (current_move_count < max_move_count)
        .then(|| (max_move_count - current_move_count).min(learnable_moves_at_level.len()))
        .unwrap_or(0);
    let (instant_learn, learn_request) = learnable_moves_at_level.split_at(instant_learn_count);
    for (move_slot_index, move_id) in instant_learn.iter().enumerate() {
        Mon::learn_move(
            context,
            move_id,
            context.mon().base_move_slots.len() + move_slot_index,
        )?;
    }

    context
        .mon_mut()
        .learnable_moves
        .extend(learn_request.iter().cloned());

    Ok(())
}

/// Gives experience to a single Mon.
///
/// Experience is calculated by [`give_out_experience`].
pub fn gain_experience(context: &mut MonContext, exp: u32) -> Result<(), Error> {
    if context.mon().level == 100 {
        return Ok(());
    }

    core_battle_logs::experience(context, exp)?;
    context.mon_mut().experience += exp;

    let leveling_rate = context
        .battle()
        .dex
        .species
        .get(&context.mon().species)
        .into_result()?
        .data
        .leveling_rate;
    let new_level = leveling_rate.level_from_exp(context.mon().experience);

    if new_level > context.mon().level {
        let mon_handle = context.mon_handle();
        // If Mon is not active in battle, level up directly to the target level.
        if !context.mon().active {
            BattleQueue::insert_action_into_sorted_position(
                context.as_battle_context_mut(),
                Action::LevelUp(LevelUpAction {
                    mon: mon_handle,
                    level: Some(new_level),
                }),
            )?;
        } else {
            for _ in context.mon().level..new_level {
                BattleQueue::insert_action_into_sorted_position(
                    context.as_battle_context_mut(),
                    Action::LevelUp(LevelUpAction {
                        mon: mon_handle,
                        level: None,
                    }),
                )?;
            }
        }
    }

    Ok(())
}

/// Schedules actions for giving out experience to all foe Mons after a Mon faints.
pub fn give_out_experience(
    context: &mut Context,
    fainted_mon_handle: MonHandle,
) -> Result<(), Error> {
    let foe_side = context.mon_context(fainted_mon_handle)?.foe_side().index;
    for foe_handle in context
        .battle()
        .all_mon_handles_on_side(foe_side)
        .collect::<Vec<_>>()
    {
        let mut context = context.mon_context(foe_handle)?;
        if !context.player().player_type.gains_experience() || context.mon().fainted {
            continue;
        }

        let participated = context
            .as_battle_context()
            .mon(fainted_mon_handle)?
            .foes_fought_while_active
            .contains(&foe_handle);

        // TODO: If Exp Share is activated, all Mons get experience.
        if !participated {
            return Ok(());
        }

        // Update EVs now and recalculate stats.
        let species = context
            .as_battle_context()
            .mon(fainted_mon_handle)?
            .base_species
            .clone();
        let ev_yield = context
            .battle()
            .dex
            .species
            .get(&species)?
            .data
            .ev_yield
            .clone();
        for (stat, value) in ev_yield.entries() {
            let current_ev_sum = context.mon().evs.sum();
            let ev_limit = context.battle().format.rules.numeric_rules.ev_limit;
            let new_ev_value = context.mon().evs.get(stat)
                + (current_ev_sum < ev_limit)
                    .then(|| value.min((ev_limit - current_ev_sum) as u16))
                    .unwrap_or(0);
            context.mon_mut().evs.set(stat, new_ev_value);
        }
        Mon::recalculate_stats(&mut context)?;
        Mon::recalculate_base_stats(&mut context)?;

        let exp = calculate_exp_gain(&mut context, fainted_mon_handle)?;
        let mon_handle = context.mon_handle();
        match context
            .battle_mut()
            .queue
            .find_action_mut(|action| match action {
                Action::Experience(action) => action.mon == mon_handle,
                _ => false,
            }) {
            Some(Action::Experience(action)) => {
                action.exp += exp;
            }
            _ => {
                let player_index = context.player().index;
                let mon_index = context.mon().team_position;
                let active = context.mon().active;
                BattleQueue::insert_action_into_sorted_position(
                    context.as_battle_context_mut(),
                    Action::Experience(ExperienceAction {
                        mon: foe_handle,
                        player_index,
                        mon_index,
                        active,
                        exp,
                    }),
                )?;
            }
        }
    }

    Ok(())
}

/// Attempts to escape the battle, using the speed of the given Mon.
pub fn try_escape(context: &mut MonContext, force: bool) -> Result<bool, Error> {
    if context.player().escaped {
        return Ok(true);
    }

    context.player_mut().escape_attempts += 1;

    let escaped = Player::can_escape(context.as_player_context());
    let mut escaped = escaped && force || Mon::can_escape(context)?;
    if escaped && !context.player().player_type.wild() && !force {
        let speed = context.mon().speed;

        // Take the average of the speed of all foes.
        let mut foe_speed = 0;
        let mut foe_count = 0;
        for foe in context
            .battle()
            .active_mon_handles_on_side(context.foe_side().index)
        {
            foe_speed += context.as_battle_context().mon(foe)?.speed;
            foe_count += 1;
        }
        let foe_speed = foe_speed / foe_count;

        let odds = Fraction::from(speed * 32);
        let odds = odds / Fraction::new(foe_speed, 4);
        let odds = odds.floor() + 30 * context.player().escape_attempts;
        escaped = rand_util::chance(context.battle_mut().prng.as_mut(), odds as u64, 256);
    }

    if !escaped {
        core_battle_logs::cannot_escape(context.as_player_context_mut())?;
        return Ok(false);
    }

    context.player_mut().escaped = true;
    for mon in context
        .player()
        .active_mon_handles()
        .cloned()
        .collect::<Vec<_>>()
    {
        switch_out(
            &mut context.as_battle_context_mut().mon_context(mon)?,
            false,
        )?;
    }
    context.player_mut().mons_left = 0;

    core_battle_logs::escaped(context.as_player_context_mut())?;

    Ok(true)
}

/// Sets the weather on the field.
pub fn set_weather(context: &mut FieldEffectContext, weather: &Id) -> Result<bool, Error> {
    let weather_handle = context
        .battle_mut()
        .get_effect_handle_by_id(weather)?
        .clone();
    let weather = weather_handle
        .try_id()
        .wrap_error_with_message("weather must have an id")?
        .clone();

    if context
        .battle()
        .field
        .weather
        .as_ref()
        .is_some_and(|existing| existing == &weather)
    {
        return Ok(false);
    }

    if !core_battle_effects::run_event_for_field_effect(
        context,
        fxlang::BattleEvent::SetWeather,
        fxlang::VariableInput::from_iter([(fxlang::Value::Effect(weather_handle.clone()))]),
    ) {
        return Ok(false);
    }

    let previous_weather = context.battle().field.weather.clone();
    let previous_weather_state = context.battle().field.weather_state.clone();

    context.battle_mut().field.weather = Some(weather.clone());
    let source_handle = context.source_handle();
    context.battle_mut().field.weather_state =
        initial_effect_state(context.as_effect_context_mut(), None, source_handle)?;

    if let Some(weather_condition) =
        CoreBattle::get_effect_by_handle(context.as_battle_context_mut(), &weather_handle)?
            .fxlang_condition()
    {
        if let Some(duration) = weather_condition.duration {
            context
                .battle_mut()
                .field
                .weather_state
                .set_duration(duration);
        }

        if let Some(duration) = core_battle_effects::run_weather_event_expecting_u8(
            context,
            fxlang::BattleEvent::Duration,
        ) {
            context
                .battle_mut()
                .field
                .weather_state
                .set_duration(duration);
        }
    }

    if core_battle_effects::run_weather_event_expecting_bool(
        context,
        fxlang::BattleEvent::FieldStart,
    )
    .is_some_and(|result| !result)
    {
        context.battle_mut().field.weather = previous_weather;
        context.battle_mut().field.weather_state = previous_weather_state;
        return Ok(false);
    }

    // TODO: WeatherChange event.

    Ok(true)
}

/// Clears the weather on the field.
pub fn clear_weather(context: &mut FieldEffectContext) -> Result<bool, Error> {
    if !core_battle_effects::run_event_for_field_effect(
        context,
        fxlang::BattleEvent::ClearWeather,
        fxlang::VariableInput::default(),
    ) {
        return Ok(false);
    }
    core_battle_effects::run_weather_event(context, fxlang::BattleEvent::FieldEnd);
    context.battle_mut().field.weather = None;
    context.battle_mut().field.weather_state = fxlang::EffectState::new();
    // TODO: WeatherChange event.
    Ok(true)
}

/// Sets the target Mon's ability.
pub fn set_ability(
    context: &mut ApplyingEffectContext,
    ability: &Id,
    from_forme_change: bool,
) -> Result<bool, Error> {
    if context.target().hp == 0 {
        return Ok(false);
    }

    // TODO: SetAbility event, which can cancel this completely.

    core_battle_effects::run_mon_ability_event(context, fxlang::BattleEvent::End);
    if !from_forme_change {
        core_battle_logs::end_ability(context)?;
    }

    let ability_priority = context.battle_mut().next_ability_priority();
    let ability = context.battle().dex.abilities.get_by_id(ability)?;
    context.target_mut().ability = AbilitySlot {
        id: ability.id().clone(),
        name: ability.data.name.clone(),
        priority: ability_priority,
        effect_state: fxlang::EffectState::new(),
    };

    core_battle_effects::run_mon_ability_event(context, fxlang::BattleEvent::Start);

    Ok(true)
}

/// Transforms the Mon into the target Mon.
///
/// Used to implement the move "Transform."
pub fn transform_into(
    context: &mut ApplyingEffectContext,
    target: MonHandle,
    log_effect: bool,
) -> Result<bool, Error> {
    if context.target().transformed {
        return Ok(false);
    }

    let target_context = context.as_battle_context_mut().mon_context(target)?;
    if target_context.mon().fainted || target_context.mon().transformed {
        return Ok(false);
    }

    // Collect all data specific to the target Mon that should be set after changing the
    // species.
    let weight = target_context.mon().weight;
    let types = target_context.mon().types.clone();
    let stats = target_context.mon().stats.clone();
    let boosts = target_context.mon().boosts.clone();
    let ability_id = target_context.mon().ability.id.clone();
    let mut move_slots = target_context.mon().move_slots.clone();
    for move_slot in &mut move_slots {
        move_slot.pp = move_slot.max_pp.min(5);
        move_slot.max_pp = move_slot.max_pp.min(5);
        move_slot.disabled = false;
        move_slot.used = false;
        move_slot.simulated = true;
    }

    // Set the species first, for the baseline transformation.
    let species = target_context.mon().species.clone();
    context.target_mut().transformed = true;
    Mon::set_species(&mut context.target_context()?, species)?;

    // Then, manually set everything else.
    context.target_mut().weight = weight;
    context.target_mut().types = types;
    context.target_mut().stats = stats;
    context.target_mut().boosts = boosts;
    set_ability(context, &ability_id, true)?;
    context.target_mut().move_slots = move_slots;

    core_battle_logs::transform(context, target, log_effect)?;

    Ok(true)
}

/// Sets the target Mon's item.
pub fn set_item(context: &mut ApplyingEffectContext, item: &Id) -> Result<bool, Error> {
    if context.target().hp == 0 || !context.target().active {
        return Ok(false);
    }

    core_battle_effects::run_mon_item_event(context, fxlang::BattleEvent::End);

    let item = context.battle().dex.items.get_by_id(item)?;
    context.target_mut().item = Some(ItemSlot {
        id: item.id().clone(),
        name: item.data.name.clone(),
        effect_state: fxlang::EffectState::new(),
    });

    core_battle_effects::run_mon_item_event(context, fxlang::BattleEvent::Start);

    Ok(true)
}

/// Adds a condition to the slot on the side.
pub fn add_slot_condition(
    context: &mut SideEffectContext,
    slot: usize,
    condition: &Id,
) -> Result<bool, Error> {
    let slot_condition_handle = context
        .battle_mut()
        .get_effect_handle_by_id(condition)?
        .clone();
    let condition = slot_condition_handle
        .try_id()
        .wrap_error_with_message("slot condition must have an id")?
        .clone();

    if context
        .side()
        .slot_conditions
        .get(&slot)
        .is_some_and(|conditions| conditions.contains_key(&condition))
    {
        return Ok(
            core_battle_effects::run_slot_condition_event_expecting_bool(
                context,
                fxlang::BattleEvent::SlotRestart,
                slot,
                &condition,
            )
            .unwrap_or(false),
        );
    }

    let source_handle = context.source_handle();
    let effect_state = initial_effect_state(context.as_effect_context_mut(), None, source_handle)?;
    context
        .side_mut()
        .slot_conditions
        .entry(slot)
        .or_default()
        .insert(condition.clone(), effect_state);

    if let Some(slot_condition) =
        CoreBattle::get_effect_by_handle(context.as_battle_context_mut(), &slot_condition_handle)?
            .fxlang_condition()
    {
        if let Some(duration) = slot_condition.duration {
            context
                .side_mut()
                .slot_conditions
                .entry(slot)
                .or_default()
                .get_mut(&condition)
                .wrap_error_with_message("expected slot condition state to exist")?
                .set_duration(duration);
        }

        if let Some(duration) = core_battle_effects::run_slot_condition_event_expecting_u8(
            context,
            fxlang::BattleEvent::Duration,
            slot,
            &condition,
        ) {
            context
                .side_mut()
                .slot_conditions
                .entry(slot)
                .or_default()
                .get_mut(&condition)
                .wrap_error_with_message("expected side condition state to exist")?
                .set_duration(duration);
        }
    }

    if core_battle_effects::run_slot_condition_event_expecting_bool(
        context,
        fxlang::BattleEvent::SlotStart,
        slot,
        &condition,
    )
    .is_some_and(|result| !result)
    {
        context
            .side_mut()
            .slot_conditions
            .entry(slot)
            .or_default()
            .remove(&condition);
        return Ok(false);
    }

    let slot_condition_name =
        CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &condition)?
            .name()
            .to_owned();
    core_battle_logs::add_slot_condition(context, slot, &slot_condition_name)?;

    Ok(true)
}

/// Removes a condition from the slot on the side.
pub fn remove_slot_condition(
    context: &mut SideEffectContext,
    slot: usize,
    condition: &Id,
) -> Result<bool, Error> {
    let slot_condition_handle = context
        .battle_mut()
        .get_effect_handle_by_id(condition)?
        .clone();
    let condition = slot_condition_handle
        .try_id()
        .wrap_error_with_message("slot condition must have an id")?
        .clone();

    if !context
        .side()
        .slot_conditions
        .get(&slot)
        .is_some_and(|conditions| conditions.contains_key(&condition))
    {
        return Ok(false);
    }

    core_battle_effects::run_slot_condition_event(
        context,
        fxlang::BattleEvent::SlotEnd,
        slot,
        &condition,
    );
    context
        .side_mut()
        .slot_conditions
        .entry(slot)
        .or_default()
        .remove(&condition);

    let condition_name = CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &condition)?
        .name()
        .to_owned();
    core_battle_logs::remove_slot_condition(context, slot, &condition_name)?;

    Ok(true)
}
