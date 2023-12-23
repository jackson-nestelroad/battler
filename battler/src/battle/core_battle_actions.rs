use std::ops::Deref;

use lazy_static::lazy_static;

use crate::{
    battle::{
        core_battle_logs,
        ActiveMoveContext,
        Battle,
        Mon,
        MonContext,
        MonHandle,
        MoveHandle,
        MoveOutcome,
    },
    battle_event,
    battler_error,
    common::{
        Error,
        Id,
        Identifiable,
        UnsafelyDetachBorrow,
        WrapResultError,
    },
    moves::{
        MoveTarget,
        SelfDestructType,
    },
};

/// Switches a Mon into the given position.
pub fn switch_in(context: &mut MonContext, position: usize) -> Result<(), Error> {
    if context.mon_mut().active {
        context
            .battle_mut()
            .hint("A switch failed because the Mon trying to switch in is already in.");
        return Ok(());
    }

    let active_len = context.player().active.len();
    if position >= active_len {
        return Err(battler_error!(
            "Invalid switch position {position} / {active_len}"
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
        let mut mon = context.battle().mon_mut(mon)?;
        mon.switch_out();
    }
    Mon::switch_in(context, position);
    context.player_mut().active[position] = Some(context.mon_handle());

    let event = battle_event!("switch", Mon::active_details(context)?);
    context.battle_mut().log(event);

    Ok(())
}

fn register_active_move(
    context: &mut MonContext,
    move_id: &Id,
    target: Option<MonHandle>,
) -> Result<MoveHandle, Error> {
    let active_move = context
        .battle_mut()
        .dex
        .moves
        .get_by_id(move_id)?
        .deref()
        .clone();
    let active_move_handle = context.battle().registry.register_move(active_move);
    let mon_handle = context.mon_handle();
    context
        .battle_mut()
        .set_active_move(active_move_handle, mon_handle, target)?;
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
    let target =
        context
            .battle_mut()
            .get_target(mon_handle, move_id, target_location, original_target)?;

    // Make a copy of the move so we can work with it and modify it for the turn.
    let active_mon_handle = register_active_move(context, move_id, target)?;
    context.active_move_mut()?.external = external;

    // TODO: Run BeforeMove checks.
    // TODO: Abort move if requested.

    context.mon_mut().last_damage = 0;
    // External moves do not have PP deducted.
    if !external {
        // TODO: Check for locked move, which will not deduct PP (think Uproar).
        let active_move = context.active_move()?;
        let move_id = active_move.id();
        // SAFETY: move_id is only used for lookup.
        let move_id = unsafe { move_id.unsafely_detach_borrow() };
        if !context.mon_mut().deduct_pp(move_id, 1) && !move_id.eq("struggle") {
            // No PP, so this move action cannot be carried through.
            let event = battle_event!(
                "cant",
                Mon::position_details(context)?,
                "nopp",
                context.active_move()?.data.name,
            );
            context.battle_mut().log(event);
            context.battle_mut().clear_active_move()?;
            return Ok(());
        }

        // At this point, the move will be attempted, so we should remember it.
        context.mon_mut().last_move_selected = Some(active_mon_handle);
        context.mon_mut().last_move_target = target_location;
    }

    // Use the move.
    let move_id = context.active_move()?.id().clone();
    let target = context.mon().active_target;
    use_move(context, &move_id, target)?;

    // TODO: AfterMove event.
    // TODO: Run faint messages.
    // TODO: Check if battle has ended.

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
        target = context.battle_mut().random_target(mon_handle, move_id)?;
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
        target = context.battle_mut().random_target(mon_handle, move_id)?;
    }

    // Mon fainted before this move could be made.
    if context.mon().fainted {
        return Ok(MoveOutcome::Failed);
    }

    // Log that the move is being used.
    let mut event = battle_event!(
        "move",
        Mon::position_details(context.as_mon_context())?,
        context.active_move().data.name,
    );
    if let Some(target) = target {
        event.push(&Mon::position_details(
            &context.target_mon_context(target)?,
        )?);
    }
    context.battle_mut().log(event);

    if context.mon().active_target.is_none() && context.active_move().data.target.requires_target()
    {
        core_battle_logs::fail(&mut context.as_mon_context_mut())?;
        return Ok(MoveOutcome::Failed);
    }

    let targets = get_move_targets(&mut context, target)?;
    // TODO: Targeted event.
    // TODO: TryMove event.
    // TODO: UseMoveMessage event.

    if context.active_move().data.self_destruct == Some(SelfDestructType::Always) {
        // TODO: Faint the user.
    }

    let mut outcome = MoveOutcome::Success;
    if !context.active_move().data.target.affects_mons_directly() {
        todo!("moves that do not affect Mons directly are not implemented")
    } else {
        if targets.is_empty() {
            core_battle_logs::fail(&mut context.as_mon_context_mut())?;
            return Ok(MoveOutcome::Failed);
        }
        outcome = try_direct_move(&mut context, &targets)?;
    }

    // TODO: Move hit on self for boosts?

    // TODO: Faint the user if needed.

    // TODO: MoveFail event.

    todo!("use_move_internal is not implemented");
    Ok(MoveOutcome::Success)
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
                let target_mon = context.battle().mon(possible_target)?;
                if !target_mon.fainted || !target_mon.is_ally(context.mon()) {
                    // A targeted for has fainted, so the move should retarget.
                    let mon = context.mon_handle();
                    let active_move = context.active_move().id().clone();
                    target = context.battle_mut().random_target(mon, &active_move)?;
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

    todo!("try_direct_move is unfinished")
}

mod direct_move_step {
    use std::ops::Mul;

    use crate::{
        battle::{
            core_battle_logs,
            ActiveMoveContext,
            ActiveTargetContext,
            Mon,
            MonContext,
            MonHandle,
            MoveDamage,
        },
        common::{
            Error,
            Fraction,
            MaybeOwnedMut,
            WrapResultError,
        },
        mons::Stat,
        moves::{
            Accuracy,
            DamageType,
            MonOverride,
            MoveCategory,
            MoveTarget,
            MultihitType,
        },
    };

    pub struct MoveStepTarget {
        handle: MonHandle,
        /// Move steps can assume by default that this will be `true`.
        should_continue: bool,
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
        if targets.iter().all(|target| !target.should_continue) {
            core_battle_logs::fail(&mut context.as_mon_context_mut())?;
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
            let immune = immune || Mon::is_immune(&mut target_context, move_type)?;
            if immune {
                core_battle_logs::immune(&mut target_context)?;
            }
            target.should_continue = !immune;
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
                core_battle_logs::miss(context.as_active_move_context_mut(), target.handle)?;
                target.should_continue = false;
                // TODO: AccuracyFailure event.
            }
        }
        Ok(())
    }

    fn accuracy_check(context: &mut ActiveTargetContext) -> Result<bool, Error> {
        let mut accuracy = context.active_move().data.accuracy;
        // OHKO moves bypass accuracy modifiers.
        if !context.active_move().data.ohko_type.is_some() {
            // TODO: ModifyAccuracy event.
            if let Accuracy::Chance(accuracy) = &mut accuracy {
                let mut boost = 0;
                if !context.active_move().data.ignore_accuracy {
                    // TODO: ModifyBoost event.
                    boost = context.mon().boosts.acc.max(6).min(-6);
                }
                if !context.active_move().data.ignore_evasion {
                    // TODO: ModifyBoost event.
                    boost = (boost - context.target_mon_context()?.mon().boosts.eva)
                        .max(6)
                        .min(-6);
                }
                let multiplier = if boost > 0 {
                    Fraction::new((3 + boost) as u8, 3)
                } else {
                    Fraction::new(3, (3 - boost) as u8)
                };
                *accuracy = multiplier.mul(*accuracy).floor();
            }
        }

        // TODO: Accuracy event.
        match accuracy {
            Accuracy::Chance(accuracy) => {
                Ok(context.battle_mut().prng.chance(accuracy as u64, 100))
            }
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
        context.mon_mut().last_damage = 0;

        let hits = match context.active_move().data.multihit {
            None => 1,
            Some(MultihitType::Static(hits)) => hits,
            Some(MultihitType::Range(min, max)) => {
                if min == 2 && max == 5 {
                    // 35-35-15-15 for 2-3-4-5 hits.
                    *context
                        .battle_mut()
                        .prng
                        .sample_slice(&[2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 5, 5, 5])
                        .wrap_error()?
                } else {
                    context.battle_mut().prng.range(min as u64, max as u64) as u8
                }
            }
        };

        // TODO: Consider Loaded Dice item.

        let mut targets = targets
            .iter()
            .map(|target| target.handle)
            .collect::<Vec<_>>();
        let mut total_damage = vec![0; targets.len()];
        for hit in 1..=hits {
            // No more targets.
            if targets.is_empty() {
                break;
            }

            // Record number of hits.
            context.active_move_mut().hit = hit;

            // Of all the eligible targets, determine which ones we will actually hit.
            let mut next_hit_targets = Vec::with_capacity(targets.len());
            for target in targets {
                let mut context = context.target_context(target)?;
                if context.active_move().data.multiaccuracy && hit > 1 {
                    if !accuracy_check(&mut context)? {
                        continue;
                    }
                }

                // If we made it this far, the target is eligible for another hit.
                next_hit_targets.push(target);
            }
            targets = next_hit_targets;

            // TODO: spreadMoveHit (this is where damage is done to all targets!).
        }

        // At this point, everything hits.
        Ok(())
    }

    struct HitTarget {
        handle: MonHandle,
        damage: MoveDamage,
    }

    fn hit_targets(
        context: &mut ActiveMoveContext,
        targets: &mut [HitTarget],
        is_secondary: bool,
        is_self: bool,
    ) -> Result<(), Error> {
        for target in targets {
            target.damage = MoveDamage::Damage(0);
        }

        let move_target = context.active_move().data.target.clone();
        if move_target == MoveTarget::All {
            // TODO: TryHitField event.
        } else if move_target == MoveTarget::FoeSide
            || move_target == MoveTarget::AllySide
            || move_target == MoveTarget::AllyTeam
        {
            // TODO: TryHitSide event.
        } else {
            // TODO: TryHit event for each target.
        }

        // TODO: If any of the above events fail, the move should fail.
        // TODO: If we run multiple TryHit events for multiple targets, the targets hit should be
        // filtered.

        // First, check for substitute.
        if !is_secondary && !is_self && move_target.affects_mons_directly() {
            // TODO: TryPrimaryHit event, which should catch substitutes.
        }

        // TODO: If we hit a substitute, filter those targets out.

        // Calculate damage for each target.
        // TODO: getSpreadDamage (calculate_spread_damage).

        // TODO: spreadDamage (apply the damage).

        // TODO: runMoveEffects.

        // TODO: Self drops.

        // TODO: Secondary effects.

        // TODO: Force switch.

        todo!("hit_targets is unimplemented")
    }

    fn calculate_spread_damage(
        context: &mut ActiveMoveContext,
        targets: &mut [HitTarget],
        is_secondary: bool,
        is_self: bool,
    ) -> Result<(), Error> {
        for target in targets {
            context
                .battle_mut()
                .set_active_target(Some(target.handle))?;
            // TODO: call calculate_damage.
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

        // Static damage.
        match context.active_move().data.damage {
            Some(DamageType::Level) => return Ok(MoveDamage::Damage(context.mon().level as u16)),
            Some(DamageType::Set(damage)) => return Ok(MoveDamage::Damage(damage)),
            _ => (),
        }

        // Critical hit.
        // TODO: ModifyCritRatio event.
        let crit_ratio = context.active_move().data.crit_ratio.unwrap_or(0);
        let crit_ratio = crit_ratio.min(0).max(4);
        let crit_mult = [0, 24, 8, 2, 1];
        context.active_move_mut().hit_data(&target_mon_handle).crit =
            context.active_move().data.will_crit
                || (crit_ratio > 0
                    && context
                        .battle_mut()
                        .prng
                        .chance(1, crit_mult[crit_ratio as usize]));

        if context.active_move_mut().hit_data(&target_mon_handle).crit {
            // TODO: CriticalHit event.
        }

        let base_power = context.active_move().data.base_power;

        // TODO: BasePower event.
        let base_power = base_power.min(1);
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

        // TODO: Mon::calculate_stat.

        todo!()
    }
}
