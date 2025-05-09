use std::{
    collections::{
        BTreeMap,
        VecDeque,
    },
    usize,
};

use ahash::{
    HashMap,
    HashSet,
};
use anyhow::{
    Context,
    Error,
    Result,
};

use crate::{
    discovery::{
        DiscoveryRequired,
        DiscoveryRequiredSet,
    },
    log::{
        EffectName,
        Log,
        LogEntry,
        MonName,
        MonNameList,
    },
    ui,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Ambiguity {
    #[default]
    Precise,
    Ambiguous,
}

/// Data about some condition.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConditionData {
    pub since_turn: usize,
}

/// Data about some condition specifically attached to a Mon.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MonConditionData {
    pub condition_data: ConditionData,
    pub until_target_moves: bool,
}

/// Volatile data for a Mon, which applies only when the Mon is active.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MonVolatileData {
    pub ability: Option<String>,
    pub conditions: BTreeMap<String, MonConditionData>,
    pub types: Vec<String>,
    pub stat_boosts: BTreeMap<String, i64>,
}

impl MonVolatileData {
    fn record_ability(&mut self, name: String) {
        self.ability = Some(name);
    }

    fn record_condition(&mut self, condition: String, condition_data: MonConditionData) {
        self.conditions.insert(condition, condition_data);
    }

    fn remove_condition(&mut self, condition: &str) {
        self.conditions.remove(condition);
    }

    fn record_types<I>(&mut self, types: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.types = types.into_iter().collect();
    }

    fn record_stat_boost(&mut self, stat: String, diff: i64) {
        *self.stat_boosts.entry(stat).or_default() += diff;
    }
}

/// The physical appearance of a Mon, which is expected to *never* change through the course of a
/// battle.
///
/// When a Mon creates an illusion, it is expected to mimic this physical appearance.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct MonPhysicalAppearance {
    pub name: String,
    pub species: String,
    pub gender: String,
    pub shiny: bool,
}

impl MonPhysicalAppearance {
    fn matches(&self, other: &Self) -> bool {
        (self.name.is_empty() || other.name.is_empty() || self.name == other.name)
            && (self.species.is_empty()
                || other.species.is_empty()
                || self.species == other.species)
            && (self.gender.is_empty() || self.gender.is_empty() || self.gender == other.gender)
            && self.shiny == other.shiny
    }
}

#[derive(Debug, Default, Clone)]
struct MonBattleAppearanceFromSwitchIn {
    level: u64,
    health: (u64, u64),
    status: String,
}

/// Data about a Mon that is slowly discovered and may change throughout the course of the battle.
///
/// Some data, like `level` and `health`, are discovered on switch in. Other data, like `moves` and
/// `ability`, are discovered when used or activated.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MonBattleAppearance {
    pub level: DiscoveryRequired<u64>,
    pub health: DiscoveryRequired<(u64, u64)>,
    pub status: DiscoveryRequired<String>,
    pub ability: DiscoveryRequired<String>,
    pub item: DiscoveryRequired<String>,

    pub moves: DiscoveryRequiredSet<String>,
}

impl MonBattleAppearance {
    fn is_empty(&self) -> bool {
        self.level.is_empty()
            && self.health.is_empty()
            && self.status.is_empty()
            && self.ability.is_empty()
            && self.item.is_empty()
            && self.moves.is_empty()
    }

    fn make_ambiguous(mut self) -> Self {
        self.level = self.level.make_ambiguous();
        self.health = self.health.make_ambiguous();
        self.status = self.status.make_ambiguous();
        self.ability = self.ability.make_ambiguous();
        self.item = self.item.make_ambiguous();
        self.moves = self.moves.make_ambiguous();
        self
    }

    fn record_level(&mut self, level: DiscoveryRequired<u64>, ambiguity: Ambiguity) {
        self.level = if ambiguity == Ambiguity::Ambiguous {
            self.level.take().merge(level)
        } else {
            self.level.take().record(level)
        };
    }

    fn record_health(&mut self, health: DiscoveryRequired<(u64, u64)>, ambiguity: Ambiguity) {
        self.health = if ambiguity == Ambiguity::Ambiguous {
            self.health.take().merge(health)
        } else {
            self.health.take().record(health)
        };
    }

    fn record_status(&mut self, status: DiscoveryRequired<String>, ambiguity: Ambiguity) {
        self.status = if ambiguity == Ambiguity::Ambiguous {
            self.status.take().merge(status)
        } else {
            self.status.take().record(status)
        };
    }

    fn record_ability(&mut self, ability: DiscoveryRequired<String>, ambiguity: Ambiguity) {
        self.ability = if ambiguity == Ambiguity::Ambiguous {
            self.ability.take().merge(ability)
        } else {
            self.ability.take().record(ability)
        };
    }

    fn record_item(&mut self, item: DiscoveryRequired<String>, ambiguity: Ambiguity) {
        self.item = if ambiguity == Ambiguity::Ambiguous {
            self.item.take().merge(item)
        } else {
            self.item.take().record(item)
        };
    }

    fn record_move(&mut self, name: String, ambiguity: Ambiguity) {
        if ambiguity == Ambiguity::Ambiguous {
            self.moves.record_possible(name);
        } else {
            self.moves.record_known(name);
        }
    }

    fn record_all(&mut self, other: Self) {
        self.record_level(other.level, Ambiguity::Precise);
        self.record_health(other.health, Ambiguity::Precise);
        self.record_status(other.status, Ambiguity::Precise);
        self.record_ability(other.ability, Ambiguity::Precise);
        self.record_item(other.item, Ambiguity::Precise);

        for mov in other.moves.known().iter().cloned() {
            self.record_move(mov, Ambiguity::Precise);
        }
        for mov in other.moves.possible_values().iter().cloned() {
            self.record_move(mov, Ambiguity::Ambiguous);
        }
    }
}

impl From<&MonBattleAppearanceFromSwitchIn> for MonBattleAppearance {
    fn from(value: &MonBattleAppearanceFromSwitchIn) -> Self {
        let mut data = MonBattleAppearance::default();
        data.record_level(value.level.into(), Ambiguity::Precise);
        data.record_health(value.health.into(), Ambiguity::Precise);
        data.record_status(value.status.clone().into(), Ambiguity::Precise);
        data
    }
}

/// A version of [`MonBattleAppearance`] that supports recovery when a Mon is discovered to be an
/// illusion of another Mon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonBattleAppearanceWithRecovery {
    /// Mon is inactive, so it contains a single battle appearance.
    Inactive(MonBattleAppearance),
    /// Mon is active, so it records the difference between battle data up to the last switch out
    /// and battle data from the last switch in. If the Mon is found to be an illusion, the battle
    /// data from the last switch in is moved to another Mon.
    Active {
        primary_battle_appearance: MonBattleAppearance,
        battle_appearance_up_to_last_switch_out: MonBattleAppearance,
        battle_appearance_from_last_switch_in: MonBattleAppearance,
    },
}

impl MonBattleAppearanceWithRecovery {
    fn is_active(&self) -> bool {
        match self {
            Self::Inactive(_) => false,
            Self::Active { .. } => true,
        }
    }

    fn take_primary(self) -> MonBattleAppearance {
        match self {
            Self::Inactive(appearance) => appearance,
            Self::Active {
                primary_battle_appearance,
                ..
            } => primary_battle_appearance,
        }
    }

    /// The primary battle appearance data.
    pub fn primary(&self) -> &MonBattleAppearance {
        match self {
            Self::Inactive(appearance) => appearance,
            Self::Active {
                primary_battle_appearance,
                ..
            } => primary_battle_appearance,
        }
    }

    fn primary_mut(&mut self) -> &mut MonBattleAppearance {
        match self {
            Self::Inactive(appearance) => appearance,
            Self::Active {
                primary_battle_appearance,
                ..
            } => primary_battle_appearance,
        }
    }

    fn matches_switch_in(&self, appearance: &MonBattleAppearanceFromSwitchIn) -> bool {
        self.primary().level.can_be(&appearance.level)
            && self.primary().health.can_be(&appearance.health)
            && self.primary().status.can_be(&appearance.status)
    }

    fn switch_in(&mut self) {
        let mut taken = Self::default();
        std::mem::swap(self, &mut taken);
        *self = match taken {
            Self::Inactive(appearance) => Self::Active {
                primary_battle_appearance: appearance.clone(),
                battle_appearance_up_to_last_switch_out: appearance,
                battle_appearance_from_last_switch_in: MonBattleAppearance::default(),
            },
            active @ Self::Active { .. } => active,
        }
    }

    fn switch_out(&mut self) {
        let mut taken = Self::default();
        std::mem::swap(self, &mut taken);
        *self = match taken {
            Self::Inactive(appearance) => Self::Inactive(appearance),
            Self::Active {
                primary_battle_appearance,
                ..
            } => Self::Inactive(primary_battle_appearance),
        }
    }

    fn recover(&mut self) -> MonBattleAppearance {
        match self {
            Self::Inactive(_) => MonBattleAppearance::default(),
            Self::Active {
                primary_battle_appearance,
                battle_appearance_up_to_last_switch_out,
                battle_appearance_from_last_switch_in,
            } => {
                *primary_battle_appearance = battle_appearance_up_to_last_switch_out.clone();
                let mut out = MonBattleAppearance::default();
                std::mem::swap(&mut out, battle_appearance_from_last_switch_in);
                out
            }
        }
    }

    fn apply_for_each_battle_appearance<F>(&mut self, f: F)
    where
        F: Fn(&mut MonBattleAppearance),
    {
        match self {
            Self::Inactive(appearance) => f(appearance),
            Self::Active {
                primary_battle_appearance,
                battle_appearance_from_last_switch_in,
                ..
            } => {
                f(primary_battle_appearance);
                f(battle_appearance_from_last_switch_in);
            }
        }
    }

    fn record_level(&mut self, level: DiscoveryRequired<u64>, ambiguity: Ambiguity) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_level(level.clone(), ambiguity);
        });
    }

    fn record_health(&mut self, health: DiscoveryRequired<(u64, u64)>, ambiguity: Ambiguity) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_health(health.clone(), ambiguity);
        });
    }

    fn record_status(&mut self, status: DiscoveryRequired<String>, ambiguity: Ambiguity) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_status(status.clone(), ambiguity);
        });
    }

    fn record_ability(&mut self, ability: DiscoveryRequired<String>, ambiguity: Ambiguity) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_ability(ability.clone(), ambiguity);
        });
    }

    fn record_item(&mut self, item: DiscoveryRequired<String>, ambiguity: Ambiguity) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_item(item.clone(), ambiguity);
        });
    }

    fn record_move(&mut self, name: String, ambiguity: Ambiguity) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_move(name.clone(), ambiguity);
        });
    }

    fn record_all(&mut self, other: MonBattleAppearance) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_all(other.clone());
        });
    }
}

impl Default for MonBattleAppearanceWithRecovery {
    fn default() -> Self {
        Self::Inactive(MonBattleAppearance::default())
    }
}

impl From<MonBattleAppearance> for MonBattleAppearanceWithRecovery {
    fn from(value: MonBattleAppearance) -> Self {
        Self::Inactive(value)
    }
}

/// A single Mon in a battle.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Mon {
    pub physical_appearance: MonPhysicalAppearance,
    pub battle_appearances: VecDeque<MonBattleAppearanceWithRecovery>,
    pub fainted: bool,
    pub volatile_data: MonVolatileData,
}

impl Mon {
    const MAX_BATTLE_APPEARANCES_LENGTH: usize = 3;

    fn new<I>(physical_appearance: MonPhysicalAppearance, battle_appearances: I) -> Self
    where
        I: IntoIterator<Item = MonBattleAppearance>,
    {
        Self {
            physical_appearance,
            battle_appearances: battle_appearances
                .into_iter()
                .map(|appearance| appearance.into())
                .collect(),
            ..Default::default()
        }
    }

    fn switch_in(&mut self) {
        self.revive();
        self.volatile_data = MonVolatileData::default();
    }

    fn switch_out(&mut self) {
        self.volatile_data = MonVolatileData::default();

        for battle_appearance in &mut self.battle_appearances {
            battle_appearance.switch_out();
        }
    }

    fn faint(&mut self) {
        self.fainted = true;
        self.switch_out();
    }

    fn revive(&mut self) {
        self.fainted = false;
    }

    fn push_battle_appearance(&mut self) -> usize {
        // If we exceed maximum number of battle appearances, recycle it as the base.
        let appearance = if self.battle_appearances.len() >= Self::MAX_BATTLE_APPEARANCES_LENGTH {
            // SAFETY: self.battle_appearances is not empty.
            self.battle_appearances.pop_front().unwrap().take_primary()
        } else {
            MonBattleAppearance::default()
        };
        let appearance = appearance.make_ambiguous();
        self.battle_appearances.push_back(appearance.into());
        self.battle_appearances.len() - 1
    }

    fn remove_battle_appearance(&mut self, index: usize) {
        self.battle_appearances.remove(index);
    }
}

