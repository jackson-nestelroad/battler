use alloc::{
    borrow::ToOwned,
    boxed::Box,
    collections::{
        BTreeMap,
        BTreeSet,
        VecDeque,
    },
    format,
    string::String,
    vec::Vec,
};

use anyhow::{
    Error,
    Result,
};
use hashbrown::{
    HashMap,
    HashSet,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    DiscoveryRequired,
    DiscoveryRequiredSet,
};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ambiguity {
    #[default]
    Precise,
    Ambiguous,
}

/// Data about some condition.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionData {
    pub since_turn: usize,
    pub data: HashMap<String, String>,
}

/// Volatile data for a Mon, which applies only when the Mon is active.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonVolatileData {
    pub moves: BTreeSet<String>,
    pub ability: Option<String>,
    pub conditions: BTreeMap<String, ConditionData>,
    pub types: Vec<String>,
    pub added_type: Option<String>,
    pub stat_boosts: BTreeMap<String, i64>,
    pub forme_change: Option<String>,
    pub transformed: Option<(MonPhysicalAppearance, MonBattleAppearanceReference)>,
}

impl MonVolatileData {
    pub(crate) fn record_move(&mut self, name: String) {
        self.moves.insert(name);
    }

    pub(crate) fn record_ability(&mut self, name: String) {
        self.ability = Some(name);
    }

    pub(crate) fn record_condition(&mut self, condition: String, condition_data: ConditionData) {
        self.conditions.insert(condition, condition_data);
    }

    pub(crate) fn remove_condition(&mut self, condition: &str) {
        self.conditions.remove(condition);
    }

    pub(crate) fn record_types<I>(&mut self, types: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.types = types.into_iter().collect();
    }

    pub(crate) fn record_stat_boost(&mut self, stat: String, diff: i64) {
        *self.stat_boosts.entry(stat).or_default() += diff;
    }

    pub(crate) fn record_forme_change(&mut self, forme: String) {
        self.forme_change = Some(forme);
    }

    pub(crate) fn record_transformation(
        &mut self,
        appearance: MonPhysicalAppearance,
        reference: MonBattleAppearanceReference,
    ) {
        self.transformed = Some((appearance, reference));
    }
}

/// The physical appearance of a Mon, which is expected to *never* change through the course of a
/// battle.
///
/// When a Mon creates an illusion, it is expected to mimic this physical appearance.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonPhysicalAppearance {
    pub name: String,
    pub species: String,
    pub gender: String,
    pub shiny: bool,
}

impl MonPhysicalAppearance {
    pub(crate) fn matches(&self, other: &Self) -> bool {
        (self.name.is_empty() || other.name.is_empty() || self.name == other.name)
            && (self.species.is_empty()
                || other.species.is_empty()
                || self.species == other.species)
            && (self.gender.is_empty() || self.gender.is_empty() || self.gender == other.gender)
            && self.shiny == other.shiny
    }
}

#[derive(Debug, Default, Clone)]
pub struct MonBattleAppearanceFromSwitchIn {
    pub level: u64,
    pub health: (u64, u64),
    pub status: String,
    pub terastallization: String,
}

/// Data about a Mon that is slowly discovered and may change throughout the course of the battle.
///
/// Some data, like `level` and `health`, are discovered on switch in. Other data, like `moves` and
/// `ability`, are discovered when used or activated.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonBattleAppearance {
    pub level: DiscoveryRequired<u64>,
    pub health: DiscoveryRequired<(u64, u64)>,
    pub status: DiscoveryRequired<String>,
    pub ability: DiscoveryRequired<String>,
    pub item: DiscoveryRequired<String>,
    pub terastallization: DiscoveryRequired<String>,

    pub moves: DiscoveryRequiredSet<String>,

    pub move_history: VecDeque<String>,
}

impl MonBattleAppearance {
    pub(crate) fn is_empty(&self) -> bool {
        self.level.is_empty()
            && self.health.is_empty()
            && (self.status.is_empty()
                || self.status.known().is_some_and(|status| status.is_empty()))
            && self.ability.is_empty()
            && self.item.is_empty()
            && (self.terastallization.is_empty()
                || self
                    .terastallization
                    .known()
                    .is_some_and(|tera| tera.is_empty()))
            && self.moves.is_empty()
    }

    pub(crate) fn make_ambiguous(mut self) -> Self {
        self.level = self.level.make_ambiguous();
        self.health = self.health.make_ambiguous();
        self.status = self.status.make_ambiguous();
        self.ability = self.ability.make_ambiguous();
        self.item = self.item.make_ambiguous();
        self.terastallization = self.terastallization.make_ambiguous();
        self.moves = self.moves.make_ambiguous();
        self
    }

