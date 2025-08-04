use std::sync::LazyLock;

use anyhow::Result;
use battler::Fraction;
use battler_calc::simulate::StatusEffect;
use futures_util::{
    FutureExt,
    future::BoxFuture,
};

use crate::trainer::context::{
    Target,
    TrainerMonContext,
};

pub(crate) type ModifyMoveScore = for<'a> fn(
    &'a TrainerMonContext<'a>,
    &'a str,
    &'a Target<'a>,
    &'a mut i64,
) -> BoxFuture<'a, Result<()>>;
pub(crate) type ModifyMatchUpScore =
    for<'a> fn(&'a TrainerMonContext<'a>, &'a mut i64) -> BoxFuture<'a, Result<()>>;

// Creating an async closure with all of our references is a bit of a headache.
//
// This macro makes it pretty easy.
macro_rules! modify_move_score {
    ( | $context:ident, $name:ident, $mon:ident, $score:ident | $fn:block ) => {
        (for<'a> |#[allow(unused)] $context: &'a TrainerMonContext<'a>,
                  #[allow(unused)] $name: &'a str,
                  #[allow(unused)] $mon: &'a Target<'a>,
                  #[allow(unused)] $score: &'a mut i64|
                 -> BoxFuture<'a, Result<()>> {
            async {
                $fn;
                Ok(())
            }
            .boxed()
        }) as _
    };
}

macro_rules! modify_match_up_score {
    ( | $context:ident, $score:ident | $fn:block ) => {
        (for<'a> |#[allow(unused)] $context: &'a TrainerMonContext<'a>,
                  #[allow(unused)] $score: &'a mut i64|
                 -> BoxFuture<'a, Result<()>> {
            async {
                $fn;
                Ok(())
            }
            .boxed()
        }) as _
    };
}