/// A reference to a [`MonBattleAppearance`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonBattleAppearanceReference {
    pub player: String,
    pub mon_index: usize,
    pub battle_appearance_index: usize,
}

/// A player participating in a battle, which consists of one or more Mons.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Player {
    pub name: String,
    pub id: String,
    pub position: usize,
    pub team_size: usize,
    pub mons: Vec<Mon>,
    pub left_battle: bool,
}

impl Player {
    /// Checks if the player has Mons that cannot be distinguished between one another in some
    /// scenarios.
    ///
    /// If two Mons have the exact same physical appearance, they are considered ambiguous and some
    /// searching functions may not work as intended.
    pub fn has_ambiguous_mons(&self) -> bool {
        self.mons
            .iter()
            .map(|mon| mon.physical_appearance.clone())
            .collect::<HashSet<_>>()
            .len()
            == self.mons.len()
    }
}

/// A side of the battle, which consists of one or more players and a list of active Mons.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Side {
    pub name: String,
    pub id: usize,
    pub players: BTreeMap<String, Player>,
    pub conditions: BTreeMap<String, ConditionData>,
    pub active: Vec<Option<MonBattleAppearanceReference>>,
}

impl Side {
    fn player_or_else(&self, player: &str) -> Result<&Player> {
        self.players
            .get(player)
            .ok_or_else(|| Error::msg(format!("player {player} does not exist")))
    }

    fn player_mut_or_else(&mut self, player: &str) -> Result<&mut Player> {
        self.players
            .get_mut(player)
            .ok_or_else(|| Error::msg(format!("player {player} does not exist")))
    }

    fn mon_by_reference_or_else(&self, reference: &MonBattleAppearanceReference) -> Result<&Mon> {
        let player = self.player_or_else(&reference.player)?;
        player.mons.get(reference.mon_index).ok_or_else(|| {
            Error::msg(format!(
                "mon at index {} does not exist",
                reference.mon_index
            ))
        })
    }

    fn mon_mut_by_reference_or_else(
        &mut self,
        reference: &MonBattleAppearanceReference,
    ) -> Result<&mut Mon> {
        let player = self.player_mut_or_else(&reference.player)?;
        player.mons.get_mut(reference.mon_index).ok_or_else(|| {
            Error::msg(format!(
                "mon at index {} does not exist",
                reference.mon_index
            ))
        })
    }

    fn mon_battle_appearance_with_recovery_by_reference_or_else(
        &self,
        reference: &MonBattleAppearanceReference,
    ) -> Result<&MonBattleAppearanceWithRecovery> {
        self.mon_by_reference_or_else(reference)?
            .battle_appearances
            .get(reference.battle_appearance_index)
            .ok_or_else(|| {
                Error::msg(format!(
                    "battle data at index {} does not exist",
                    reference.battle_appearance_index
                ))
            })
    }

    fn mon_battle_appearance_with_recovery_mut_by_reference_or_else(
        &mut self,
        reference: &MonBattleAppearanceReference,
    ) -> Result<&mut MonBattleAppearanceWithRecovery> {
        self.mon_mut_by_reference_or_else(reference)?
            .battle_appearances
            .get_mut(reference.battle_appearance_index)
            .ok_or_else(|| {
                Error::msg(format!(
                    "battle data at index {} does not exist",
                    reference.battle_appearance_index
                ))
            })
    }

    fn active_mon_reference_by_position(
        &self,
        position: usize,
    ) -> Option<MonBattleAppearanceReference> {
        self.active.get(position).cloned().flatten()
    }

    fn mon_index_is_active(&self, index: usize) -> bool {
        self.active.iter().any(|active| {
            active
                .as_ref()
                .is_some_and(|active| active.mon_index == index)
        })
    }

    fn mon_is_active(&self, reference: &MonBattleAppearanceReference) -> bool {
        self.active
            .iter()
            .any(|active| active.as_ref().is_some_and(|active| active == reference))
    }

    fn mons_by_name(
        &mut self,
        player: &str,
        name: &str,
        active_filter: Option<bool>,
    ) -> Result<Vec<MonBattleAppearanceReference>> {
        let player = self.player_or_else(player)?;
        Ok(player
            .mons
            .iter()
            .enumerate()
            .filter(|(_, mon)| mon.physical_appearance.name == name)
            .flat_map(|(mon_index, mon)| {
                (0..mon.battle_appearances.len()).map(move |index| MonBattleAppearanceReference {
                    player: player.id.to_owned(),
                    mon_index,
                    battle_appearance_index: index,
                })
            })
            .filter(|mon| match active_filter {
                Some(active) => self.mon_is_active(mon) == active,
                None => true,
            })
            .collect())
    }