    pub(crate) fn record_level(&mut self, level: DiscoveryRequired<u64>, ambiguity: Ambiguity) {
        self.level = if ambiguity == Ambiguity::Ambiguous {
            self.level.take().merge(level)
        } else {
            self.level.take().record(level)
        };
    }

    pub(crate) fn record_health(
        &mut self,
        health: DiscoveryRequired<(u64, u64)>,
        ambiguity: Ambiguity,
    ) {
        let health = match (self.health.known(), health.known()) {
            (Some((_, max)), Some((0, 1))) => DiscoveryRequired::Known((0, *max)),
            _ => health,
        };
        self.health = if ambiguity == Ambiguity::Ambiguous {
            self.health.take().merge(health)
        } else {
            self.health.take().record(health)
        };
    }

    pub(crate) fn record_status(
        &mut self,
        status: DiscoveryRequired<String>,
        ambiguity: Ambiguity,
    ) {
        self.status = if ambiguity == Ambiguity::Ambiguous {
            self.status.take().merge(status)
        } else {
            self.status.take().record(status)
        };
    }

    pub(crate) fn record_ability(
        &mut self,
        ability: DiscoveryRequired<String>,
        ambiguity: Ambiguity,
    ) {
        self.ability = if ambiguity == Ambiguity::Ambiguous {
            self.ability.take().merge(ability)
        } else {
            self.ability.take().record(ability)
        };
    }

    pub(crate) fn record_item(&mut self, item: DiscoveryRequired<String>, ambiguity: Ambiguity) {
        self.item = if ambiguity == Ambiguity::Ambiguous {
            self.item.take().merge(item)
        } else {
            self.item.take().record(item)
        };
    }

    pub(crate) fn record_terastallization(
        &mut self,
        terastallization: DiscoveryRequired<String>,
        ambiguity: Ambiguity,
    ) {
        self.terastallization = if ambiguity == Ambiguity::Ambiguous {
            self.terastallization.take().merge(terastallization)
        } else {
            self.terastallization.take().record(terastallization)
        };
    }

    pub(crate) fn record_move(&mut self, name: String, ambiguity: Ambiguity) {
        if ambiguity == Ambiguity::Ambiguous {
            self.moves.record_possible(name);
        } else {
            self.moves.record_known(name);
        }
    }

    pub(crate) fn record_used_move(&mut self, name: String) {
        static MOVE_HISTORY_LIMIT: usize = 10;
        self.move_history.push_back(name);
        if self.move_history.len() > MOVE_HISTORY_LIMIT {
            self.move_history.pop_front();
        }
    }

    pub(crate) fn forget_move(&mut self, name: String, ambiguity: Ambiguity) {
        if ambiguity == Ambiguity::Ambiguous {
            self.moves.downgrade_to_possible_value(name);
        } else {
            self.moves.remove_known(&name);
        }
    }

    pub(crate) fn record_all(&mut self, other: Self) {
        self.record_level(other.level, Ambiguity::Precise);
        self.record_health(other.health, Ambiguity::Precise);
        self.record_status(other.status, Ambiguity::Precise);
        self.record_ability(other.ability, Ambiguity::Precise);
        self.record_item(other.item, Ambiguity::Precise);
        self.record_terastallization(other.terastallization, Ambiguity::Precise);

        for mov in other.moves.known().iter().cloned() {
            self.record_move(mov, Ambiguity::Precise);
        }
        for mov in other.moves.possible_values().iter().cloned() {
            self.record_move(mov, Ambiguity::Ambiguous);
        }

        for mov in other.move_history {
            self.record_used_move(mov);
        }
    }
}

impl From<&MonBattleAppearanceFromSwitchIn> for MonBattleAppearance {
    fn from(value: &MonBattleAppearanceFromSwitchIn) -> Self {
        let mut data = MonBattleAppearance::default();
        data.record_level(value.level.into(), Ambiguity::Precise);
        data.record_health(value.health.into(), Ambiguity::Precise);
        data.record_status(value.status.clone().into(), Ambiguity::Precise);
        data.record_terastallization(value.terastallization.clone().into(), Ambiguity::Precise);
        data
    }
}