// TODO: Need to better represent allies vs. targets, beneficial vs. harmful effects.
pub(crate) static BASIC_MODIFY_MOVE_SCORE_HOOKS: LazyLock<Vec<ModifyMoveScore>> =
    LazyLock::new(|| {
        Vec::from_iter([
            modify_move_score!(|context, name, target, score| {
                // Do not hit allies.
                if target.mon.is_ally(&context.mon)? {
                    *score -= 30;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move fails.
                let result = context.move_result(name, target).await?;
                if let Some(hit) = result.first_hit()
                    && hit.failed
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move seemingly does nothing beneficial.
                let result = context.move_result(name, target).await?;
                if result.damage_on_target().b() == 0
                    && result.combined_status_effect_on_target() == StatusEffect::default()
                    && result.combined_status_effect_on_user() == StatusEffect::default()
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move applies a volatile that is already applied.
                let result = context.move_result(name, target).await?;
                if result.damage_on_target().b() == 0
                    && let effect = &result.combined_status_effect_on_target()
                    && let Some(volatile) = &effect.volatile
                    && effect
                        == &(StatusEffect {
                            volatile: Some(volatile.clone()),
                            ..Default::default()
                        })
                    && let max_count = match volatile.as_str() {
                        "Bide" => u64::MAX,
                        "Charge" => u64::MAX,
                        "Stockpile" => 3,
                        _ => 1,
                    }
                    && let Some(condition) = target.mon.condition_data(volatile.as_str())?
                    && match max_count {
                        1 => true,
                        _ => condition.data.get("count").is_some_and(|count| {
                            count.parse::<u64>().is_ok_and(|count| count >= max_count)
                        }),
                    }
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move applies a side condition that is already applied.
                let result = context.move_result(name, target).await?;
                if result.damage_on_target().b() == 0
                    && let effect = &result.combined_status_effect_on_target()
                    && let Some(side_condition) = &effect.side_condition
                    && effect
                        == &(StatusEffect {
                            side_condition: Some(side_condition.clone()),
                            ..Default::default()
                        })
                    && let max_count = match side_condition.as_str() {
                        "Spikes" => 3,
                        "Toxic Spikes" => 2,
                        _ => 1,
                    }
                    && let Some(condition) =
                        target.mon.side_condition_data(side_condition.as_str())?
                    && match max_count {
                        1 => true,
                        _ => condition.data.get("count").is_some_and(|count| {
                            count.parse::<u64>().is_ok_and(|count| count >= max_count)
                        }),
                    }
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move applies weather that is already applied.
                let result = context.move_result(name, target).await?;
                if result.damage_on_target().b() == 0
                    && let effect = &result.combined_status_effect_on_target()
                    && let Some(weather) = &effect.weather
                    && effect
                        == &(StatusEffect {
                            weather: Some(weather.clone()),
                            ..Default::default()
                        })
                    && context
                        .state
                        .field
                        .weather
                        .as_ref()
                        .is_some_and(|current_weather| current_weather == weather)
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move applies a terrain that is already applied.
                let result = context.move_result(name, target).await?;
                if result.damage_on_target().b() == 0
                    && let effect = &result.combined_status_effect_on_target()
                    && let Some(terrain) = &effect.terrain
                    && effect
                        == &(StatusEffect {
                            terrain: Some(terrain.clone()),
                            ..Default::default()
                        })
                    && context.state.field.conditions.contains_key(terrain)
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move applies a field condition that is already applied.
                let result = context.move_result(name, target).await?;
                if result.damage_on_target().b() == 0
                    && let effect = &result.combined_status_effect_on_target()
                    && let Some(pseudo_weather) = &effect.pseudo_weather
                    && effect
                        == &(StatusEffect {
                            pseudo_weather: Some(pseudo_weather.clone()),
                            ..Default::default()
                        })
                    && context.state.field.conditions.contains_key(pseudo_weather)
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move heals the target at full health.
                let result = context.move_result(name, target).await?;
                if result.damage_on_target().b() == 0
                    && result.total_heal().b() > 0
                    && let Some(health) = target.mon.health_fraction()?
                    && health == 1
                {
                    *score -= 8;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move targets the user and likely kills it.
                let result = context.move_result(name, target).await?;
                if target.mon.is_same(&context.mon)?
                    && let Some(health) = context.mon.health_fraction()?
                    && Fraction::new(result.total_damage().b(), result.target_hp.b())
                        + Fraction::new(1, 100)
                        >= health
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move boosts stats that are already maxed out.
                let result = context.move_result(name, target).await?;
                if result.damage_on_target().b() == 0
                    && let Some(boosts) = &result.combined_status_effect_on_target().boosts
                    && let target_boosts = target.mon.boosts()?
                    && boosts
                        .non_zero_iter()
                        .all(|(boost, val)| val > 0 && target_boosts.get(boost) >= 6)
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move reduces stats that are already minimized.
                let result = context.move_result(name, target).await?;
                if result.damage_on_target().b() == 0
                    && let Some(boosts) = &result.combined_status_effect_on_target().boosts
                    && let target_boosts = target.mon.boosts()?
                    && boosts
                        .non_zero_iter()
                        .all(|(boost, val)| val < 0 && target_boosts.get(boost) <= -6)
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move boosts speed and Trick Room is active.
                let result = context.move_result(name, target).await?;
                if context.state.field.conditions.contains_key("Trick Room")
                    && target.mon.is_ally(&context.mon)?
                    && result.damage_on_target().b() == 0
                    && let Some(boosts) = &result.combined_status_effect_on_target().boosts
                    && boosts.spe > 0
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move reduces speed and Trick Room is active.
                let result = context.move_result(name, target).await?;
                if context.state.field.conditions.contains_key("Trick Room")
                    && target.mon.is_foe(&context.mon)?
                    && result.damage_on_target().b() == 0
                    && let Some(boosts) = &result.combined_status_effect_on_target().boosts
                    && boosts.spe < 0
                {
                    *score -= 10;
                }
            }),
            modify_move_score!(|context, name, target, score| {
                // Move forces a switch but target cannot switch.
                let result = context.move_result(name, target).await?;
                if result.damage_on_target().b() == 0
                    && let effect = &result.combined_status_effect_on_target()
                    && effect.switch
                    && effect
                        == &(StatusEffect {
                            switch: true,
                            ..Default::default()
                        })
                    && !target.mon.player_can_switch()?
                {
                    *score -= 10;
                }
            }),
        ])
    });

pub(crate) static MODIFY_MATCH_UP_SCORE_HOOKS: LazyLock<Vec<ModifyMatchUpScore>> =
    LazyLock::new(|| {
        Vec::from_iter([modify_match_up_score!(|context, score| {
            if let Some(mon) = context.mon.active_mon_state()?
                && let Some(perish_song) = mon.volatile_data.conditions.get("Perish Song")
                && let Some(count) = perish_song.data.get("perish")
                && count == "1"
            {
                *score = i64::MIN.into();
            }
        })])
    });