    fn mon_by_appearance(
        &mut self,
        player_id: &str,
        physical_appearance: &MonPhysicalAppearance,
        battle_appearance: Option<&MonBattleAppearanceFromSwitchIn>,
    ) -> Result<MonBattleAppearanceReference> {
        let player = self.player_or_else(player_id)?;
        let player_has_seen_all_mons = player.mons.len() >= player.team_size;
        let mons_by_appearance = player
            .mons
            .iter()
            .enumerate()
            .filter(|(mon_index, mon)| {
                mon.physical_appearance.matches(&physical_appearance)
                    && (player_has_seen_all_mons || !self.mon_index_is_active(*mon_index))
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let inactive_mon_references_by_battle_appearance = mons_by_appearance
            .iter()
            .flat_map(|mon_index| {
                // SAFETY: mon_index was generated from enumeration of player.mons.
                let mon = player.mons.get(*mon_index).unwrap();
                mon.battle_appearances
                    .iter()
                    .enumerate()
                    .filter(|(_, mon)| match battle_appearance {
                        Some(battle_appearance) => mon.matches_switch_in(battle_appearance),
                        None => true,
                    })
                    .map(move |(index, _)| MonBattleAppearanceReference {
                        player: player.id.to_owned(),
                        mon_index: *mon_index,
                        battle_appearance_index: index,
                    })
                    .filter(|reference| !self.mon_is_active(&reference))
            })
            .collect::<Vec<_>>();

        // If we matched some Mon battle appearance directly, just use the first one.
        if let Some(mon_reference) = inactive_mon_references_by_battle_appearance
            .into_iter()
            .next()
        {
            return Ok(mon_reference);
        }

        // If we matched some Mon by appearance, and we do not have room for any more unique Mons,
        // push the new appearance to the matched Mon.
        if let Some(mon_index) = mons_by_appearance.first().cloned()
            && player_has_seen_all_mons
        {
            let player = self.player_mut_or_else(player_id)?;
            // SAFETY: mon_index was generated from an enumeration of player.mons.
            let mon = player.mons.get_mut(mon_index).unwrap();
            let battle_appearance_index = mon.push_battle_appearance();
            return Ok(MonBattleAppearanceReference {
                player: player.id.to_owned(),
                mon_index,
                battle_appearance_index,
            });
        }

        // If we hit the maximum number of Mons, attempt to free a position up by merging two Mons
        // together.
        //
        // This should pretty much always work unless the player's team size was not correct and we
        // have seen more Mons with different physical appearances.
        let replace_index = if player_has_seen_all_mons {
            self.merge_one_mon(player_id)?
        } else {
            None
        };

        // Otherwise, add a brand new Mon.
        let replace_index = match replace_index {
            Some(replace_index) => replace_index,
            None => {
                let player = self.player_mut_or_else(player_id)?;
                player.mons.push(Mon::default());
                player.mons.len() - 1
            }
        };

        let player = self.player_mut_or_else(player_id)?;
        // SAFETY: replace_index is a valid index into player.mons.
        let mon = player.mons.get_mut(replace_index).unwrap();

        mon.physical_appearance = physical_appearance.clone();
        let battle_appearance_index = mon.push_battle_appearance();

        Ok(MonBattleAppearanceReference {
            player: player.id.to_owned(),
            mon_index: replace_index,
            battle_appearance_index,
        })
    }

    fn switch_in(
        &mut self,
        player_id: &str,
        physical_appearance: &MonPhysicalAppearance,
        battle_appearance: &MonBattleAppearanceFromSwitchIn,
        ignore_battle_appearance_for_matching: bool,
    ) -> Result<MonBattleAppearanceReference> {
        let reference = self.mon_by_appearance(
            player_id,
            physical_appearance,
            if ignore_battle_appearance_for_matching {
                None
            } else {
                Some(battle_appearance)
            },
        )?;

        // If the Mon happens to be fainted, then it fainted as an illusion.
        let mon = self.mon_mut_by_reference_or_else(&reference)?;
        if mon.fainted {
            self.faint_an_inactive_illusion_user(player_id)?;
        }

        let mon = self.mon_mut_by_reference_or_else(&reference)?;
        mon.switch_in();

        let mon_battle_appearance =
            self.mon_battle_appearance_with_recovery_mut_by_reference_or_else(&reference)?;
        mon_battle_appearance.switch_in();
        mon_battle_appearance.record_all(battle_appearance.into());

        Ok(reference)
    }

    fn faint_an_inactive_illusion_user(&mut self, player_id: &str) -> Result<()> {
        let player = self.player_or_else(player_id)?;
        let mon = player
            .mons
            .iter()
            .enumerate()
            .filter(|(mon_index, mon)| {
                !mon.fainted
                    && mon.battle_appearances.iter().enumerate().any(
                        |(battle_appearance_index, battle_appearance)| {
                            battle_appearance.primary().ability.can_be("Illusion")
                                && !self.mon_is_active(&MonBattleAppearanceReference {
                                    player: player_id.to_owned(),
                                    mon_index: *mon_index,
                                    battle_appearance_index,
                                })
                        },
                    )
            })
            .map(|(i, _)| i)
            .next();

        if let Some(mon) = mon {
            let player = self.player_mut_or_else(player_id)?;
            // SAFETY: Index is generated from enumeration of player.mons.
            let mon = player.mons.get_mut(mon).unwrap();
            mon.faint();
        }

        Ok(())
    }

    fn merge_one_mon(&mut self, player_id: &str) -> Result<Option<usize>> {
        let player = self.player_or_else(player_id)?;
        for mon_index in 0..player.mons.len() {
            let player = self.player_or_else(player_id)?;

            // SAFETY: Index is always less than player.mons.len().
            let mon = player.mons.get(mon_index).unwrap();

            // Cannot merge into a fainted Mon.
            if mon.fainted {
                continue;
            }

            // Cannot merge into an active Mon.
            if self.mon_index_is_active(mon_index) {
                continue;
            }

            // For an inactive Mon, attempt to merge in another inactive Mon with the same
            // appearance.
            let mut other_mons = player
                .mons
                .iter()
                .enumerate()
                .filter(|(i, other_mon)| {
                    *i != mon_index
                        && other_mon
                            .physical_appearance
                            .matches(&mon.physical_appearance)
                        && !self.mon_index_is_active(*i)
                })
                .map(|(i, mon)| (i, mon.fainted))
                .collect::<Vec<_>>();

            // Put unfainted Mons first.
            other_mons.sort_by_key(|(i, fainted)| {
                if *fainted {
                    *i as isize
                } else {
                    -(*i as isize)
                }
            });

            let (replace_index, fainted) = match other_mons.first() {
                Some(index) => *index,
                None => continue,
            };
            // An unfainted Mon is not available, but a fainted one is.
            //
            // This means that an illusion user fainted in place of this Mon, since we hit the team
            // size limit (which is why a merge occurs in the first place).
            //
            // We should find some illusion user and faint it. If we select the wrong one, that Mon
            // will appear and cause other illusion users to faint.
            if fainted {
                self.faint_an_inactive_illusion_user(player_id)?;
            }

            // Move battle appearances out of this Mon and into the other, effectively merging the
            // two.

            let player = self.player_mut_or_else(&player_id)?;
            // SAFETY: other_mon_index is an index into player.mons.
            let other_mon = player.mons.get_mut(replace_index).unwrap();
            let mut battle_appearances = VecDeque::default();
            std::mem::swap(&mut battle_appearances, &mut other_mon.battle_appearances);

            // SAFETY: Index is always less than player.mons.len().
            let mon = player.mons.get_mut(mon_index).unwrap();

            // Move the BattleAppearance.
            mon.battle_appearances.extend(battle_appearances);

            return Ok(Some(replace_index));
        }

        // We were not able to remove anything.
        Ok(None)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Field {
    pub sides: Vec<Side>,
    pub weather: Option<String>,
    pub conditions: BTreeMap<String, ConditionData>,
    pub rules: Vec<String>,
    pub max_side_length: usize,
}

impl Field {
    fn side_mut_or_else(&mut self, side: usize) -> Result<&mut Side> {
        self.sides
            .get_mut(side)
            .ok_or_else(|| Error::msg(format!("side {side} does not exist")))
    }

    fn side_and_player_or_else(&self, player: &str) -> Result<(usize, &Player)> {
        for (i, side) in self.sides.iter().enumerate() {
            if let Ok(player) = side.player_or_else(player) {
                return Ok((i, player));
            }
        }
        return Err(Error::msg(format!("player {player} does not exist")));
    }

    fn side_and_player_mut_or_else(&mut self, player: &str) -> Result<(usize, &mut Player)> {
        for (i, side) in self.sides.iter_mut().enumerate() {
            if let Ok(player) = side.player_mut_or_else(player) {
                return Ok((i, player));
            }
        }
        return Err(Error::msg(format!("player {player} does not exist")));
    }

    fn side_for_player(&self, player: &str) -> Result<usize> {
        self.side_and_player_or_else(player).map(|(side, _)| side)
    }

    fn player_mut_or_else(&mut self, player: &str) -> Result<&mut Player> {
        self.side_and_player_mut_or_else(player)
            .map(|(_, player)| player)
    }

    fn active_mons_on_side<'a>(
        &'a mut self,
        side: usize,
    ) -> Box<dyn Iterator<Item = (usize, MonBattleAppearanceReference)> + 'a> {
        match self.sides.get(side) {
            Some(side) => Box::new(
                side.active
                    .iter()
                    .enumerate()
                    .filter_map(|(i, val)| val.clone().map(|val| (i, val))),
            ),
            None => Box::new(std::iter::empty()),
        }
    }

    fn active_mons(&mut self) -> impl Iterator<Item = MonBattleAppearanceReference> {
        self.sides
            .iter()
            .flat_map(|side| side.active.iter().cloned().filter_map(|val| val))
    }

    fn mon_mut_by_reference_or_else(
        &mut self,
        reference: &MonBattleAppearanceReference,
    ) -> Result<&mut Mon> {
        let side = self.side_for_player(&reference.player)?;
        let side = self.side_mut_or_else(side)?;
        side.mon_mut_by_reference_or_else(reference)
    }

    fn mon_battle_appearance_with_recovery_mut_by_reference_or_else(
        &mut self,
        reference: &MonBattleAppearanceReference,
    ) -> Result<&mut MonBattleAppearanceWithRecovery> {
        let side = self.side_for_player(&reference.player)?;
        let side = self.side_mut_or_else(side)?;
        side.mon_battle_appearance_with_recovery_mut_by_reference_or_else(reference)
    }

    fn active_mon_reference_by_position(
        &mut self,
        side: usize,
        position: usize,
    ) -> Result<Option<MonBattleAppearanceReference>> {
        let side = self.side_mut_or_else(side)?;
        Ok(side.active_mon_reference_by_position(position))
    }

    fn active_mon_reference_by_position_or_else(
        &mut self,
        side: usize,
        position: usize,
    ) -> Result<MonBattleAppearanceReference> {
        self.active_mon_reference_by_position(side, position)?
            .ok_or_else(|| {
                Error::msg(format!(
                    "side {side} has no active mon in position {position}"
                ))
            })
    }

    fn mons_by_name(
        &mut self,
        player: &str,
        name: &str,
        active_filter: Option<bool>,
    ) -> Result<Vec<MonBattleAppearanceReference>> {
        let (side, _) = self.side_and_player_mut_or_else(player)?;
        self.side_mut_or_else(side)?
            .mons_by_name(player, name, active_filter)
    }

    fn mon_by_appearance(
        &mut self,
        player: &str,
        physical_appearance: &MonPhysicalAppearance,
        battle_appearance: Option<&MonBattleAppearanceFromSwitchIn>,
    ) -> Result<MonBattleAppearanceReference> {
        let (side, _) = self.side_and_player_mut_or_else(player)?;
        self.side_mut_or_else(side)?.mon_by_appearance(
            player,
            physical_appearance,
            battle_appearance,
        )
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum BattlePhase {
    #[default]
    PreBattle,
    PreTeamPreview,
    TeamPreview(usize),
    Battle,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct BattleState {
    pub phase: BattlePhase,
    pub turn: usize,
    pub battle_type: String,
    pub field: Field,
    pub ui_log: Vec<Vec<ui::UiLogEntry>>,
}

pub fn alter_battle_state(state: BattleState, log: &Log) -> Result<BattleState> {
    let mut state = state;
    alter_battle_state_internal(&mut state, log)?;
    Ok(state)
}

fn alter_battle_state_internal(state: &mut BattleState, log: &Log) -> Result<()> {
    let last_turn_in_state = state.turn.saturating_sub(1);
    let current_turn = log.current_turn();
    for turn in last_turn_in_state..current_turn {
        alter_battle_state_for_turn(state, log, turn)?;
    }
    state.turn = current_turn;
    Ok(())
}

fn alter_battle_state_for_turn(state: &mut BattleState, log: &Log, turn: usize) -> Result<()> {
    state.turn = turn.try_into().context("failed to convert turn number")?;

    let mut ui_log = Vec::default();
    for entry in log.entries_for_turn(turn) {
        alter_battle_state_for_entry(state, &mut ui_log, entry)?;
    }

    if turn + 1 > state.ui_log.len() {
        state.ui_log.resize_with(turn + 1, Vec::default);
    }
    // SAFETY: Resized above.
    *state.ui_log.get_mut(turn).unwrap() = ui_log;

    Ok(())
}

fn mon_name_from_log_entry(entry: &LogEntry) -> Result<MonName> {
    let name = entry
        .value("name")
        .ok_or_else(|| Error::msg("missing name"))?;
    let player = entry
        .value("player")
        .ok_or_else(|| Error::msg("missing player"))?;
    let position = entry
        .value::<usize>("position")
        .map(|position| position - 1);
    Ok(MonName {
        name,
        player,
        position,
    })
}

fn health_from_log_entry(entry: &LogEntry) -> Result<(u64, u64)> {
    entry
        .value_ref("health")
        .map(|health| match health.split_once('/') {
            Some((a, b)) => Ok((a.parse()?, b.parse()?)),
            None => Ok((health.parse()?, 1)),
        })
        .transpose()
        .map(|health| health.unwrap_or((0, 1)))
}

fn mon_appearance_from_log_entry(
    entry: &LogEntry,
) -> Result<(MonPhysicalAppearance, MonBattleAppearanceFromSwitchIn)> {
    let name: String = entry.value("name").unwrap_or_default();
    let species: String = entry.value("species").unwrap_or_default();
    let level: u64 = entry.value("level").unwrap_or_default();
    let gender = entry.value("gender").unwrap_or_default();
    let shiny = entry.value_ref("shiny").is_some();
    let health = health_from_log_entry(entry)?;
    let status: String = entry.value("status").unwrap_or_default();
    Ok((
        MonPhysicalAppearance {
            name,
            species,
            gender,
            shiny,
        },
        MonBattleAppearanceFromSwitchIn {
            level,
            health,
            status,
        },
    ))
}

fn mon_name_to_mon_for_ui_log(state: &mut BattleState, mon: &MonName) -> Result<ui::Mon> {
    match &mon.position {
        Some(position) => {
            let side = state.field.side_for_player(&mon.player)?;
            Ok(ui::Mon::Active(ui::FieldPosition {
                side,
                position: *position - 1,
            }))
        }
        None => Ok(ui::Mon::Inactive(ui::MonReference {
            player: mon.player.clone(),
            name: mon.name.clone(),
        })),
    }
}

fn effect_from_log_entry(entry: &LogEntry, effect_value_name: Option<&str>) -> Result<EffectName> {
    match effect_value_name {
        Some(name) => entry.value_or_else(name),
        None => {
            let check_effect_name = |entry: &LogEntry, name: &str| {
                entry.value::<String>(name).map(|value| EffectName {
                    effect_type: Some(name.to_owned()),
                    name: value,
                })
            };
            check_effect_name(entry, "move")
                .or_else(|| check_effect_name(entry, "ability"))
                .or_else(|| check_effect_name(entry, "item"))
                .or_else(|| check_effect_name(entry, "condition"))
                .or_else(|| check_effect_name(entry, "status"))
                .or_else(|| check_effect_name(entry, "type"))
                .or_else(|| check_effect_name(entry, "weather"))
                .or_else(|| check_effect_name(entry, "clause"))
                .or_else(|| check_effect_name(entry, "species"))
                .ok_or_else(|| Error::msg("missing effect"))
        }
    }
}

fn effect_data_from_log_entry(
    state: &mut BattleState,
    entry: &LogEntry,
    effect_value_name: Option<&str>,
) -> Result<ui::EffectData> {
    let effect = effect_from_log_entry(entry, effect_value_name).ok();
    let side = entry.value("side");
    let slot = entry.value("slot");
    let player = entry.value("player");
    let target = entry
        .value::<MonName>("mon")
        .map(|mon| mon_name_to_mon_for_ui_log(state, &mon))
        .transpose()?;
    let source = entry
        .value::<MonName>("of")
        .map(|mon| mon_name_to_mon_for_ui_log(state, &mon))
        .transpose()?;
    let source_effect = effect_from_log_entry(entry, Some("from")).ok();

    // Additional data that may be useful to the user interface for specific effects.
    let additional = entry
        .values()
        .filter(|(key, _)| match *key {
            "from" | "mon" | "of" | "player" | "side" | "slot" => false,
            _ => true,
        })
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect();

    Ok(ui::EffectData {
        effect: effect.map(|effect| effect.into()),
        side,
        slot,
        player,
        target,
        source,
        source_effect: source_effect.map(|effect| effect.into()),
        additional,
    })
}

fn mons_by_mon_name(
    state: &mut BattleState,
    mon: &MonName,
) -> Result<Vec<MonBattleAppearanceReference>> {
    match mon.position {
        Some(position) => {
            let side = state.field.side_for_player(&mon.player)?;
            Ok(state
                .field
                .active_mon_reference_by_position(side, position - 1)?
                .map(|mon| Vec::from_iter([mon]))
                .unwrap_or_default())
        }
        None => state
            .field
            .mons_by_name(&mon.player, &mon.name, Some(false)),
    }
}

fn apply_for_each_mon_reference<F>(state: &mut BattleState, mon: &MonName, f: F) -> Result<()>
where
    F: Fn(&mut BattleState, MonBattleAppearanceReference, Ambiguity) -> Result<()>,
{
    let mons = mons_by_mon_name(state, mon)?;

    let ambiguity = if mons.len() == 1 {
        Ambiguity::Precise
    } else {
        Ambiguity::Ambiguous
    };

    for mon in mons {
        f(state, mon, ambiguity)?;
    }

    Ok(())
}

fn apply_for_each_mon_battle_appearance<F>(
    state: &mut BattleState,
    mon: &MonName,
    f: F,
) -> Result<()>
where
    F: Fn(&mut MonBattleAppearanceWithRecovery, Ambiguity),
{
    apply_for_each_mon_reference(state, mon, |state, mon, ambiguity| {
        let mon = state
            .field
            .mon_battle_appearance_with_recovery_mut_by_reference_or_else(&mon)?;
        f(mon, ambiguity);
        Ok(())
    })
}

fn apply_for_each_mon<F>(state: &mut BattleState, mon: &MonName, f: F) -> Result<()>
where
    F: Fn(&mut Mon, Ambiguity),
{
    apply_for_each_mon_reference(state, mon, |state, mon, ambiguity| {
        let mon = state.field.mon_mut_by_reference_or_else(&mon)?;
        f(mon, ambiguity);
        Ok(())
    })
}

fn record_effect_from_mon(
    state: &mut BattleState,
    effect: &ui::Effect,
    mon: &MonName,
) -> Result<()> {
    match effect.effect_type.as_ref().map(|s| s.as_str()) {
        Some("ability") => {
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_ability(effect.name.clone().into(), ambiguity);
            })?;
        }
        Some("item") => {
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_item(effect.name.clone().into(), ambiguity);
            })?;
        }
        _ => (),
    }
    Ok(())
}

fn modify_state_from_effect(
    state: &mut BattleState,
    entry: &LogEntry,
    effect: &ui::EffectData,
) -> Result<()> {
    match (&effect.source_effect, entry.value::<MonName>("of")) {
        (Some(source_effect), Some(source)) => {
            record_effect_from_mon(state, source_effect, &source)?
        }
        _ => (),
    }

    match entry.title() {
        "ability" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect.effect {
                apply_for_each_mon(state, &mon, |mon, _| {
                    mon.volatile_data.record_ability(effect.name.clone());
                })?;
            }
        }
        "abilityend" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.record_ability(String::default());
            })?;
        }
        "activate" => match (&effect.effect, entry.value::<MonName>("mon")) {
            (Some(effect), Some(mon)) => record_effect_from_mon(state, effect, &mon)?,
            _ => (),
        },
        "clearnegativeboosts" => {
            for mon in state.field.active_mons().collect::<Vec<_>>() {
                let mon = state.field.mon_mut_by_reference_or_else(&mon)?;

                for stat in mon
                    .volatile_data
                    .stat_boosts
                    .keys()
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    if let std::collections::btree_map::Entry::Occupied(entry) =
                        mon.volatile_data.stat_boosts.entry(stat)
                        && *entry.get() < 0
                    {
                        entry.remove_entry();
                    }
                }
            }
        }
        "clearweather" => {
            state.field.weather = None;
        }
        "curestatus" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_status(String::default().into(), ambiguity);
            })?;
        }
        "damage" | "heal" => {
            let health = health_from_log_entry(&entry)?;
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_health(health.into(), ambiguity);
            })?;
        }
        "end" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect.effect {
                apply_for_each_mon(state, &mon, |mon, _| {
                    mon.volatile_data.remove_condition(&effect.name);
                })?;

                record_effect_from_mon(state, &effect, &mon)?;
            }
        }
        "faint" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.faint();
            })?;
        }
        "fieldend" => {
            if let Some(effect) = &effect.effect {
                state.field.conditions.remove(&effect.name);
            }
        }
        "fieldstart" => {
            if let Some(effect) = &effect.effect {
                state.field.conditions.insert(
                    effect.name.clone(),
                    ConditionData {
                        since_turn: state.turn,
                        ..Default::default()
                    },
                );
            }
        }
        "item" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect.effect {
                apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                    mon.record_item(effect.name.clone().into(), ambiguity);
                })?;
            }
        }
        "itemend" => {
            let mon = entry.value_or_else("mon")?;
            apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                mon.record_item(String::default().into(), ambiguity);
            })?;
        }
        "prepare" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect.effect {
                let turn = state.turn;
                apply_for_each_mon(state, &mon, |mon, _| {
                    mon.volatile_data.record_condition(
                        effect.name.clone(),
                        MonConditionData {
                            condition_data: ConditionData {
                                since_turn: turn,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    );
                })?;
            }
        }
        "sideend" => {
            let side = entry.value_or_else("side")?;
            let side = state.field.side_mut_or_else(side)?;
            if let Some(effect) = &effect.effect {
                side.conditions.remove(&effect.name);
            }
        }
        "sidestart" => {
            let side = entry.value_or_else("side")?;
            let side = state.field.side_mut_or_else(side)?;
            if let Some(effect) = &effect.effect {
                side.conditions.insert(
                    effect.name.clone(),
                    ConditionData {
                        since_turn: state.turn,
                        ..Default::default()
                    },
                );
            }
        }
        "singlemove" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect.effect {
                let turn = state.turn;
                apply_for_each_mon(state, &mon, |mon, _| {
                    mon.volatile_data.record_condition(
                        effect.name.clone(),
                        MonConditionData {
                            condition_data: ConditionData {
                                since_turn: turn,
                                ..Default::default()
                            },
                            until_target_moves: true,
                            ..Default::default()
                        },
                    );
                })?;
            }
        }
        "status" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect.effect {
                apply_for_each_mon_battle_appearance(state, &mon, |mon, ambiguity| {
                    mon.record_status(effect.name.clone().into(), ambiguity);
                })?;
            }
        }
        "start" => {
            let mon = entry.value_or_else("mon")?;
            if let Some(effect) = &effect.effect {
                let turn = state.turn;
                apply_for_each_mon(state, &mon, |mon, _| {
                    mon.volatile_data.record_condition(
                        effect.name.clone(),
                        MonConditionData {
                            condition_data: ConditionData {
                                since_turn: turn,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                    );
                })?;

                record_effect_from_mon(state, &effect, &mon)?;
            }
        }
        "typechange" => {
            let mon = entry.value_or_else("mon")?;
            let types: String = entry.value_or_else("types")?;
            let types = types.split('/').map(|s| s.to_owned()).collect::<Vec<_>>();
            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.record_types(types.clone());
            })?;
        }
        "weather" => {
            if let Some(effect) = &effect.effect {
                state.field.weather = Some(effect.name.clone());
            }
        }
        _ => (),
    }
    Ok(())
}