/// A version of [`MonBattleAppearance`] that supports recovery when a Mon is discovered to be an
/// illusion of another Mon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonBattleAppearanceWithRecovery {
    /// Mon is inactive, so it contains a single battle appearance.
    #[serde(rename = "inactive")]
    Inactive(MonBattleAppearance),
    /// Mon is active, so it records the difference between battle data up to the last switch out
    /// and battle data from the last switch in. If the Mon is found to be an illusion, the battle
    /// data from the last switch in is moved to another Mon.
    #[serde(rename = "active")]
    Active {
        primary_battle_appearance: MonBattleAppearance,
        battle_appearance_up_to_last_switch_out: MonBattleAppearance,
        battle_appearance_from_last_switch_in: MonBattleAppearance,
    },
}

impl MonBattleAppearanceWithRecovery {
    pub(crate) fn take_primary(self) -> MonBattleAppearance {
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

    pub(crate) fn matches_switch_in(&self, appearance: &MonBattleAppearanceFromSwitchIn) -> bool {
        // Only match the level. Health and status may change without us knowing.
        self.primary().level.can_be(&appearance.level)
    }

    pub(crate) fn switch_in(&mut self) {
        let mut taken = Self::default();
        core::mem::swap(self, &mut taken);
        *self = match taken {
            Self::Inactive(appearance) => Self::Active {
                primary_battle_appearance: appearance.clone(),
                battle_appearance_up_to_last_switch_out: appearance,
                battle_appearance_from_last_switch_in: MonBattleAppearance::default(),
            },
            active @ Self::Active { .. } => active,
        }
    }

    pub(crate) fn switch_out(&mut self) {
        let mut taken = Self::default();
        core::mem::swap(self, &mut taken);
        *self = match taken {
            Self::Inactive(appearance) => Self::Inactive(appearance),
            Self::Active {
                primary_battle_appearance,
                ..
            } => Self::Inactive(primary_battle_appearance),
        }
    }

    pub(crate) fn recover(&mut self) -> MonBattleAppearance {
        match self {
            Self::Inactive(_) => MonBattleAppearance::default(),
            Self::Active {
                primary_battle_appearance,
                battle_appearance_up_to_last_switch_out,
                battle_appearance_from_last_switch_in,
            } => {
                *primary_battle_appearance = battle_appearance_up_to_last_switch_out.clone();
                let mut out = MonBattleAppearance::default();
                core::mem::swap(&mut out, battle_appearance_from_last_switch_in);
                out
            }
        }
    }

    pub(crate) fn apply_for_each_battle_appearance<F>(&mut self, f: F)
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

    pub(crate) fn record_level(&mut self, level: DiscoveryRequired<u64>, ambiguity: Ambiguity) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_level(level.clone(), ambiguity);
        });
    }

    pub(crate) fn record_health(
        &mut self,
        health: DiscoveryRequired<(u64, u64)>,
        ambiguity: Ambiguity,
    ) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_health(health.clone(), ambiguity);
        });
    }

    pub(crate) fn record_status(
        &mut self,
        status: DiscoveryRequired<String>,
        ambiguity: Ambiguity,
    ) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_status(status.clone(), ambiguity);
        });
    }

    pub(crate) fn record_ability(
        &mut self,
        ability: DiscoveryRequired<String>,
        ambiguity: Ambiguity,
    ) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_ability(ability.clone(), ambiguity);
        });
    }

    pub(crate) fn record_item(&mut self, item: DiscoveryRequired<String>, ambiguity: Ambiguity) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_item(item.clone(), ambiguity);
        });
    }

    pub(crate) fn record_terastallization(
        &mut self,
        terastallization: DiscoveryRequired<String>,
        ambiguity: Ambiguity,
    ) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_terastallization(terastallization.clone(), ambiguity);
        });
    }

    pub(crate) fn record_move(&mut self, name: String, ambiguity: Ambiguity) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_move(name.clone(), ambiguity);
        });
    }

    pub(crate) fn record_used_move(&mut self, name: String) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.record_used_move(name.clone());
        });
    }

    pub(crate) fn forget_move(&mut self, name: String, ambiguity: Ambiguity) {
        self.apply_for_each_battle_appearance(|appearance| {
            appearance.forget_move(name.clone(), ambiguity);
        });
    }

    pub(crate) fn record_all(&mut self, other: MonBattleAppearance) {
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mon {
    pub physical_appearance: MonPhysicalAppearance,
    pub battle_appearances: VecDeque<MonBattleAppearanceWithRecovery>,
    pub fainted: bool,
    pub volatile_data: MonVolatileData,
}

impl Mon {
    const MAX_BATTLE_APPEARANCES_LENGTH: usize = 25;

    pub(crate) fn new<I>(physical_appearance: MonPhysicalAppearance, battle_appearances: I) -> Self
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

    pub(crate) fn switch_in(&mut self) {
        self.switch_out();
        self.revive();
    }

    pub(crate) fn switch_out(&mut self) {
        self.volatile_data = MonVolatileData::default();

        for battle_appearance in &mut self.battle_appearances {
            battle_appearance.switch_out();
        }
    }

    pub(crate) fn faint(&mut self) {
        self.fainted = true;
    }

    pub(crate) fn revive(&mut self) {
        self.fainted = false;
    }

    pub(crate) fn push_battle_appearance(&mut self) -> usize {
        // If we exceed maximum number of battle appearances, recycle it as the base.
        //
        // Currently, the maximum is set really high so this does not really happen.
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

    pub(crate) fn remove_battle_appearance(&mut self, index: usize) {
        self.battle_appearances.remove(index);
    }
}

/// A reference to a [`MonBattleAppearance`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonBattleAppearanceReference {
    pub player: String,
    pub mon_index: usize,
    pub battle_appearance_index: usize,
}

/// A player participating in a battle, which consists of one or more Mons.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Side {
    pub name: String,
    pub id: usize,
    pub players: BTreeMap<String, Player>,
    pub conditions: BTreeMap<String, ConditionData>,
    pub slot_conditions: Vec<BTreeMap<String, ConditionData>>,
    pub active: Vec<Option<MonBattleAppearanceReference>>,
}

