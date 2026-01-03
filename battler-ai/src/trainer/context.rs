use std::collections::BTreeSet;

use ahash::{
    HashMap,
    HashSet,
};
use anyhow::{
    Error,
    Result,
};
use battler::{
    DataStoreByName,
    Fraction,
    MoveTarget,
    PlayerBattleData,
    Stat,
};
use battler_calc::{
    common::Range,
    simulate::{
        MoveSimulatorInputFlags,
        MultiHit,
        attacker_type_effectiveness,
        calculate_single_stat,
        simulate_move,
    },
    state::Move,
};
use battler_calc_client_util::{
    Mon,
    MonReference,
    move_simulator_input_from_battle_state,
};
use battler_prng::PseudoRandomNumberGenerator;
use battler_state::{
    BattleState,
    MonBattleAppearanceReference,
    side_or_else,
};
use futures_util::lock::Mutex;

use crate::trainer::common::{
    assumptions,
    move_simulator_input_for_non_moves,
    move_simulator_input_for_non_moves_no_defender,
};

#[derive(Clone)]
pub struct Target<'a> {
    pub direct: bool,
    pub mon: Mon<'a, 'a>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum MonCacheKey {
    Battle {
        side: usize,
        player: String,
        mon: usize,
    },
    State(MonBattleAppearanceReference),
}