fn alter_battle_state_for_entry(
    state: &mut BattleState,
    ui_log: &mut Vec<ui::UiLogEntry>,
    entry: &LogEntry,
) -> Result<()> {
    match entry.title() {
        "ability"
        | "abilityend"
        | "activate"
        | "block"
        | "cant"
        | "catch"
        | "catchfailed"
        | "clearnegativeboosts"
        | "clearweather"
        | "curestatus"
        | "crit"
        | "damage"
        | "deductpp"
        | "end"
        | "fail"
        | "faint"
        | "fieldactivate"
        | "fieldend"
        | "fieldstart"
        | "formechange"
        | "heal"
        | "hitcount"
        | "immune"
        | "item"
        | "itemend"
        | "miss"
        | "ohko"
        | "prepare"
        | "resisted"
        | "restorepp"
        | "revive"
        | "sethp"
        | "setpp"
        | "sidestart"
        | "sideend"
        | "singlemove"
        | "singleturn"
        | "status"
        | "start"
        | "supereffective"
        | "typechange"
        | "uncatchable"
        | "weather" => {
            let effect_value_name = match entry.title() {
                "cant" => Some("reason"),
                "fail" => Some("what"),
                _ => None,
            };
            let effect = effect_data_from_log_entry(state, entry, effect_value_name)?;
            modify_state_from_effect(state, entry, &effect)?;

            // Generate UI log for the effect. Some effects may have special logs.
            match entry.title() {
                "catch" => {
                    ui_log.push(ui::UiLogEntry::Caught { effect });
                }
                "damage" => {
                    let health = health_from_log_entry(entry)?;
                    ui_log.push(ui::UiLogEntry::Damage { health, effect });
                }
                "faint" => {
                    ui_log.push(ui::UiLogEntry::Faint { effect });
                }
                "formechange" => {
                    ui_log.push(ui::UiLogEntry::UpdateAppearance { effect });
                }
                "heal" => {
                    let health = health_from_log_entry(entry)?;
                    ui_log.push(ui::UiLogEntry::Heal { health, effect });
                }
                "revive" => {
                    ui_log.push(ui::UiLogEntry::Revive { effect });
                }
                "sethp" => {
                    let health = health_from_log_entry(entry)?;
                    ui_log.push(ui::UiLogEntry::SetHealth { health, effect });
                }
                _ => {
                    ui_log.push(ui::UiLogEntry::Effect {
                        title: entry.title().to_owned(),
                        effect,
                    });
                }
            }
        }
        "battlestart" => {
            state.phase = BattlePhase::Battle;
        }
        "boost" | "unboost" => {
            let mon: MonName = entry.value_or_else("mon")?;

            let stat: String = entry.value_or_else("stat")?;
            let by: i64 = entry.value_or_else("by")?;
            let by = if entry.title() == "unboost" { -by } else { by };

            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.record_stat_boost(stat.clone(), by);
            })?;

            ui_log.push(ui::UiLogEntry::StatBoost {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                stat,
                by,
            });
        }
        "cannotescape" => {
            let player = entry.value_or_else("player")?;
            ui_log.push(ui::UiLogEntry::PlayerMessage {
                title: entry.title().to_owned(),
                player,
            });
        }
        "debug" | "fxlang_debug" => ui_log.push(ui::UiLogEntry::Debug {
            title: entry.title().to_owned(),
            values: entry
                .values()
                .map(|(k, v)| (k.to_owned(), v.to_owned()))
                .collect(),
        }),
        "didnotlearnmove" => {
            let mon = entry.value_or_else("mon")?;
            let move_name = entry.value_or_else("name")?;
            ui_log.push(ui::UiLogEntry::MoveUpdateRejected {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                move_name,
            });
        }
        "escaped" | "forfeited" => {
            let player: String = entry.value_or_else("player")?;
            let side_index = state.field.side_for_player(&player)?;

            state.field.player_mut_or_else(&player)?.left_battle = true;

            // All Mons belonging to the player leave immediately.
            let active_mons = state
                .field
                .active_mons_on_side(side_index)
                .filter(|(_, reference)| reference.player == player)
                .collect::<Vec<_>>();

            let side = state.field.side_mut_or_else(side_index)?;
            for (i, mon) in &active_mons {
                // SAFETY: active_mons_on_side returns valid indices for side.active.
                *side.active.get_mut(*i).unwrap() = None;
                side.mon_mut_by_reference_or_else(mon)?.switch_out();
            }

            ui_log.push(ui::UiLogEntry::Leave {
                title: entry.title().to_owned(),
                player: player.clone(),
                positions: active_mons
                    .into_iter()
                    .map(|(i, _)| ui::FieldPosition {
                        side: side_index,
                        position: i,
                    })
                    .collect(),
            });
        }
        "exp" => {
            let mon = entry.value_or_else("mon")?;
            let exp = entry.value_or_else("exp")?;
            ui_log.push(ui::UiLogEntry::Experience {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                exp,
            })
        }
        "info" => {
            if let Some(battle_type) = entry.value::<String>("battletype") {
                state.battle_type = battle_type.to_lowercase();
            }
            if let Some(rule) = entry.value::<String>("rule") {
                state.field.rules.push(rule.to_owned());
            }
        }
        "learnedmove" => {
            let mon = entry.value_or_else("mon")?;
            let move_name = entry.value_or_else("name")?;
            ui_log.push(ui::UiLogEntry::MoveUpdateRejected {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                move_name,
            });
        }
        "levelup" => {
            let mon = entry.value_or_else("mon")?;
            let level = entry.value_or_else("level")?;

            let mut stats = HashMap::default();

            let mut add_stat_to_map_if_present = |name: &str| {
                if let Some(stat) = entry.value(name) {
                    stats.insert(name.to_owned(), stat);
                }
            };
            add_stat_to_map_if_present("hp");
            add_stat_to_map_if_present("atk");
            add_stat_to_map_if_present("def");
            add_stat_to_map_if_present("spa");
            add_stat_to_map_if_present("spd");
            add_stat_to_map_if_present("spe");

            ui_log.push(ui::UiLogEntry::LevelUp {
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                level,
                stats,
            });
        }
        "maxsidelength" => {
            state.field.max_side_length = entry.value_or_else("length")?;
        }
        "mon" => {
            let (physical_appearance, battle_appearance) = mon_appearance_from_log_entry(entry)?;
            let player: String = entry.value_or_else("player")?;
            let player = state.field.player_mut_or_else(&player)?;
            player
                .mons
                .push(Mon::new(physical_appearance, [(&battle_appearance).into()]));
        }
        "move" | "animatemove" => {
            let mon: MonName = entry.value_or_else("mon")?;
            let name: String = entry.value_or_else("name")?;
            let used_directly = entry.title() == "move";
            let target: Option<MonName> = entry.value("target");
            let spread: Option<MonNameList> = entry.value("spread");
            let from: Option<EffectName> = entry.value("from");
            let animate = entry.value_ref("noanim").is_none();

            if used_directly && from.is_none() {
                apply_for_each_mon_reference(state, &mon, |state, mon_reference, ambiguity| {
                    let mon = state.field.mon_mut_by_reference_or_else(&mon_reference)?;
                    if !mon.volatile_data.conditions.contains_key(&name) {
                        let mon = state
                            .field
                            .mon_battle_appearance_with_recovery_mut_by_reference_or_else(
                                &mon_reference,
                            )?;
                        mon.record_move(name.clone(), ambiguity);
                    }
                    Ok(())
                })?;
            }

            apply_for_each_mon(state, &mon, |mon, _| {
                mon.volatile_data.remove_condition(&name);

                for name in mon
                    .volatile_data
                    .conditions
                    .iter()
                    .filter_map(|(key, val)| val.until_target_moves.then_some(key))
                    .cloned()
                    .collect::<Vec<_>>()
                {
                    mon.volatile_data.remove_condition(&name);
                }
            })?;

            ui_log.push(ui::UiLogEntry::Move {
                name,
                mon: mon_name_to_mon_for_ui_log(state, &mon)?,
                target: if let Some(spread) = spread {
                    Some(ui::MoveTarget::Spread(
                        spread
                            .0
                            .into_iter()
                            .map(|mon| mon_name_to_mon_for_ui_log(state, &mon))
                            .collect::<Result<HashSet<_>>>()?,
                    ))
                } else if let Some(mon) = target {
                    Some(ui::MoveTarget::Single(mon_name_to_mon_for_ui_log(
                        state, &mon,
                    )?))
                } else {
                    None
                },
                animate,
            })
        }
        "player" => {
            let id: String = entry.value_or_else("id")?;
            let name = entry.value_or_else("name")?;
            let side: usize = entry.value_or_else("side")?;
            let position = entry.value_or_else::<usize>("position")?;
            let side = state.field.side_mut_or_else(side)?;
            side.players.insert(
                id.clone(),
                Player {
                    name,
                    id,
                    position,
                    ..Default::default()
                },
            );
        }
        "residual" => (),
        "side" => {
            let id: usize = entry.value_or_else("id")?;
            let name = entry.value_or_else("name")?;
            if id + 1 > state.field.sides.len() {
                state.field.sides.resize_with(id + 1, Side::default);
            }
            // SAFETY: Resized above.
            let side = state.field.side_mut_or_else(id).unwrap();
            side.id = id;
            side.name = name;
        }
        "specieschange" => {
            let (physical_appearance, _) = mon_appearance_from_log_entry(entry)?;
            let mon = mon_name_from_log_entry(entry)?;
            apply_for_each_mon(state, &mon, |mon, ambiguity| {
                mon.physical_appearance.species = match ambiguity {
                    Ambiguity::Ambiguous => String::default(),
                    Ambiguity::Precise => physical_appearance.species.clone(),
                }
            })?;
        }
        "switch" | "drag" | "appear" | "replace" => {
            let (physical_appearance, battle_appearance) = mon_appearance_from_log_entry(entry)?;
            let player: String = entry.value_or_else("player")?;
            let position = entry.value_or_else::<usize>("position")? - 1;

            let side_index = state.field.side_for_player(&player)?;
            let side = state.field.side_mut_or_else(side_index)?;

            if position + 1 > side.active.len() {
                side.active.resize_with(position + 1, Option::default);
            }

            // SAFETY: Resized above.
            let previous = side.active.get_mut(position).cloned().unwrap();

            let replace = entry.title() == "replace";
            let mut current_appearance = None;

            // First, handle illusion recovery.
            if let Some(previous) = &previous {
                // If applicable, handle illusion recovery first.

                if replace {
                    // When an illusion breaks, we recover the old Mon before switching it out, and
                    // move the current appearance to the replacement Mon.
                    current_appearance = Some(
                        side.mon_battle_appearance_with_recovery_mut_by_reference_or_else(
                            &previous,
                        )?
                        .recover(),
                    );

                    // Mark that the previous Mon is inactive immediately.
                    //
                    // Ordinarily, we want the previous Mon to still be considered active when
                    // switching in the new Mon, so that it is clear that the new and previous Mons
                    // are distinct. However, in the case of illusion replacement, the Mon that the
                    // illusion took the appearance of was *never* active, so we want that Mon to be
                    // a candidate for merging.
                    //
                    // There is technically an edge case here: if an illusion user creates an
                    // illusion of a Mon that looks exactly identical (by physical appearance) to
                    // it, then when the illusion breaks, the active Mon will not really change.
                    // However, this case is acceptable because there is ambiguity *anyway*. To
                    // avoid this edge case, we would need to track switching for non-overlap with a
                    // separate field somewhere.
                    //
                    // SAFETY: Resized above.
                    *side.active.get_mut(position).unwrap() = None;
                }

                side.mon_mut_by_reference_or_else(previous)?.switch_out();

                // If the replaced Mon ends up empty, we can remove that battle appearance.
                if side
                    .mon_battle_appearance_with_recovery_mut_by_reference_or_else(&previous)?
                    .primary()
                    .is_empty()
                {
                    side.mon_mut_by_reference_or_else(&previous)?
                        .remove_battle_appearance(previous.battle_appearance_index);
                }
            }

            // Third, switch in the new Mon.
            //
            // This may result in some merging.
            let mon = side.switch_in(&player, &physical_appearance, &battle_appearance, replace)?;
            let mon_index = mon.mon_index;

            // Fourth, merge in the current appearance from prior to the illusion breaking, if
            // applicable.
            if let Some(current_appearance) = current_appearance {
                side.mon_battle_appearance_with_recovery_mut_by_reference_or_else(&mon)?
                    .record_all(current_appearance);
            }

            // Finally, set the active position to the new Mon.
            //
            // SAFETY: Resized above.
            *side.active.get_mut(position).unwrap() = Some(mon.clone());

            ui_log.push(ui::UiLogEntry::Switch {
                switch_type: entry.title().to_owned(),
                player,
                mon: mon_index,
                into_position: ui::FieldPosition {
                    side: side_index,
                    position,
                },
            });
        }
        "teampreviewstart" => {
            state.phase = BattlePhase::PreTeamPreview;
        }
        "teampreview" => {
            let pick = entry.value_or_else("pick")?;
            state.phase = BattlePhase::TeamPreview(pick);
        }
        "teamsize" => {
            let player: String = entry.value_or_else("player")?;
            let size = entry.value_or_else("size")?;
            let player = state.field.player_mut_or_else(&player)?;
            player.team_size = size;

            // TODO: We could try to remember Mons from team preview and match them up as they
            // appear.
            player.mons.clear();
        }
        "tie" => {
            ui_log.push(ui::UiLogEntry::Tie);
        }
        "time" => (),
        "transform" => todo!(),
        "turn" => (),
        "turnlimit" => {
            ui_log.push(ui::UiLogEntry::Message {
                content: "The battle reached the turn limit. The battle will end in a tie."
                    .to_owned(),
            });
        }
        "useitem" => {
            let player = entry.value_or_else("player")?;
            let item = entry.value_or_else("item")?;
            let target = entry.value("mon");
            ui_log.push(ui::UiLogEntry::UseItem {
                player,
                item,
                target: target
                    .map(|target| mon_name_to_mon_for_ui_log(state, &target))
                    .transpose()?,
            });
        }
        "win" => {
            let side = entry.value_or_else("side")?;
            ui_log.push(ui::UiLogEntry::Win { side });
        }
        title @ _ => return Err(Error::msg(format!("unsupported log: {title}"))),
    }
    Ok(())
}