impl Side {
    pub(crate) fn player_or_else(&self, player: &str) -> Result<&Player> {
        self.players
            .get(player)
            .ok_or_else(|| Error::msg(format!("player {player} does not exist")))
    }

    pub(crate) fn player_mut_or_else(&mut self, player: &str) -> Result<&mut Player> {
        self.players
            .get_mut(player)
            .ok_or_else(|| Error::msg(format!("player {player} does not exist")))
    }

    pub(crate) fn mon_by_reference_or_else(
        &self,
        reference: &MonBattleAppearanceReference,
    ) -> Result<&Mon> {
        let player = self.player_or_else(&reference.player)?;
        player.mons.get(reference.mon_index).ok_or_else(|| {
            Error::msg(format!(
                "mon at index {} does not exist",
                reference.mon_index
            ))
        })
    }

    pub(crate) fn mon_mut_by_reference_or_else(
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

    pub(crate) fn mon_battle_appearance_with_recovery_by_reference_or_else(
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

    pub(crate) fn mon_battle_appearance_with_recovery_mut_by_reference_or_else(
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

    pub(crate) fn active_mon_reference_by_position(
        &self,
        position: usize,
    ) -> Option<MonBattleAppearanceReference> {
        self.active.get(position).cloned().flatten()
    }

    pub(crate) fn mon_index_is_active(&self, index: usize) -> bool {
        self.active.iter().any(|active| {
            active
                .as_ref()
                .is_some_and(|active| active.mon_index == index)
        })
    }

    pub(crate) fn mon_is_active(&self, reference: &MonBattleAppearanceReference) -> bool {
        self.active
            .iter()
            .any(|active| active.as_ref().is_some_and(|active| active == reference))
    }

    pub(crate) fn mons_by_name(
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

    pub(crate) fn mon_by_appearance(
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
                    && (player_has_seen_all_mons
                        || (!mon.fainted && !self.mon_index_is_active(*mon_index)))
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

    pub(crate) fn switch_in(
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

    pub(crate) fn switch_out(
        &mut self,
        mon: &MonBattleAppearanceReference,
        remove_from_active_position: bool,
    ) -> Result<()> {
        for (i, active) in self
            .active
            .iter()
            .enumerate()
            .filter_map(|(i, reference)| reference.clone().map(|reference| (i, reference)))
            .filter(|(_, reference)| reference == mon)
            .collect::<Vec<_>>()
        {
            self.mon_mut_by_reference_or_else(&active)?.switch_out();

            if remove_from_active_position {
                // SAFETY: i was generated from enumeration of self.active.
                let active = self.active.get_mut(i).unwrap();
                *active = None;
            }
        }
        Ok(())
    }

    pub(crate) fn faint_an_inactive_illusion_user(&mut self, player_id: &str) -> Result<()> {
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

    pub(crate) fn merge_one_mon(&mut self, player_id: &str) -> Result<Option<usize>> {
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
            core::mem::swap(&mut battle_appearances, &mut other_mon.battle_appearances);

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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub sides: Vec<Side>,
    pub environment: Option<String>,
    pub time: Option<String>,
    pub weather: Option<String>,
    pub conditions: BTreeMap<String, ConditionData>,
    pub rules: Vec<String>,
    pub max_side_length: usize,
}

impl Field {
    pub(crate) fn side_or_else(&self, side: usize) -> Result<&Side> {
        self.sides
            .get(side)
            .ok_or_else(|| Error::msg(format!("side {side} does not exist")))
    }

    pub(crate) fn side_mut_or_else(&mut self, side: usize) -> Result<&mut Side> {
        self.sides
            .get_mut(side)
            .ok_or_else(|| Error::msg(format!("side {side} does not exist")))
    }

    pub(crate) fn side_and_player_or_else(&self, player: &str) -> Result<(usize, &Player)> {
        for (i, side) in self.sides.iter().enumerate() {
            if let Ok(player) = side.player_or_else(player) {
                return Ok((i, player));
            }
        }
        return Err(Error::msg(format!("player {player} does not exist")));
    }

    pub(crate) fn side_and_player_mut_or_else(
        &mut self,
        player: &str,
    ) -> Result<(usize, &mut Player)> {
        for (i, side) in self.sides.iter_mut().enumerate() {
            if let Ok(player) = side.player_mut_or_else(player) {
                return Ok((i, player));
            }
        }
        return Err(Error::msg(format!("player {player} does not exist")));
    }

    pub(crate) fn side_for_player(&self, player: &str) -> Result<usize> {
        self.side_and_player_or_else(player).map(|(side, _)| side)
    }

    pub(crate) fn player_mut_or_else(&mut self, player: &str) -> Result<&mut Player> {
        self.side_and_player_mut_or_else(player)
            .map(|(_, player)| player)
    }

    pub(crate) fn active_mons_on_side<'a>(
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
            None => Box::new(core::iter::empty()),
        }
    }

    pub(crate) fn active_mons(&mut self) -> impl Iterator<Item = MonBattleAppearanceReference> {
        self.sides
            .iter()
            .flat_map(|side| side.active.iter().cloned().filter_map(|val| val))
    }

    pub(crate) fn mon_by_reference_or_else(
        &self,
        reference: &MonBattleAppearanceReference,
    ) -> Result<&Mon> {
        let side = self.side_for_player(&reference.player)?;
        let side = self.side_or_else(side)?;
        side.mon_by_reference_or_else(reference)
    }

    pub(crate) fn mon_mut_by_reference_or_else(
        &mut self,
        reference: &MonBattleAppearanceReference,
    ) -> Result<&mut Mon> {
        let side = self.side_for_player(&reference.player)?;
        let side = self.side_mut_or_else(side)?;
        side.mon_mut_by_reference_or_else(reference)
    }

    pub(crate) fn mon_battle_appearance_with_recovery_by_reference_or_else(
        &self,
        reference: &MonBattleAppearanceReference,
    ) -> Result<&MonBattleAppearanceWithRecovery> {
        let side = self.side_for_player(&reference.player)?;
        let side = self.side_or_else(side)?;
        side.mon_battle_appearance_with_recovery_by_reference_or_else(reference)
    }

    pub(crate) fn mon_battle_appearance_with_recovery_mut_by_reference_or_else(
        &mut self,
        reference: &MonBattleAppearanceReference,
    ) -> Result<&mut MonBattleAppearanceWithRecovery> {
        let side = self.side_for_player(&reference.player)?;
        let side = self.side_mut_or_else(side)?;
        side.mon_battle_appearance_with_recovery_mut_by_reference_or_else(reference)
    }

    pub(crate) fn active_mon_reference_by_position(
        &mut self,
        side: usize,
        position: usize,
    ) -> Result<Option<MonBattleAppearanceReference>> {
        let side = self.side_mut_or_else(side)?;
        Ok(side.active_mon_reference_by_position(position))
    }

    pub(crate) fn mons_by_name(
        &mut self,
        player: &str,
        name: &str,
        active_filter: Option<bool>,
    ) -> Result<Vec<MonBattleAppearanceReference>> {
        let (side, _) = self.side_and_player_mut_or_else(player)?;
        self.side_mut_or_else(side)?
            .mons_by_name(player, name, active_filter)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BattlePhase {
    #[default]
    #[serde(rename = "pre_battle")]
    PreBattle,
    #[serde(rename = "pre_team_preview")]
    PreTeamPreview,
    #[serde(rename = "team_preview")]
    TeamPreview(usize),
    #[serde(rename = "battle")]
    Battle,
    #[serde(rename = "finished")]
    Finished,
}