impl From<&MonReference<'_>> for MonCacheKey {
    fn from(value: &MonReference) -> Self {
        match value {
            MonReference::Battle {
                side,
                player,
                battle_data,
            } => MonCacheKey::Battle {
                side: *side,
                player: player.clone(),
                mon: battle_data.player_team_position,
            },
            MonReference::State(reference) => MonCacheKey::State((*reference).clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MoveSimulatorCacheKey {
    attacker: MonCacheKey,
    defender: MonCacheKey,
}

pub struct TrainerMonContext<'a> {
    pub data: &'a dyn DataStoreByName,
    pub state: &'a BattleState,
    #[allow(unused)]
    pub prng: &'a Mutex<Box<dyn PseudoRandomNumberGenerator>>,
    pub player_data: &'a PlayerBattleData,
    pub allies: &'a [PlayerBattleData],
    pub mon: Mon<'a, 'a>,

    simulated_moves: Mutex<HashMap<MoveSimulatorCacheKey, HashMap<String, MultiHit>>>,
    simulated_type_effectiveness: Mutex<HashMap<MoveSimulatorCacheKey, Fraction<u64>>>,
    simulated_speed: Mutex<HashMap<MonCacheKey, Range<u64>>>,
}

impl<'a> TrainerMonContext<'a> {
    pub fn new(
        data: &'a dyn DataStoreByName,
        state: &'a BattleState,
        prng: &'a Mutex<Box<dyn PseudoRandomNumberGenerator>>,
        player_data: &'a PlayerBattleData,
        allies: &'a [PlayerBattleData],
        mon: Mon<'a, 'a>,
    ) -> Self {
        Self {
            data,
            state,
            prng,
            player_data,
            allies,
            mon,
            simulated_moves: Mutex::new(HashMap::default()),
            simulated_type_effectiveness: Mutex::new(HashMap::default()),
            simulated_speed: Mutex::new(HashMap::default()),
        }
    }

    #[allow(unused)]
    pub fn side(&self) -> usize {
        self.player_data.side
    }

    #[allow(unused)]
    pub fn ally_sides(&self) -> impl Iterator<Item = usize> {
        let sides = std::iter::once(self.player_data.side)
            .chain(self.allies.iter().map(|ally| ally.side))
            .collect::<BTreeSet<_>>();
        sides.into_iter()
    }

    #[allow(unused)]
    pub fn foe_sides(&self) -> impl Iterator<Item = usize> {
        let ally_sides = self.ally_sides().collect::<HashSet<_>>();
        self.state
            .field
            .sides
            .iter()
            .enumerate()
            .map(|(i, _)| i)
            .filter(move |i| !ally_sides.contains(&i))
    }

    fn mon_from_player_data(
        &self,
        player_data: &'a PlayerBattleData,
        position: usize,
    ) -> Result<Mon<'a, 'a>> {
        let side = player_data.side;
        let battle_data = player_data
            .mons
            .iter()
            .find(|battle_data| {
                battle_data
                    .side_position
                    .is_some_and(|battle_data_position| battle_data_position == position)
            })
            .ok_or_else(|| {
                Error::msg("active mon in state is not in the same position in battle data")
            })?;
        Ok(Mon::new(
            MonReference::Battle {
                side,
                player: player_data.id.clone(),
                battle_data,
            },
            self.state,
            self.data,
        ))
    }

    fn create_mon_for_reference(
        &self,
        reference: &'a MonBattleAppearanceReference,
    ) -> Result<Mon<'a, 'a>> {
        let mon = Mon::new(MonReference::State(reference), self.state, self.data);

        if let Some(position) = mon.active_position()? {
            if self.player_data.id == reference.player {
                return self.mon_from_player_data(self.player_data, position);
            }

            if let Some(ally) = self
                .allies
                .iter()
                .find(|player_data| player_data.id == reference.player)
            {
                return self.mon_from_player_data(ally, position);
            }
        }

        Ok(mon)
    }

    pub fn all_foes(&self) -> Result<impl Iterator<Item = Mon<'a, 'a>> + Clone> {
        // Should not depend on targeting, since targeting only works for active Mons.
        Ok(self
            .foe_sides()
            .map(|side| side_or_else(self.state, side))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flat_map(|side| {
                side.active.iter().map(|active| {
                    active
                        .as_ref()
                        .map(|reference| self.create_mon_for_reference(reference))
                        .transpose()
                })
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter_map(|mon| mon))
    }

    fn valid_target(&self, move_target: MoveTarget, target: &Mon) -> Result<bool> {
        let relative_position = match self.mon.relative_position(target)? {
            Some(position) => position,
            None => return Ok(false),
        };

        Ok(move_target.is_affected(relative_position, 2))
    }

    fn all_active_mons<'s>(&'s self) -> Result<impl Iterator<Item = Mon<'a, 'a>> + 's> {
        Ok((0..self.state.field.sides.len())
            .map(|side| Ok(side_or_else(self.state, side)?.active.iter()))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flat_map(|i| i)
            .filter_map(|mon| mon.as_ref().map(|mon| self.create_mon_for_reference(mon)))
            .collect::<Result<Vec<_>>>()?
            .into_iter())
    }

    pub fn possible_targets(
        &self,
        move_target: MoveTarget,
    ) -> Result<impl Iterator<Item = Target<'a>> + Clone> {
        Ok(self
            .all_active_mons()?
            .map(|mon| {
                let valid = self.valid_target(move_target, &mon)?;
                Ok((mon, valid))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter_map(move |(mon, valid)| {
                valid.then_some(Target {
                    direct: move_target.affects_mons_directly(),
                    mon,
                })
            }))
    }

    pub fn target_choice(&self, target: &Mon<'_, '_>) -> Result<isize> {
        let position = target
            .active_position()?
            .ok_or_else(|| Error::msg("cannot target an inactive mon"))?;
        let position = TryInto::<isize>::try_into(position)?;
        if target.side()? == self.mon.side()? {
            Ok(-position)
        } else {
            Ok(position)
        }
    }

    pub async fn move_result(&self, move_name: &str, target: &Target<'_>) -> Result<MultiHit> {
        let key = MoveSimulatorCacheKey {
            attacker: self.mon.reference().into(),
            defender: target.mon.reference().into(),
        };
        if let Some(cache) = self.simulated_moves.lock().await.get(&key)
            && let Some(result) = cache.get(move_name).cloned()
        {
            return Ok(result);
        }

        let result = simulate_move(move_simulator_input_from_battle_state(
            self.data,
            self.state,
            self.mon.reference().clone(),
            target.mon.reference().clone(),
            Move {
                name: move_name.to_owned(),
                ..Default::default()
            },
            &assumptions(),
            MoveSimulatorInputFlags {
                indirect: !target.direct,
                ..Default::default()
            },
        )?)?;

        self.simulated_moves
            .lock()
            .await
            .entry(key)
            .or_default()
            .insert(move_name.to_owned(), result.clone());
        Ok(result)
    }

    pub async fn type_effectiveness(
        &self,
        attacker: &Mon<'_, '_>,
        defender: &Mon<'_, '_>,
    ) -> Result<Fraction<u64>> {
        let key = MoveSimulatorCacheKey {
            attacker: attacker.reference().into(),
            defender: defender.reference().into(),
        };
        if let Some(result) = self
            .simulated_type_effectiveness
            .lock()
            .await
            .get(&key)
            .cloned()
        {
            return Ok(result);
        }

        let result =
            attacker_type_effectiveness(move_simulator_input_for_non_moves(&attacker, &defender)?)
                .map(|val| *val.value())?;

        self.simulated_type_effectiveness
            .lock()
            .await
            .insert(key, result);
        Ok(result)
    }

    pub async fn speed(&self, mon: &Mon<'_, '_>) -> Result<Range<u64>> {
        // Speed does not take any defenders into account.
        let key = mon.reference().into();
        if let Some(result) = self.simulated_speed.lock().await.get(&key).cloned() {
            return Ok(result);
        }

        let result = calculate_single_stat(
            move_simulator_input_for_non_moves_no_defender(&mon)?,
            Stat::Spe,
        )
        .map(|val| *val.value())?;

        self.simulated_speed.lock().await.insert(key, result);
        Ok(result)
    }

    pub async fn match_up_score(&self, defender: &Mon<'_, '_>) -> Result<Fraction<u64>> {
        let attacker_speed = self.speed(&self.mon).await?.avg();
        let defender_speed = self.speed(defender).await?.avg();
        let outspeed = if attacker_speed >= defender_speed {
            Fraction::new(5, 4)
        } else {
            Fraction::from(1u64)
        };

        let attacker_type_effectiveness = self.type_effectiveness(&self.mon, defender).await?;
        let defender_type_effectiveness = self.type_effectiveness(defender, &self.mon).await?;

        let attacker_health = self.mon.health_fraction()?.unwrap_or(Fraction::from(1u64));
        let defender_health = defender.health_fraction()?.unwrap_or(Fraction::from(1u64));
        let health_diff = attacker_health + (Fraction::from(1u64) - defender_health);

        let attacker_score = attacker_type_effectiveness * outspeed;
        let defender_score = defender_type_effectiveness.inverse();

        let score = (attacker_score + defender_score) * health_diff;

        Ok(score)
    }
}