#[cfg(test)]
mod state_test {
    use std::collections::{
        BTreeMap,
        VecDeque,
    };

    use ahash::HashMap;

    use crate::{
        discovery::{
            DiscoveryRequired,
            DiscoveryRequiredSet,
        },
        log::Log,
        state::{
            BattlePhase,
            BattleState,
            Field,
            Mon,
            MonBattleAppearance,
            MonBattleAppearanceReference,
            MonBattleAppearanceWithRecovery,
            MonPhysicalAppearance,
            MonVolatileData,
            Player,
            Side,
            alter_battle_state,
        },
        ui,
    };

    #[test]
    fn constructs_sides_and_players_before_battle_start() {
        let log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "turn|turn:1",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state,
            BattleState {
                phase: BattlePhase::Battle,
                turn: 1,
                battle_type: "singles".to_owned(),
                field: Field {
                    sides: Vec::from_iter([
                        Side {
                            name: "Side 1".to_owned(),
                            id: 0,
                            players: BTreeMap::from_iter([(
                                "player-1".to_owned(),
                                Player {
                                    name: "Player 1".to_owned(),
                                    id: "player-1".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::default(),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::default(),
                            ..Default::default()
                        },
                        Side {
                            name: "Side 2".to_owned(),
                            id: 1,
                            players: BTreeMap::from_iter([(
                                "player-2".to_owned(),
                                Player {
                                    name: "Player 2".to_owned(),
                                    id: "player-2".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::default(),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::default(),
                            ..Default::default()
                        }
                    ]),
                    max_side_length: 2,
                    ..Default::default()
                },
                ui_log: Vec::from_iter([Vec::default()]),
            }
        );
    }

    #[test]
    fn adds_mon_for_initial_switch_in() {
        let log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state,
            BattleState {
                phase: BattlePhase::Battle,
                turn: 1,
                battle_type: "singles".to_owned(),
                field: Field {
                    sides: Vec::from_iter([
                        Side {
                            name: "Side 1".to_owned(),
                            id: 0,
                            players: BTreeMap::from_iter([(
                                "player-1".to_owned(),
                                Player {
                                    name: "Player 1".to_owned(),
                                    id: "player-1".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::from_iter([Mon {
                                        physical_appearance: MonPhysicalAppearance {
                                            name: "Squirtle".to_owned(),
                                            species: "Squirtle".to_owned(),
                                            gender: "M".to_owned(),
                                            ..Default::default()
                                        },
                                        battle_appearances: VecDeque::from_iter([
                                            MonBattleAppearanceWithRecovery::Active {
                                                primary_battle_appearance: MonBattleAppearance {
                                                    level: 5.into(),
                                                    health: (100, 100).into(),
                                                    status: String::default().into(),
                                                    ..Default::default()
                                                },
                                                battle_appearance_up_to_last_switch_out:
                                                    MonBattleAppearance::default(),
                                                battle_appearance_from_last_switch_in:
                                                    MonBattleAppearance {
                                                        level: 5.into(),
                                                        health: (100, 100).into(),
                                                        status: String::default().into(),
                                                        ..Default::default()
                                                    },
                                            }
                                        ]),
                                        fainted: false,
                                        volatile_data: MonVolatileData::default(),
                                    }]),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::from_iter([Some(MonBattleAppearanceReference {
                                player: "player-1".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            })]),
                            ..Default::default()
                        },
                        Side {
                            name: "Side 2".to_owned(),
                            id: 1,
                            players: BTreeMap::from_iter([(
                                "player-2".to_owned(),
                                Player {
                                    name: "Player 2".to_owned(),
                                    id: "player-2".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::from_iter([Mon {
                                        physical_appearance: MonPhysicalAppearance {
                                            name: "Charmander".to_owned(),
                                            species: "Charmander".to_owned(),
                                            gender: "M".to_owned(),
                                            ..Default::default()
                                        },
                                        battle_appearances: VecDeque::from_iter([
                                            MonBattleAppearanceWithRecovery::Active {
                                                primary_battle_appearance: MonBattleAppearance {
                                                    level: 5.into(),
                                                    health: (100, 100).into(),
                                                    status: String::default().into(),
                                                    ..Default::default()
                                                },
                                                battle_appearance_up_to_last_switch_out:
                                                    MonBattleAppearance::default(),
                                                battle_appearance_from_last_switch_in:
                                                    MonBattleAppearance {
                                                        level: 5.into(),
                                                        health: (100, 100).into(),
                                                        status: String::default().into(),
                                                        ..Default::default()
                                                    },
                                            }
                                        ]),
                                        fainted: false,
                                        volatile_data: MonVolatileData::default(),
                                    }]),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::from_iter([Some(MonBattleAppearanceReference {
                                player: "player-2".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            })]),
                            ..Default::default()
                        }
                    ]),
                    max_side_length: 2,
                    ..Default::default()
                },
                ui_log: Vec::from_iter([Vec::from_iter([
                    ui::UiLogEntry::Switch {
                        switch_type: "switch".to_owned(),
                        player: "player-1".to_owned(),
                        mon: 0,
                        into_position: ui::FieldPosition {
                            side: 0,
                            position: 0,
                        }
                    },
                    ui::UiLogEntry::Switch {
                        switch_type: "switch".to_owned(),
                        player: "player-2".to_owned(),
                        mon: 0,
                        into_position: ui::FieldPosition {
                            side: 1,
                            position: 0,
                        }
                    }
                ])]),
            }
        );
    }

    #[test]
    fn records_simple_move_and_damage() {
        let log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "time|time:0",
            "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:75/100",
            "residual",
            "turn|turn:2",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state,
            BattleState {
                phase: BattlePhase::Battle,
                turn: 2,
                battle_type: "singles".to_owned(),
                field: Field {
                    sides: Vec::from_iter([
                        Side {
                            name: "Side 1".to_owned(),
                            id: 0,
                            players: BTreeMap::from_iter([(
                                "player-1".to_owned(),
                                Player {
                                    name: "Player 1".to_owned(),
                                    id: "player-1".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::from_iter([Mon {
                                        physical_appearance: MonPhysicalAppearance {
                                            name: "Squirtle".to_owned(),
                                            species: "Squirtle".to_owned(),
                                            gender: "M".to_owned(),
                                            ..Default::default()
                                        },
                                        battle_appearances: VecDeque::from_iter([
                                            MonBattleAppearanceWithRecovery::Active {
                                                primary_battle_appearance: MonBattleAppearance {
                                                    level: 5.into(),
                                                    health: (100, 100).into(),
                                                    status: String::default().into(),
                                                    moves: DiscoveryRequiredSet::from_known([
                                                        "Pound".to_owned()
                                                    ]),
                                                    ..Default::default()
                                                },
                                                battle_appearance_up_to_last_switch_out:
                                                    MonBattleAppearance::default(),
                                                battle_appearance_from_last_switch_in:
                                                    MonBattleAppearance {
                                                        level: 5.into(),
                                                        health: (100, 100).into(),
                                                        status: String::default().into(),
                                                        moves: DiscoveryRequiredSet::from_known([
                                                            "Pound".to_owned()
                                                        ]),
                                                        ..Default::default()
                                                    },
                                            }
                                        ]),
                                        fainted: false,
                                        volatile_data: MonVolatileData::default(),
                                    }]),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::from_iter([Some(MonBattleAppearanceReference {
                                player: "player-1".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            })]),
                            ..Default::default()
                        },
                        Side {
                            name: "Side 2".to_owned(),
                            id: 1,
                            players: BTreeMap::from_iter([(
                                "player-2".to_owned(),
                                Player {
                                    name: "Player 2".to_owned(),
                                    id: "player-2".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::from_iter([Mon {
                                        physical_appearance: MonPhysicalAppearance {
                                            name: "Charmander".to_owned(),
                                            species: "Charmander".to_owned(),
                                            gender: "M".to_owned(),
                                            ..Default::default()
                                        },
                                        battle_appearances: VecDeque::from_iter([
                                            MonBattleAppearanceWithRecovery::Active {
                                                primary_battle_appearance: MonBattleAppearance {
                                                    level: 5.into(),
                                                    health: (75, 100).into(),
                                                    status: String::default().into(),
                                                    ..Default::default()
                                                },
                                                battle_appearance_up_to_last_switch_out:
                                                    MonBattleAppearance::default(),
                                                battle_appearance_from_last_switch_in:
                                                    MonBattleAppearance {
                                                        level: 5.into(),
                                                        health: (75, 100).into(),
                                                        status: String::default().into(),
                                                        ..Default::default()
                                                    },
                                            }
                                        ]),
                                        fainted: false,
                                        volatile_data: MonVolatileData::default(),
                                    }]),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::from_iter([Some(MonBattleAppearanceReference {
                                player: "player-2".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            })]),
                            ..Default::default()
                        }
                    ]),
                    max_side_length: 2,
                    ..Default::default()
                },
                ui_log: Vec::from_iter([
                    Vec::from_iter([
                        ui::UiLogEntry::Switch {
                            switch_type: "switch".to_owned(),
                            player: "player-1".to_owned(),
                            mon: 0,
                            into_position: ui::FieldPosition {
                                side: 0,
                                position: 0,
                            }
                        },
                        ui::UiLogEntry::Switch {
                            switch_type: "switch".to_owned(),
                            player: "player-2".to_owned(),
                            mon: 0,
                            into_position: ui::FieldPosition {
                                side: 1,
                                position: 0,
                            }
                        }
                    ]),
                    Vec::from_iter([
                        ui::UiLogEntry::Move {
                            name: "Pound".to_owned(),
                            mon: ui::Mon::Active(ui::FieldPosition {
                                side: 0,
                                position: 0,
                            }),
                            target: Some(ui::MoveTarget::Single(ui::Mon::Active(
                                ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                }
                            ))),
                            animate: true,
                        },
                        ui::UiLogEntry::Damage {
                            health: (75, 100),
                            effect: ui::EffectData {
                                target: Some(ui::Mon::Active(ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                })),
                                additional: HashMap::from_iter([(
                                    "health".to_owned(),
                                    "75/100".to_owned()
                                )]),
                                ..Default::default()
                            }
                        }
                    ])
                ]),
            }
        );
    }

    #[test]
    fn records_new_mon_revealed_from_switch() {
        let log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "time|time:0",
            "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:75/100",
            "residual",
            "turn|turn:2",
            "time|time:0",
            "switch|player:player-1|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:5|gender:M",
            "residual",
            "turn|turn:3"
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state,
            BattleState {
                phase: BattlePhase::Battle,
                turn: 3,
                battle_type: "singles".to_owned(),
                field: Field {
                    sides: Vec::from_iter([
                        Side {
                            name: "Side 1".to_owned(),
                            id: 0,
                            players: BTreeMap::from_iter([(
                                "player-1".to_owned(),
                                Player {
                                    name: "Player 1".to_owned(),
                                    id: "player-1".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::from_iter([
                                        Mon {
                                            physical_appearance: MonPhysicalAppearance {
                                                name: "Squirtle".to_owned(),
                                                species: "Squirtle".to_owned(),
                                                gender: "M".to_owned(),
                                                ..Default::default()
                                            },
                                            battle_appearances: VecDeque::from_iter([
                                                MonBattleAppearanceWithRecovery::Inactive(
                                                    MonBattleAppearance {
                                                        level: 5.into(),
                                                        health: (100, 100).into(),
                                                        status: String::default().into(),
                                                        moves: DiscoveryRequiredSet::from_known([
                                                            "Pound".to_owned()
                                                        ]),
                                                        ..Default::default()
                                                    }
                                                ),
                                            ]),
                                            fainted: false,
                                            volatile_data: MonVolatileData::default(),
                                        },
                                        Mon {
                                            physical_appearance: MonPhysicalAppearance {
                                                name: "Bulbasaur".to_owned(),
                                                species: "Bulbasaur".to_owned(),
                                                gender: "M".to_owned(),
                                                ..Default::default()
                                            },
                                            battle_appearances: VecDeque::from_iter([
                                                MonBattleAppearanceWithRecovery::Active {
                                                    primary_battle_appearance:
                                                        MonBattleAppearance {
                                                            level: 5.into(),
                                                            health: (50, 100).into(),
                                                            status: String::default().into(),
                                                            ..Default::default()
                                                        },
                                                    battle_appearance_up_to_last_switch_out:
                                                        MonBattleAppearance::default(),
                                                    battle_appearance_from_last_switch_in:
                                                        MonBattleAppearance {
                                                            level: 5.into(),
                                                            health: (50, 100).into(),
                                                            status: String::default().into(),
                                                            ..Default::default()
                                                        },
                                                },
                                            ]),
                                            fainted: false,
                                            volatile_data: MonVolatileData::default(),
                                        }
                                    ]),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::from_iter([Some(MonBattleAppearanceReference {
                                player: "player-1".to_owned(),
                                mon_index: 1,
                                battle_appearance_index: 0,
                            })]),
                            ..Default::default()
                        },
                        Side {
                            name: "Side 2".to_owned(),
                            id: 1,
                            players: BTreeMap::from_iter([(
                                "player-2".to_owned(),
                                Player {
                                    name: "Player 2".to_owned(),
                                    id: "player-2".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::from_iter([Mon {
                                        physical_appearance: MonPhysicalAppearance {
                                            name: "Charmander".to_owned(),
                                            species: "Charmander".to_owned(),
                                            gender: "M".to_owned(),
                                            ..Default::default()
                                        },
                                        battle_appearances: VecDeque::from_iter([
                                            MonBattleAppearanceWithRecovery::Active {
                                                primary_battle_appearance: MonBattleAppearance {
                                                    level: 5.into(),
                                                    health: (75, 100).into(),
                                                    status: String::default().into(),
                                                    ..Default::default()
                                                },
                                                battle_appearance_up_to_last_switch_out:
                                                    MonBattleAppearance::default(),
                                                battle_appearance_from_last_switch_in:
                                                    MonBattleAppearance {
                                                        level: 5.into(),
                                                        health: (75, 100).into(),
                                                        status: String::default().into(),
                                                        ..Default::default()
                                                    },
                                            }
                                        ]),
                                        fainted: false,
                                        volatile_data: MonVolatileData::default(),
                                    }]),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::from_iter([Some(MonBattleAppearanceReference {
                                player: "player-2".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            })]),
                            ..Default::default()
                        }
                    ]),
                    max_side_length: 2,
                    ..Default::default()
                },
                ui_log: Vec::from_iter([
                    Vec::from_iter([
                        ui::UiLogEntry::Switch {
                            switch_type: "switch".to_owned(),
                            player: "player-1".to_owned(),
                            mon: 0,
                            into_position: ui::FieldPosition {
                                side: 0,
                                position: 0,
                            }
                        },
                        ui::UiLogEntry::Switch {
                            switch_type: "switch".to_owned(),
                            player: "player-2".to_owned(),
                            mon: 0,
                            into_position: ui::FieldPosition {
                                side: 1,
                                position: 0,
                            }
                        }
                    ]),
                    Vec::from_iter([
                        ui::UiLogEntry::Move {
                            name: "Pound".to_owned(),
                            mon: ui::Mon::Active(ui::FieldPosition {
                                side: 0,
                                position: 0,
                            }),
                            target: Some(ui::MoveTarget::Single(ui::Mon::Active(
                                ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                }
                            ))),
                            animate: true,
                        },
                        ui::UiLogEntry::Damage {
                            health: (75, 100),
                            effect: ui::EffectData {
                                target: Some(ui::Mon::Active(ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                })),
                                additional: HashMap::from_iter([(
                                    "health".to_owned(),
                                    "75/100".to_owned()
                                )]),
                                ..Default::default()
                            }
                        }
                    ]),
                    Vec::from_iter([ui::UiLogEntry::Switch {
                        switch_type: "switch".to_owned(),
                        player: "player-1".to_owned(),
                        mon: 1,
                        into_position: ui::FieldPosition {
                            side: 0,
                            position: 0,
                        }
                    }])
                ]),
            }
        );
    }

    #[test]
    fn uses_old_mon_reappeared_from_switch() {
        let log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "time|time:0",
            "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:75/100",
            "residual",
            "turn|turn:2",
            "time|time:0",
            "switch|player:player-1|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:5|gender:M",
            "residual",
            "turn|turn:3",
            "time|time:0",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "residual",
            "turn|turn:4",
            "time|time:0",
            "move|mon:Squirtle,player-1,1|name:Water Gun|target:Charmander,player-2,1",
            "supereffective|mon:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:1/100",
            "residual",
            "turn|turn:5",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state,
            BattleState {
                phase: BattlePhase::Battle,
                turn: 5,
                battle_type: "singles".to_owned(),
                field: Field {
                    sides: Vec::from_iter([
                        Side {
                            name: "Side 1".to_owned(),
                            id: 0,
                            players: BTreeMap::from_iter([(
                                "player-1".to_owned(),
                                Player {
                                    name: "Player 1".to_owned(),
                                    id: "player-1".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::from_iter([
                                        Mon {
                                            physical_appearance: MonPhysicalAppearance {
                                                name: "Squirtle".to_owned(),
                                                species: "Squirtle".to_owned(),
                                                gender: "M".to_owned(),
                                                ..Default::default()
                                            },
                                            battle_appearances: VecDeque::from_iter([
                                                MonBattleAppearanceWithRecovery::Active {
                                                    primary_battle_appearance:
                                                        MonBattleAppearance {
                                                            level: 5.into(),
                                                            health: (100, 100).into(),
                                                            status: String::default().into(),
                                                            moves: DiscoveryRequiredSet::from_known(
                                                                [
                                                                    "Pound".to_owned(),
                                                                    "Water Gun".to_owned()
                                                                ]
                                                            ),
                                                            ..Default::default()
                                                        },
                                                    battle_appearance_up_to_last_switch_out:
                                                        MonBattleAppearance {
                                                            level: 5.into(),
                                                            health: (100, 100).into(),
                                                            status: String::default().into(),
                                                            moves: DiscoveryRequiredSet::from_known(
                                                                ["Pound".to_owned(),]
                                                            ),
                                                            ..Default::default()
                                                        },
                                                    battle_appearance_from_last_switch_in:
                                                        MonBattleAppearance {
                                                            level: 5.into(),
                                                            health: (100, 100).into(),
                                                            status: String::default().into(),
                                                            moves: DiscoveryRequiredSet::from_known(
                                                                ["Water Gun".to_owned()]
                                                            ),
                                                            ..Default::default()
                                                        },
                                                },
                                            ]),
                                            fainted: false,
                                            volatile_data: MonVolatileData::default(),
                                        },
                                        Mon {
                                            physical_appearance: MonPhysicalAppearance {
                                                name: "Bulbasaur".to_owned(),
                                                species: "Bulbasaur".to_owned(),
                                                gender: "M".to_owned(),
                                                ..Default::default()
                                            },
                                            battle_appearances: VecDeque::from_iter([
                                                MonBattleAppearanceWithRecovery::Inactive(
                                                    MonBattleAppearance {
                                                        level: 5.into(),
                                                        health: (50, 100).into(),
                                                        status: String::default().into(),
                                                        ..Default::default()
                                                    }
                                                ),
                                            ]),
                                            fainted: false,
                                            volatile_data: MonVolatileData::default(),
                                        }
                                    ]),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::from_iter([Some(MonBattleAppearanceReference {
                                player: "player-1".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            })]),
                            ..Default::default()
                        },
                        Side {
                            name: "Side 2".to_owned(),
                            id: 1,
                            players: BTreeMap::from_iter([(
                                "player-2".to_owned(),
                                Player {
                                    name: "Player 2".to_owned(),
                                    id: "player-2".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::from_iter([Mon {
                                        physical_appearance: MonPhysicalAppearance {
                                            name: "Charmander".to_owned(),
                                            species: "Charmander".to_owned(),
                                            gender: "M".to_owned(),
                                            ..Default::default()
                                        },
                                        battle_appearances: VecDeque::from_iter([
                                            MonBattleAppearanceWithRecovery::Active {
                                                primary_battle_appearance: MonBattleAppearance {
                                                    level: 5.into(),
                                                    health: (1, 100).into(),
                                                    status: String::default().into(),
                                                    ..Default::default()
                                                },
                                                battle_appearance_up_to_last_switch_out:
                                                    MonBattleAppearance::default(),
                                                battle_appearance_from_last_switch_in:
                                                    MonBattleAppearance {
                                                        level: 5.into(),
                                                        health: (1, 100).into(),
                                                        status: String::default().into(),
                                                        ..Default::default()
                                                    },
                                            }
                                        ]),
                                        fainted: false,
                                        volatile_data: MonVolatileData::default(),
                                    }]),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::from_iter([Some(MonBattleAppearanceReference {
                                player: "player-2".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            })]),
                            ..Default::default()
                        }
                    ]),
                    max_side_length: 2,
                    ..Default::default()
                },
                ui_log: Vec::from_iter([
                    Vec::from_iter([
                        ui::UiLogEntry::Switch {
                            switch_type: "switch".to_owned(),
                            player: "player-1".to_owned(),
                            mon: 0,
                            into_position: ui::FieldPosition {
                                side: 0,
                                position: 0,
                            }
                        },
                        ui::UiLogEntry::Switch {
                            switch_type: "switch".to_owned(),
                            player: "player-2".to_owned(),
                            mon: 0,
                            into_position: ui::FieldPosition {
                                side: 1,
                                position: 0,
                            }
                        }
                    ]),
                    Vec::from_iter([
                        ui::UiLogEntry::Move {
                            name: "Pound".to_owned(),
                            mon: ui::Mon::Active(ui::FieldPosition {
                                side: 0,
                                position: 0,
                            }),
                            target: Some(ui::MoveTarget::Single(ui::Mon::Active(
                                ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                }
                            ))),
                            animate: true,
                        },
                        ui::UiLogEntry::Damage {
                            health: (75, 100),
                            effect: ui::EffectData {
                                target: Some(ui::Mon::Active(ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                })),
                                additional: HashMap::from_iter([(
                                    "health".to_owned(),
                                    "75/100".to_owned()
                                )]),
                                ..Default::default()
                            }
                        }
                    ]),
                    Vec::from_iter([ui::UiLogEntry::Switch {
                        switch_type: "switch".to_owned(),
                        player: "player-1".to_owned(),
                        mon: 1,
                        into_position: ui::FieldPosition {
                            side: 0,
                            position: 0,
                        }
                    }]),
                    Vec::from_iter([ui::UiLogEntry::Switch {
                        switch_type: "switch".to_owned(),
                        player: "player-1".to_owned(),
                        mon: 0,
                        into_position: ui::FieldPosition {
                            side: 0,
                            position: 0,
                        }
                    }]),
                    Vec::from_iter([
                        ui::UiLogEntry::Move {
                            name: "Water Gun".to_owned(),
                            mon: ui::Mon::Active(ui::FieldPosition {
                                side: 0,
                                position: 0
                            }),
                            target: Some(ui::MoveTarget::Single(ui::Mon::Active(
                                ui::FieldPosition {
                                    side: 1,
                                    position: 0
                                }
                            ))),
                            animate: true,
                        },
                        ui::UiLogEntry::Effect {
                            title: "supereffective".to_owned(),
                            effect: ui::EffectData {
                                target: Some(ui::Mon::Active(ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                })),
                                ..Default::default()
                            }
                        },
                        ui::UiLogEntry::Damage {
                            health: (1, 100),
                            effect: ui::EffectData {
                                target: Some(ui::Mon::Active(ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                })),
                                additional: HashMap::from_iter([(
                                    "health".to_owned(),
                                    "1/100".to_owned()
                                )]),
                                ..Default::default()
                            }
                        }
                    ]),
                ]),
            }
        );
    }

    #[test]
    fn updates_ongoing_state_after_complete_turn() {
        let mut log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "time|time:0",
            "move|mon:Squirtle,player-1,1|name:Pound|target:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:75/100",
            "residual",
            "turn|turn:2",
            "time|time:0",
            "switch|player:player-1|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:5|gender:M",
            "residual",
            "turn|turn:3",
            "time|time:0",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "residual",
            "turn|turn:4",
            "time|time:0",
            "move|mon:Squirtle,player-1,1|name:Water Gun|target:Charmander,player-2,1",
            "supereffective|mon:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:1/100",
            "residual",
            "turn|turn:5",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();

        log.add(log.len(), "time|time:0").unwrap();
        log.add(
            log.len(),
            "move|mon:Charmander,player-2,1|name:Scratch|target:Squirtle,player-1,1",
        )
        .unwrap();
        log.add(log.len(), "damage|mon:Charmander,player-2,1|health:80/100")
            .unwrap();
        log.add(log.len(), "residual").unwrap();

        // Turn has not finished, so state is not really updated.
        let state = alter_battle_state(state, &log).unwrap();
        assert!(
            !state.field.sides[1].players.get("player-2").unwrap().mons[0].battle_appearances[0]
                .primary()
                .moves
                .known()
                .contains("Scratch")
        );

        // Finish the turn, so the state can be updated.
        log.add(log.len(), "turn|turn:6").unwrap();
        let state = alter_battle_state(state, &log).unwrap();
        assert!(
            state.field.sides[1].players.get("player-2").unwrap().mons[0].battle_appearances[0]
                .primary()
                .moves
                .known()
                .contains("Scratch")
        );
    }

    #[test]
    fn records_fainted_mon() {
        let log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "time|time:0",
            "move|mon:Squirtle,player-1,1|name:Water Gun|target:Charmander,player-2,1",
            "supereffective|mon:Charmander,player-2,1",
            "damage|mon:Charmander,player-2,1|health:0",
            "faint|mon:Charmander,player-2,1",
            "residual",
            "turn|turn:2",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state,
            BattleState {
                phase: BattlePhase::Battle,
                turn: 2,
                battle_type: "singles".to_owned(),
                field: Field {
                    sides: Vec::from_iter([
                        Side {
                            name: "Side 1".to_owned(),
                            id: 0,
                            players: BTreeMap::from_iter([(
                                "player-1".to_owned(),
                                Player {
                                    name: "Player 1".to_owned(),
                                    id: "player-1".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::from_iter([Mon {
                                        physical_appearance: MonPhysicalAppearance {
                                            name: "Squirtle".to_owned(),
                                            species: "Squirtle".to_owned(),
                                            gender: "M".to_owned(),
                                            ..Default::default()
                                        },
                                        battle_appearances: VecDeque::from_iter([
                                            MonBattleAppearanceWithRecovery::Active {
                                                primary_battle_appearance: MonBattleAppearance {
                                                    level: 5.into(),
                                                    health: (100, 100).into(),
                                                    status: String::default().into(),
                                                    moves: DiscoveryRequiredSet::from_known([
                                                        "Water Gun".to_owned()
                                                    ]),
                                                    ..Default::default()
                                                },
                                                battle_appearance_up_to_last_switch_out:
                                                    MonBattleAppearance::default(),
                                                battle_appearance_from_last_switch_in:
                                                    MonBattleAppearance {
                                                        level: 5.into(),
                                                        health: (100, 100).into(),
                                                        status: String::default().into(),
                                                        moves: DiscoveryRequiredSet::from_known([
                                                            "Water Gun".to_owned()
                                                        ]),
                                                        ..Default::default()
                                                    },
                                            }
                                        ]),
                                        fainted: false,
                                        volatile_data: MonVolatileData::default(),
                                    }]),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::from_iter([Some(MonBattleAppearanceReference {
                                player: "player-1".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            })]),
                            ..Default::default()
                        },
                        Side {
                            name: "Side 2".to_owned(),
                            id: 1,
                            players: BTreeMap::from_iter([(
                                "player-2".to_owned(),
                                Player {
                                    name: "Player 2".to_owned(),
                                    id: "player-2".to_owned(),
                                    position: 0,
                                    team_size: 3,
                                    mons: Vec::from_iter([Mon {
                                        physical_appearance: MonPhysicalAppearance {
                                            name: "Charmander".to_owned(),
                                            species: "Charmander".to_owned(),
                                            gender: "M".to_owned(),
                                            ..Default::default()
                                        },
                                        battle_appearances: VecDeque::from_iter([
                                            MonBattleAppearanceWithRecovery::Inactive(
                                                MonBattleAppearance {
                                                    level: 5.into(),
                                                    health: (0, 1).into(),
                                                    status: String::default().into(),
                                                    ..Default::default()
                                                }
                                            ),
                                        ]),
                                        fainted: true,
                                        volatile_data: MonVolatileData::default(),
                                    }]),
                                    ..Default::default()
                                }
                            )]),
                            active: Vec::from_iter([Some(MonBattleAppearanceReference {
                                player: "player-2".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            })]),
                            ..Default::default()
                        }
                    ]),
                    max_side_length: 2,
                    ..Default::default()
                },
                ui_log: Vec::from_iter([
                    Vec::from_iter([
                        ui::UiLogEntry::Switch {
                            switch_type: "switch".to_owned(),
                            player: "player-1".to_owned(),
                            mon: 0,
                            into_position: ui::FieldPosition {
                                side: 0,
                                position: 0,
                            }
                        },
                        ui::UiLogEntry::Switch {
                            switch_type: "switch".to_owned(),
                            player: "player-2".to_owned(),
                            mon: 0,
                            into_position: ui::FieldPosition {
                                side: 1,
                                position: 0,
                            }
                        }
                    ]),
                    Vec::from_iter([
                        ui::UiLogEntry::Move {
                            name: "Water Gun".to_owned(),
                            mon: ui::Mon::Active(ui::FieldPosition {
                                side: 0,
                                position: 0,
                            }),
                            target: Some(ui::MoveTarget::Single(ui::Mon::Active(
                                ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                }
                            ))),
                            animate: true,
                        },
                        ui::UiLogEntry::Effect {
                            title: "supereffective".to_owned(),
                            effect: ui::EffectData {
                                target: Some(ui::Mon::Active(ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                })),
                                ..Default::default()
                            }
                        },
                        ui::UiLogEntry::Damage {
                            health: (0, 1),
                            effect: ui::EffectData {
                                target: Some(ui::Mon::Active(ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                })),
                                additional: HashMap::from_iter([(
                                    "health".to_owned(),
                                    "0".to_owned()
                                )]),
                                ..Default::default()
                            }
                        },
                        ui::UiLogEntry::Faint {
                            effect: ui::EffectData {
                                target: Some(ui::Mon::Active(ui::FieldPosition {
                                    side: 1,
                                    position: 0,
                                })),
                                ..Default::default()
                            }
                        }
                    ])
                ]),
            }
        );
    }

    #[test]
    fn keeps_track_of_multiple_battle_appearances_due_to_single_illusion_user_with_unique_level() {
        // First, we just see all Mons.
        let log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
            "residual",
            "turn|turn:2",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:6|gender:M",
            "residual",
            "turn|turn:3",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 6.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance::default(),
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 6.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                        }
                    ]),
                    ..Default::default()
                }
            ])
        );

        // Second, the illusion is revealed.
        //
        // Since we hit the team size, the level 6 Charmander is merged with the level 5 one, but
        // its battle appearance is quickly removed because it is empty.
        let log = Log::try_from(&[
            "damage|mon:Charmander,player-2,1|health:75/100",
            "replace|player:player-2|position:1|name:Zoroark|health:75/100|species:Zoroark|level:6|gender:F",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "residual",
            "turn|turn:4",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "F".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 6.into(),
                                health: (75, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance::default(),
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 6.into(),
                                health: (75, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                ..Default::default()
                            },
                        }
                    ]),
                    ..Default::default()
                }
            ])
        );

        // Third, we test different information being revealed by the illusion at different times.
        let log = Log::try_from(&[
            "move|mon:Zoroark,player-2,1|name:Bite",
            "turn|turn:5",
            "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
            "move|mon:Bulbasaur,player-2,1|name:Absorb",
            "turn|turn:6",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:7",
            "move|mon:Charmander,player-2,1|name:Growl",
            "turn|turn:8",
            "switch|player:player-2|position:1|name:Bulbasaur|health:75/100|species:Bulbasaur|level:6|gender:M",
            "turn|turn:9",
            "move|mon:Bulbasaur,player-2,1|name:Dark Pulse",
            "turn|turn:10",
            "damage|mon:Bulbasaur,player-2,1|health:50/100",
            "replace|player:player-2|position:1|name:Zoroark|health:50/100|species:Zoroark|level:6|gender:F",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "turn|turn:11",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Growl".to_owned()]),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Absorb".to_owned()]),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "F".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 6.into(),
                                health: (50, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                moves: DiscoveryRequiredSet::from_known([
                                    "Bite".to_owned(),
                                    "Dark Pulse".to_owned()
                                ]),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance {
                                level: 6.into(),
                                health: (75, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                moves: DiscoveryRequiredSet::from_known(["Bite".to_owned()]),
                                ..Default::default()
                            },
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 6.into(),
                                health: (50, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                moves: DiscoveryRequiredSet::from_known(["Dark Pulse".to_owned()]),
                                ..Default::default()
                            },
                        }
                    ]),
                    ..Default::default()
                }
            ])
        );

        // Fourth, show that an unrevealed illusion causes a lingering battle appearance.
        let log = Log::try_from(&[
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:12",
            "switch|player:player-2|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:6|gender:M",
            "turn|turn:13",
            "move|mon:Bulbasaur,player-2,1|name:Crunch",
            "turn|turn:14",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:15",
            "switch|player:player-2|position:1|name:Bulbasaur|health:50/100|species:Bulbasaur|level:6|gender:M",
            "turn|turn:16",
            "damage|mon:Bulbasaur,player-2,1|health:25/100",
            "replace|player:player-2|position:1|name:Zoroark|health:25/100|species:Zoroark|level:6|gender:F",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "turn|turn:17",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Growl".to_owned()]),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Absorb".to_owned()]),
                            ..Default::default()
                        }),
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 6.into(),
                            health: (50, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Crunch".to_owned()]),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "F".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 6.into(),
                                health: (25, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                moves: DiscoveryRequiredSet::from_known([
                                    "Bite".to_owned(),
                                    "Dark Pulse".to_owned()
                                ]),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance {
                                level: 6.into(),
                                health: (50, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                moves: DiscoveryRequiredSet::from_known([
                                    "Bite".to_owned(),
                                    "Dark Pulse".to_owned()
                                ]),
                                ..Default::default()
                            },
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 6.into(),
                                health: (25, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                ..Default::default()
                            },
                        }
                    ]),
                    ..Default::default()
                }
            ])
        );

        // Fifth, show that as we add more battle appearances to a single Mon, we eventually limit
        // and merge them.
        let log = Log::try_from(&[
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:18",
            "switch|player:player-2|position:1|name:Bulbasaur|health:25/100|species:Bulbasaur|level:6|gender:M",
            "turn|turn:19",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:20",
            "switch|player:player-2|position:1|name:Bulbasaur|health:12/100|species:Bulbasaur|level:6|gender:M",
            "turn|turn:21",
            "move|mon:Bulbasaur,player-2,1|name:Bite",
            "turn|turn:22",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Growl".to_owned()]),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 6.into(),
                            health: (50, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Crunch".to_owned()]),
                            ..Default::default()
                        }),
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 6.into(),
                            health: (25, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        }),
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 6.into(),
                                health: (12, 100).into(),
                                status: String::default().into(),
                                moves: DiscoveryRequiredSet::new(
                                    ["Bite".to_owned()],
                                    ["Absorb".to_owned()],
                                ),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance {
                                level: DiscoveryRequired::PossibleValues([5].into()),
                                health: DiscoveryRequired::PossibleValues([(100, 100)].into()),
                                status: DiscoveryRequired::PossibleValues(
                                    [String::default()].into()
                                ),
                                moves: DiscoveryRequiredSet::from_possible_values([
                                    "Absorb".to_owned()
                                ]),
                                ..Default::default()
                            },
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 6.into(),
                                health: (12, 100).into(),
                                status: String::default().into(),
                                moves: DiscoveryRequiredSet::from_known(["Bite".to_owned()]),
                                ..Default::default()
                            },
                        }
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "F".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 6.into(),
                            health: (25, 100).into(),
                            status: String::default().into(),
                            ability: "Illusion".to_owned().into(),
                            moves: DiscoveryRequiredSet::from_known([
                                "Bite".to_owned(),
                                "Dark Pulse".to_owned()
                            ]),
                            ..Default::default()
                        }),
                    ]),
                    ..Default::default()
                }
            ])
        );

        // Sixth, show that the battle appearance with possible values gets matched if we expose the
        // illusion and bring back the real Bulbasaur.
        let log = Log::try_from(&[
            "replace|player:player-2|position:1|name:Zoroark|health:12/100|species:Zoroark|level:6|gender:F",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "turn|turn:23",
            "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
            "turn|turn:24",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Growl".to_owned()]),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 6.into(),
                            health: (50, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Crunch".to_owned()]),
                            ..Default::default()
                        }),
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 6.into(),
                            health: (25, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        }),
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                moves: DiscoveryRequiredSet::from_possible_values([
                                    "Absorb".to_owned()
                                ]),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance {
                                level: DiscoveryRequired::PossibleValues([5].into()),
                                health: DiscoveryRequired::PossibleValues([(100, 100)].into()),
                                status: DiscoveryRequired::PossibleValues(
                                    [String::default()].into()
                                ),
                                moves: DiscoveryRequiredSet::from_possible_values([
                                    "Absorb".to_owned()
                                ]),
                                ..Default::default()
                            },
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                        }
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "F".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 6.into(),
                            health: (12, 100).into(),
                            status: String::default().into(),
                            ability: "Illusion".to_owned().into(),
                            moves: DiscoveryRequiredSet::from_known([
                                "Bite".to_owned(),
                                "Dark Pulse".to_owned()
                            ]),
                            ..Default::default()
                        }),
                    ]),
                    ..Default::default()
                }
            ])
        );

        // Seventh, faint the illusion without showing it. Then, bring the real Mon back in, and
        // show that the fainted status is moved to the illusion user.
        let log = Log::try_from(&[
            "switch|player:player-2|position:1|name:Bulbasaur|health:12/100|species:Bulbasaur|level:6|gender:M",
            "turn|turn:25",
            "faint|mon:Bulbasaur,player-2,1",
            "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
            "turn|turn:26",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Growl".to_owned()]),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 6.into(),
                            health: (25, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        }),
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                moves: DiscoveryRequiredSet::from_possible_values([
                                    "Absorb".to_owned()
                                ]),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                moves: DiscoveryRequiredSet::from_possible_values([
                                    "Absorb".to_owned()
                                ]),
                                ..Default::default()
                            },
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                        },
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 6.into(),
                            health: (12, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_possible_values(
                                ["Crunch".to_owned()]
                            ),
                            ..Default::default()
                        }),
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "F".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 6.into(),
                            health: (12, 100).into(),
                            status: String::default().into(),
                            ability: "Illusion".to_owned().into(),
                            moves: DiscoveryRequiredSet::from_known([
                                "Bite".to_owned(),
                                "Dark Pulse".to_owned()
                            ]),
                            ..Default::default()
                        }),
                    ]),
                    fainted: true,
                    ..Default::default()
                }
            ])
        );
    }

    #[test]
    fn keeps_track_of_multiple_battle_appearances_due_to_single_illusion_user_with_same_level() {
        // First, we just see all Mons.
        let log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
            "residual",
            "turn|turn:2",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "residual",
            "turn|turn:3",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                        },
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
            ])
        );

        // Second, reveal illusion user as a new Mon.
        let log = Log::try_from(&[
            "damage|mon:Charmander,player-2,1|health:75/100",
            "replace|player:player-2|position:1|name:Zoroark|health:75/100|species:Zoroark|level:5|gender:M",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "turn|turn:4",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 5.into(),
                                health: (75, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance::default(),
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 5.into(),
                                health: (75, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                ..Default::default()
                            },
                        },
                    ]),
                    ..Default::default()
                },
            ])
        );

        // Third, heal the illusion user so that it is unified back to the real Mon. Then alternate
        // between the two, which should be trackable.
        let log = Log::try_from(&[
            "heal|mon:Charmander,player-2,1|health:100/100",
            "turn|turn:5",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "move|mon:Charmander,player-2,1|name:Growl",
            "turn|turn:6",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "move|mon:Charmander,player-2,1|name:Bite",
            "turn|turn:7",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "move|mon:Charmander,player-2,1|name:Scratch",
            "turn|turn:8",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "move|mon:Charmander,player-2,1|name:Dark Pulse",
            "turn|turn:9",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known([
                                "Growl".to_owned(),
                                "Scratch".to_owned(),
                            ]),
                            ..Default::default()
                        }),
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                moves: DiscoveryRequiredSet::from_known([
                                    "Bite".to_owned(),
                                    "Dark Pulse".to_owned(),
                                ]),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                moves: DiscoveryRequiredSet::from_known(["Bite".to_owned()]),
                                ..Default::default()
                            },
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                moves: DiscoveryRequiredSet::from_known(["Dark Pulse".to_owned()]),
                                ..Default::default()
                            },
                        }
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ability: "Illusion".to_owned().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
            ])
        );

        // Fourth, reveal the illusion. Some of the data stays on the original Mon, since it is
        // technically not known that it was an illusion.
        let log = Log::try_from(&[
            "replace|player:player-2|position:1|name:Zoroark|health:100/100|species:Zoroark|level:5|gender:M",
            "turn|turn:10",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known([
                                "Growl".to_owned(),
                                "Scratch".to_owned(),
                            ]),
                            ..Default::default()
                        }),
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            moves: DiscoveryRequiredSet::from_known(["Bite".to_owned()]),
                            ..Default::default()
                        }),
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                moves: DiscoveryRequiredSet::from_known(["Dark Pulse".to_owned()]),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                ..Default::default()
                            },
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                moves: DiscoveryRequiredSet::from_known(["Dark Pulse".to_owned()]),
                                ..Default::default()
                            },
                        },
                    ]),
                    ..Default::default()
                },
            ])
        );
    }

    #[test]
    fn illusion_user_faints_before_being_revealed() {
        let log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:3",
            "teamsize|player:player-2|size:3",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
            "turn|turn:2",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:3",
            "faint|mon:Charmander,player-2,1",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:4",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        }),
                    ]),
                    fainted: true,
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance::default(),
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                        },
                    ]),
                    ..Default::default()
                },
            ])
        );
    }

    #[test]
    fn corrects_fainted_illusion_user_with_multiple_illusion_users() {
        let log = Log::try_from(&[
            "info|battletype:Singles",
            "side|id:0|name:Side 1",
            "side|id:1|name:Side 2",
            "maxsidelength|length:2",
            "player|id:player-1|name:Player 1|side:0|position:0",
            "player|id:player-2|name:Player 2|side:1|position:0",
            "teamsize|player:player-1|size:4",
            "teamsize|player:player-2|size:4",
            "battlestart",
            "switch|player:player-1|position:1|name:Squirtle|health:100/100|species:Squirtle|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:1",
            "switch|player:player-2|position:1|name:Bulbasaur|health:100/100|species:Bulbasaur|level:5|gender:M",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "replace|player:player-2|position:1|name:Zorua|health:100/100|species:Zorua|level:5|gender:M",
            "end|mon:Zorua,player-2,1|ability:Illusion",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "replace|player:player-2|position:1|name:Zoroark|health:100/100|species:Zoroark|level:5|gender:M",
            "end|mon:Zoroark,player-2,1|ability:Illusion",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "damage|mon:Charmander,player-2,1|health:0",
            "faint|mon:Charmander,player-2,1",
            "switch|player:player-2|position:1|name:Charmander|health:100/100|species:Charmander|level:5|gender:M",
            "turn|turn:2",
        ])
        .unwrap();

        let state = BattleState::default();
        let state = alter_battle_state(state, &log).unwrap();

        // At first, Zorua is guessed to have fainted when Charmander reappears.
        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (0, 1).into(),
                            status: String::default().into(),
                            ..Default::default()
                        }),
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance::default(),
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                        },
                    ]),
                    fainted: false,
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zorua".to_owned(),
                        species: "Zorua".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ability: "Illusion".to_owned().into(),
                            ..Default::default()
                        }),
                    ]),
                    fainted: true,
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ability: "Illusion".to_owned().into(),
                            ..Default::default()
                        }),
                    ]),
                    ..Default::default()
                },
            ])
        );

        // Wait! Zorua is here! So Zoroark must have fainted.
        let log = Log::try_from(&[
            "damage|mon:Charmander,player-2,1|health:50/100",
            "replace|player:player-2|position:1|name:Zorua|health:50/100|species:Zorua|level:5|gender:M",
            "turn|turn:3",
        ])
        .unwrap();

        let state = alter_battle_state(state, &log).unwrap();

        pretty_assertions::assert_eq!(
            state.field.sides[1].players["player-2"].mons,
            Vec::from_iter([
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Charmander".to_owned(),
                        species: "Charmander".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (0, 1).into(),
                            status: String::default().into(),
                            ..Default::default()
                        }),
                    ]),
                    fainted: false,
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Bulbasaur".to_owned(),
                        species: "Bulbasaur".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ..Default::default()
                        })
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zorua".to_owned(),
                        species: "Zorua".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Active {
                            primary_battle_appearance: MonBattleAppearance {
                                level: 5.into(),
                                health: (50, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                ..Default::default()
                            },
                            battle_appearance_up_to_last_switch_out: MonBattleAppearance {
                                level: 5.into(),
                                health: (100, 100).into(),
                                status: String::default().into(),
                                ability: "Illusion".to_owned().into(),
                                ..Default::default()
                            },
                            battle_appearance_from_last_switch_in: MonBattleAppearance {
                                level: 5.into(),
                                health: (50, 100).into(),
                                status: String::default().into(),
                                ..Default::default()
                            },
                        },
                    ]),
                    ..Default::default()
                },
                Mon {
                    physical_appearance: MonPhysicalAppearance {
                        name: "Zoroark".to_owned(),
                        species: "Zoroark".to_owned(),
                        gender: "M".to_owned(),
                        shiny: false,
                    },
                    battle_appearances: VecDeque::from_iter([
                        MonBattleAppearanceWithRecovery::Inactive(MonBattleAppearance {
                            level: 5.into(),
                            health: (100, 100).into(),
                            status: String::default().into(),
                            ability: "Illusion".to_owned().into(),
                            ..Default::default()
                        }),
                    ]),
                    fainted: true,
                    ..Default::default()
                },
            ])
        );

        // Note that if Zoroark appeared at this point, that means Charmander really did faint...
        // but then we can't create an illusion of it. So this scenario is impossible in a normal
        // battle.
        //
        // If a Mon could illusion a fainted Mon, we would have a problem, since our implementation
        // DIRECTLY relies on the idea that if a Mon appears when it fainted and all Mons
        // have been seen, then the Mon is an illusion.
        //
        // If you could illusion a fainted Mon, we would need to track when a Mon apparently
        // fainted. If all illusion users are seen after that point, then the real Mon can be marked
        // fainted. But this gets weird because that Mon can still *appear* in battle under the
        // illusion. To clients, it will directly look like a fainted Mon is in battle.
        //
        // This is why I think this will NEVER happen. It would be too confusing...
    }
}
