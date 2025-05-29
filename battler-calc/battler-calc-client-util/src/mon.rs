use std::{
    collections::HashSet,
    fmt::Debug,
    hash::Hash,
    str::FromStr,
};

use anyhow::{
    Error,
    Result,
};
use battler::{
    BoostTable,
    DataStoreByName,
    Fraction,
    Gender,
    MonBattleData,
    Nature,
    StatTable,
    Type,
};
use battler_client::{
    state::{
        BattleState,
        ConditionData,
        MonBattleAppearanceReference,
    },
    state_util::{
        mon_ability,
        mon_active_position,
        mon_boosts,
        mon_conditions,
        mon_health,
        mon_item,
        mon_level,
        mon_moves,
        mon_or_else,
        mon_status,
        mon_types,
        player_or_else,
        side_conditions,
        side_for_mon,
        side_or_else,
    },
};

/// An reference to a Mon.
///
/// If a Mon is owned by the user, then more data about it is available directly from the battle
/// engine. Otherwise, we only know data from the discovered battle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonReference<'m> {
    Battle {
        side: usize,
        player: String,
        battle_data: &'m MonBattleData,
    },
    State(&'m MonBattleAppearanceReference),
}

impl Hash for MonReference<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Battle {
                side,
                player,
                battle_data,
            } => {
                side.hash(state);
                player.hash(state);
                battle_data.summary.hash(state);
                battle_data.player_team_position.hash(state);
            }
            Self::State(reference) => reference.hash(state),
        }
    }
}

/// A wrapper around a [`MonReference`] that allows data to be read from either the player data or
/// the battle state.
///
/// Player data is always preferred when available.
#[derive(Clone)]
pub struct Mon<'s, 'd> {
    reference: MonReference<'s>,
    state: &'s BattleState,
    data: &'d dyn DataStoreByName,
}

impl<'s, 'd> Mon<'s, 'd> {
    pub fn new(
        reference: MonReference<'s>,
        state: &'s BattleState,
        data: &'d dyn DataStoreByName,
    ) -> Self {
        Self {
            reference,
            state,
            data,
        }
    }

    pub fn state(&self) -> &'s BattleState {
        self.state
    }

    pub fn data(&self) -> &'d dyn DataStoreByName {
        self.data
    }

    pub fn reference(&self) -> &MonReference<'s> {
        &self.reference
    }

    pub fn active_state_reference(&self) -> Result<Option<MonBattleAppearanceReference>> {
        match &self.reference {
            MonReference::Battle {
                side, battle_data, ..
            } => match battle_data.side_position {
                Some(position) => side_or_else(self.state, *side)?
                    .active
                    .get(position)
                    .cloned()
                    .ok_or_else(|| Error::msg("mon active position is empty in battle state")),
                None => return Ok(None),
            },
            MonReference::State(reference) => {
                if mon_active_position(self.state, reference)?.is_none() {
                    return Ok(None);
                }
                Ok(Some((*reference).clone()))
            }
        }
    }

    pub fn battle_data(&self) -> Result<Option<&MonBattleData>> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(Some(&battle_data)),
            MonReference::State(_) => Ok(None),
        }
    }

    pub fn side(&self) -> Result<usize> {
        match &self.reference {
            MonReference::Battle { side, .. } => Ok(*side),
            MonReference::State(reference) => side_for_mon(self.state, reference),
        }
    }

    pub fn player(&self) -> Result<String> {
        match &self.reference {
            MonReference::Battle { player, .. } => Ok(player.clone()),
            MonReference::State(reference) => Ok(reference.player.clone()),
        }
    }

    pub fn player_can_switch(&self) -> Result<bool> {
        let player = player_or_else(self.state, &self.player()?)?;
        Ok((0..player.team_size)
            .filter(|i| player.mons.get(*i).is_none_or(|mon| !mon.fainted))
            .count()
            > 1)
    }

    pub fn active_position(&self) -> Result<Option<usize>> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(battle_data.side_position),
            MonReference::State(reference) => mon_active_position(self.state, reference),
        }
    }

    pub fn active_mon_state(&self) -> Result<Option<&battler_client::state::Mon>> {
        match self.active_state_reference()? {
            Some(reference) => mon_or_else(self.state, &reference).map(|mon| Some(mon)),
            None => Ok(None),
        }
    }

    pub fn name(&self) -> Result<&str> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(&battle_data.summary.name),
            MonReference::State(reference) => {
                Ok(&mon_or_else(self.state, reference)?.physical_appearance.name)
            }
        }
    }

    pub fn species(&self) -> Result<&str> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(&battle_data.species),
            MonReference::State(reference) => Ok(&mon_or_else(self.state, reference)?
                .physical_appearance
                .species),
        }
    }

    pub fn level(&self) -> Result<Option<u64>> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(Some(battle_data.summary.level as u64)),
            MonReference::State(reference) => mon_level(self.state, reference),
        }
    }

    pub fn health(&self) -> Result<Option<(u64, u64)>> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => {
                Ok(Some((battle_data.hp as u64, battle_data.max_hp as u64)))
            }
            MonReference::State(reference) => mon_health(self.state, reference),
        }
    }

    pub fn health_fraction(&self) -> Result<Option<Fraction<u64>>> {
        Ok(self.health()?.map(|(a, b)| Fraction::new(a, b)))
    }

    pub fn ability(&self) -> Result<Option<&str>> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(Some(&battle_data.ability)),
            MonReference::State(reference) => mon_ability(self.state, reference),
        }
    }

    pub fn item(&self) -> Result<Option<&str>> {
        // NOTE: None means status is not known. Empty string means Mon has no item.
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(Some(
                battle_data
                    .item
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or_default(),
            )),
            MonReference::State(reference) => mon_item(self.state, reference),
        }
    }

    pub fn moves(&self) -> Result<Vec<&str>> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(battle_data
                .moves
                .iter()
                .map(|mov| mov.name.as_str())
                .collect()),
            MonReference::State(reference) => {
                mon_moves(self.state, reference, false).map(|iter| iter.collect())
            }
        }
    }

    pub fn gender(&self) -> Result<Gender> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(battle_data.summary.gender),
            MonReference::State(reference) => Ok(Gender::from_str(
                &mon_or_else(self.state, reference)?
                    .physical_appearance
                    .gender,
            )
            .unwrap_or_default()),
        }
    }

    pub fn shiny(&self) -> Result<bool> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(battle_data.summary.shiny),
            MonReference::State(reference) => Ok(mon_or_else(self.state, reference)?
                .physical_appearance
                .shiny),
        }
    }

    pub fn nature(&self) -> Option<Nature> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Some(battle_data.summary.nature),
            MonReference::State(_) => None,
        }
    }

    pub fn ivs(&self) -> Option<StatTable> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Some(battle_data.summary.ivs.clone()),
            MonReference::State(_) => None,
        }
    }

    pub fn evs(&self) -> Option<StatTable> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Some(battle_data.summary.evs.clone()),
            MonReference::State(_) => None,
        }
    }

    pub fn boosts(&self) -> Result<BoostTable> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(battle_data.boosts.clone()),
            MonReference::State(reference) => mon_boosts(self.state, reference),
        }
    }

    pub fn status(&self) -> Result<Option<&str>> {
        // NOTE: None means status is not known. Empty string means Mon has no status.
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(Some(
                battle_data
                    .status
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or_default(),
            )),
            MonReference::State(reference) => mon_status(self.state, reference),
        }
    }

    pub fn types(&self) -> Result<Vec<Type>> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => Ok(battle_data.types.clone()),
            MonReference::State(reference) => mon_types(self.state, reference, self.data),
        }
    }

    pub fn conditions(&self) -> Result<HashSet<&str>> {
        match self.active_state_reference()? {
            Some(reference) => Ok(mon_conditions(self.state, &reference)?.collect()),
            None => Ok(HashSet::default()),
        }
    }

    pub fn condition_data(&self, condition: &str) -> Result<Option<&ConditionData>> {
        match self.active_mon_state()? {
            Some(mon) => Ok(mon.volatile_data.conditions.get(condition)),
            None => Ok(None),
        }
    }

    pub fn side_conditions(&self) -> Result<HashSet<&str>> {
        Ok(side_conditions(self.state, self.side()?)?.collect())
    }

    pub fn side_condition_data(&self, condition: &str) -> Result<Option<&ConditionData>> {
        Ok(side_or_else(self.state, self.side()?)?
            .conditions
            .get(condition))
    }

    pub fn hidden_power_type(&self) -> Result<Option<Type>> {
        match &self.reference {
            MonReference::Battle { battle_data, .. } => {
                Ok(Some(battle_data.summary.hidden_power_type))
            }
            MonReference::State(_) => Ok(None),
        }
    }

    pub fn relative_position(&self, other: &Mon) -> Result<Option<isize>> {
        let side = self.side()?;
        let position = match self.active_position()? {
            Some(position) => position,
            None => return Ok(None),
        };

        let other_side = other.side()?;
        let other_position = match other.active_position()? {
            Some(position) => position,
            None => return Ok(None),
        };

        if side == other_side {
            let diff = position.abs_diff(other_position);
            let diff: isize = diff.try_into()?;
            Ok(Some(-diff))
        } else {
            let max_side_length = self.state.field.max_side_length;
            if other_position >= max_side_length {
                return Err(Error::msg("position is out of bounds"));
            }
            let flipped_target_position = max_side_length - other_position - 1;
            let diff = position.abs_diff(flipped_target_position) + 1;
            let diff: isize = diff.try_into()?;
            Ok(Some(diff))
        }
    }

    pub fn is_adjacent(&self, other: &Mon, adjacency_reach: usize) -> Result<bool> {
        let side = self.side()?;
        let position = match self.active_position()? {
            Some(position) => position,
            None => return Ok(false),
        };

        let other_side = other.side()?;
        let mut other_position = match other.active_position()? {
            Some(position) => position,
            None => return Ok(false),
        };

        if side != other_side {
            let max_side_length = self.state.field.max_side_length;
            if other_position >= max_side_length {
                return Err(Error::msg("position is out of bounds"));
            }
            other_position = max_side_length - other_position - 1;
        }

        let diff = position.abs_diff(other_position);

        Ok(diff <= adjacency_reach - 1)
    }

    pub fn is_ally(&self, other: &Mon) -> Result<bool> {
        Ok(self.side()? == other.side()?)
    }

    pub fn is_foe(&self, other: &Mon) -> Result<bool> {
        Ok(self.is_ally(other)?)
    }

    pub fn is_same(&self, other: &Mon) -> Result<bool> {
        if let Some(pos) = self.relative_position(other)?
            && pos == 0
        {
            return Ok(true);
        }

        match (&self.reference, &other.reference) {
            (
                MonReference::Battle {
                    side: left_side,
                    player: left_player,
                    battle_data: left_battle_data,
                },
                MonReference::Battle {
                    side: right_side,
                    player: right_player,
                    battle_data: right_battle_data,
                },
            ) => {
                return Ok(left_side == right_side
                    && left_player == right_player
                    && left_battle_data == right_battle_data);
            }
            (MonReference::State(left), MonReference::State(right)) => return Ok(left == right),
            _ => (),
        }

        // At this point, reference types are different, so we need to try our best to determine if
        // the Mons are the same.
        match (
            self.active_state_reference()?,
            other.active_state_reference()?,
        ) {
            (Some(left), Some(right)) => return Ok(left == right),
            _ => (),
        }
        // Mon in BattleData is not active, so we cannot get a state reference to check
        // equality.
        //
        // Check some other basic fields. We shouldn't really ever go to this logic unless
        // we are working with inactive Mons.
        //
        // NOTE: This is still an assumption. It is possible that a player has two Mons that
        // look exactly the same. This logic will say that the Mons are the same even if
        // they are not.
        Ok(self.side()? == other.side()?
            && self.player()? == other.player()?
            && self.name()? == other.name()?
            && self.species()? == other.species()?
            && self.gender()? == other.gender()?
            && self.shiny()? == other.shiny()?
            && self.level()? == other.level()?
            && self.status()? == other.status()?)
    }
}

impl Debug for Mon<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.reference)
    }
}

#[cfg(test)]
mod mon_test {
    use std::collections::BTreeMap;

    use battler::{
        LocalDataStore,
        MonBattleData,
    };
    use battler_client::state::{
        BattleState,
        Field,
        MonBattleAppearanceReference,
        Player,
        Side,
    };

    use crate::{
        Mon,
        MonReference,
    };

    #[test]
    fn active_location() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let state = BattleState {
            field: Field {
                max_side_length: 3,
                sides: Vec::from_iter([
                    Side {
                        players: BTreeMap::from_iter([
                            ("player-1".to_owned(), Player::default()),
                            ("player-2".to_owned(), Player::default()),
                            ("player-3".to_owned(), Player::default()),
                        ]),
                        active: Vec::from_iter([
                            Some(MonBattleAppearanceReference {
                                player: "player-1".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-2".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-3".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                        ]),
                        ..Default::default()
                    },
                    Side {
                        players: BTreeMap::from_iter([
                            ("player-4".to_owned(), Player::default()),
                            ("player-5".to_owned(), Player::default()),
                            ("player-6".to_owned(), Player::default()),
                        ]),
                        active: Vec::from_iter([
                            Some(MonBattleAppearanceReference {
                                player: "player-4".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-5".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-6".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                        ]),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            },
            ..Default::default()
        };

        let mon_player_1 = MonBattleData {
            side_position: Some(0),
            ..Default::default()
        };
        let mon_player_1 = Mon::new(
            MonReference::Battle {
                side: 0,
                player: "player-1".to_owned(),
                battle_data: &mon_player_1,
            },
            &state,
            &data,
        );

        let mon_player_5 = MonBattleAppearanceReference {
            player: "player-5".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_5 = Mon::new(MonReference::State(&mon_player_5), &state, &data);

        assert_matches::assert_matches!(mon_player_1.side(), Ok(0));
        assert_matches::assert_matches!(mon_player_1.active_position(), Ok(Some(0)));
        assert_matches::assert_matches!(mon_player_5.side(), Ok(1));
        assert_matches::assert_matches!(mon_player_5.active_position(), Ok(Some(1)));
    }

    #[test]
    fn relative_position() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let state = BattleState {
            field: Field {
                max_side_length: 3,
                sides: Vec::from_iter([
                    Side {
                        players: BTreeMap::from_iter([
                            ("player-1".to_owned(), Player::default()),
                            ("player-2".to_owned(), Player::default()),
                            ("player-3".to_owned(), Player::default()),
                        ]),
                        active: Vec::from_iter([
                            Some(MonBattleAppearanceReference {
                                player: "player-1".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-2".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-3".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                        ]),
                        ..Default::default()
                    },
                    Side {
                        players: BTreeMap::from_iter([
                            ("player-4".to_owned(), Player::default()),
                            ("player-5".to_owned(), Player::default()),
                            ("player-6".to_owned(), Player::default()),
                        ]),
                        active: Vec::from_iter([
                            Some(MonBattleAppearanceReference {
                                player: "player-4".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-5".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-6".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                        ]),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            },
            ..Default::default()
        };

        let mon_player_1 = MonBattleAppearanceReference {
            player: "player-1".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_1 = Mon::new(MonReference::State(&mon_player_1), &state, &data);

        let mon_player_2 = MonBattleAppearanceReference {
            player: "player-2".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_2 = Mon::new(MonReference::State(&mon_player_2), &state, &data);

        let mon_player_3 = MonBattleAppearanceReference {
            player: "player-3".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_3 = Mon::new(MonReference::State(&mon_player_3), &state, &data);

        let mon_player_4 = MonBattleAppearanceReference {
            player: "player-4".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_4 = Mon::new(MonReference::State(&mon_player_4), &state, &data);

        let mon_player_5 = MonBattleAppearanceReference {
            player: "player-5".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_5 = Mon::new(MonReference::State(&mon_player_5), &state, &data);

        let mon_player_6 = MonBattleAppearanceReference {
            player: "player-6".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_6 = Mon::new(MonReference::State(&mon_player_6), &state, &data);

        assert_matches::assert_matches!(mon_player_1.relative_position(&mon_player_1), Ok(Some(0)));
        assert_matches::assert_matches!(
            mon_player_1.relative_position(&mon_player_2),
            Ok(Some(-1))
        );
        assert_matches::assert_matches!(
            mon_player_1.relative_position(&mon_player_3),
            Ok(Some(-2))
        );
        assert_matches::assert_matches!(mon_player_1.relative_position(&mon_player_4), Ok(Some(3)));
        assert_matches::assert_matches!(mon_player_1.relative_position(&mon_player_5), Ok(Some(2)));
        assert_matches::assert_matches!(mon_player_1.relative_position(&mon_player_6), Ok(Some(1)));
    }

    #[test]
    fn relative_location_with_battle_data() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let state = BattleState {
            field: Field {
                max_side_length: 3,
                sides: Vec::from_iter([
                    Side {
                        players: BTreeMap::from_iter([
                            ("player-1".to_owned(), Player::default()),
                            ("player-2".to_owned(), Player::default()),
                            ("player-3".to_owned(), Player::default()),
                        ]),
                        active: Vec::from_iter([
                            Some(MonBattleAppearanceReference {
                                player: "player-1".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-2".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-3".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                        ]),
                        ..Default::default()
                    },
                    Side {
                        players: BTreeMap::from_iter([
                            ("player-4".to_owned(), Player::default()),
                            ("player-5".to_owned(), Player::default()),
                            ("player-6".to_owned(), Player::default()),
                        ]),
                        active: Vec::from_iter([
                            Some(MonBattleAppearanceReference {
                                player: "player-4".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-5".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-6".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                        ]),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            },
            ..Default::default()
        };

        let mon_player_1 = MonBattleData {
            side_position: Some(0),
            ..Default::default()
        };
        let mon_player_1 = Mon::new(
            MonReference::Battle {
                side: 0,
                player: "player-1".to_owned(),
                battle_data: &mon_player_1,
            },
            &state,
            &data,
        );

        let mon_player_2 = MonBattleData {
            side_position: Some(1),
            ..Default::default()
        };
        let mon_player_2 = Mon::new(
            MonReference::Battle {
                side: 0,
                player: "player-2".to_owned(),
                battle_data: &mon_player_2,
            },
            &state,
            &data,
        );

        let mon_player_5 = MonBattleAppearanceReference {
            player: "player-5".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_5 = Mon::new(MonReference::State(&mon_player_5), &state, &data);

        assert_matches::assert_matches!(
            mon_player_1.relative_position(&mon_player_2),
            Ok(Some(-1))
        );
        assert_matches::assert_matches!(mon_player_1.relative_position(&mon_player_5), Ok(Some(2)));
    }

    #[test]
    fn is_adjacent() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let state = BattleState {
            field: Field {
                max_side_length: 3,
                sides: Vec::from_iter([
                    Side {
                        players: BTreeMap::from_iter([
                            ("player-1".to_owned(), Player::default()),
                            ("player-2".to_owned(), Player::default()),
                            ("player-3".to_owned(), Player::default()),
                        ]),
                        active: Vec::from_iter([
                            Some(MonBattleAppearanceReference {
                                player: "player-1".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-2".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-3".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                        ]),
                        ..Default::default()
                    },
                    Side {
                        players: BTreeMap::from_iter([
                            ("player-4".to_owned(), Player::default()),
                            ("player-5".to_owned(), Player::default()),
                            ("player-6".to_owned(), Player::default()),
                        ]),
                        active: Vec::from_iter([
                            Some(MonBattleAppearanceReference {
                                player: "player-4".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-5".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                            Some(MonBattleAppearanceReference {
                                player: "player-6".to_owned(),
                                mon_index: 0,
                                battle_appearance_index: 0,
                            }),
                        ]),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            },
            ..Default::default()
        };

        let mon_player_1 = MonBattleData {
            side_position: Some(0),
            ..Default::default()
        };
        let mon_player_1 = Mon::new(
            MonReference::Battle {
                side: 0,
                player: "player-1".to_owned(),
                battle_data: &mon_player_1,
            },
            &state,
            &data,
        );

        let mon_player_2 = MonBattleData {
            side_position: Some(1),
            ..Default::default()
        };
        let mon_player_2 = Mon::new(
            MonReference::Battle {
                side: 0,
                player: "player-2".to_owned(),
                battle_data: &mon_player_2,
            },
            &state,
            &data,
        );

        let mon_player_3 = MonBattleAppearanceReference {
            player: "player-3".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_3 = Mon::new(MonReference::State(&mon_player_3), &state, &data);

        let mon_player_4 = MonBattleAppearanceReference {
            player: "player-4".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_4 = Mon::new(MonReference::State(&mon_player_4), &state, &data);

        let mon_player_5 = MonBattleAppearanceReference {
            player: "player-5".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_5 = Mon::new(MonReference::State(&mon_player_5), &state, &data);

        let mon_player_6 = MonBattleAppearanceReference {
            player: "player-6".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let mon_player_6 = Mon::new(MonReference::State(&mon_player_6), &state, &data);

        assert_matches::assert_matches!(mon_player_1.is_adjacent(&mon_player_2, 2), Ok(true));
        assert_matches::assert_matches!(mon_player_1.is_adjacent(&mon_player_3, 2), Ok(false));
        assert_matches::assert_matches!(mon_player_1.is_adjacent(&mon_player_4, 2), Ok(false));
        assert_matches::assert_matches!(mon_player_1.is_adjacent(&mon_player_5, 2), Ok(true));
        assert_matches::assert_matches!(mon_player_1.is_adjacent(&mon_player_6, 2), Ok(true));
    }

    #[test]
    fn is_same_with_battle_data_only() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let state = BattleState {
            field: Field {
                max_side_length: 1,
                sides: Vec::from_iter([
                    Side {
                        players: BTreeMap::from_iter([("player-1".to_owned(), Player::default())]),
                        active: Vec::from_iter([Some(MonBattleAppearanceReference {
                            player: "player-1".to_owned(),
                            mon_index: 0,
                            battle_appearance_index: 0,
                        })]),
                        ..Default::default()
                    },
                    Side {
                        players: BTreeMap::from_iter([("player-2".to_owned(), Player::default())]),
                        active: Vec::from_iter([Some(MonBattleAppearanceReference {
                            player: "player-2".to_owned(),
                            mon_index: 0,
                            battle_appearance_index: 0,
                        })]),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            },
            ..Default::default()
        };

        let active_mon_player_1 = MonBattleData {
            side_position: Some(0),
            ..Default::default()
        };
        let active_mon_player_1 = Mon::new(
            MonReference::Battle {
                side: 0,
                player: "player-1".to_owned(),
                battle_data: &active_mon_player_1,
            },
            &state,
            &data,
        );

        let inactive_mon_player_1 = MonBattleData {
            ..Default::default()
        };
        let inactive_mon_player_1 = Mon::new(
            MonReference::Battle {
                side: 0,
                player: "player-1".to_owned(),
                battle_data: &inactive_mon_player_1,
            },
            &state,
            &data,
        );

        let active_mon_player_2 = MonBattleData {
            side_position: Some(0),
            ..Default::default()
        };
        let active_mon_player_2 = Mon::new(
            MonReference::Battle {
                side: 1,
                player: "player-2".to_owned(),
                battle_data: &active_mon_player_2,
            },
            &state,
            &data,
        );

        let inactive_mon_player_2 = MonBattleData {
            ..Default::default()
        };
        let inactive_mon_player_2 = Mon::new(
            MonReference::Battle {
                side: 1,
                player: "player-2".to_owned(),
                battle_data: &inactive_mon_player_2,
            },
            &state,
            &data,
        );

        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&active_mon_player_1),
            Ok(true)
        );
        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&active_mon_player_2),
            Ok(false)
        );
        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&inactive_mon_player_1),
            Ok(false)
        );
        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&inactive_mon_player_2),
            Ok(false)
        );

        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&active_mon_player_1),
            Ok(false)
        );
        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&active_mon_player_2),
            Ok(false)
        );
        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&inactive_mon_player_1),
            Ok(true)
        );
        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&inactive_mon_player_2),
            Ok(false)
        );
    }

    #[test]
    fn is_same_with_state_references_only() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let state = BattleState {
            field: Field {
                max_side_length: 1,
                sides: Vec::from_iter([
                    Side {
                        players: BTreeMap::from_iter([("player-1".to_owned(), Player::default())]),
                        active: Vec::from_iter([Some(MonBattleAppearanceReference {
                            player: "player-1".to_owned(),
                            mon_index: 0,
                            battle_appearance_index: 0,
                        })]),
                        ..Default::default()
                    },
                    Side {
                        players: BTreeMap::from_iter([("player-2".to_owned(), Player::default())]),
                        active: Vec::from_iter([Some(MonBattleAppearanceReference {
                            player: "player-2".to_owned(),
                            mon_index: 0,
                            battle_appearance_index: 0,
                        })]),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            },
            ..Default::default()
        };

        let active_mon_player_1 = MonBattleAppearanceReference {
            player: "player-1".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let active_mon_player_1 =
            Mon::new(MonReference::State(&active_mon_player_1), &state, &data);

        let inactive_mon_player_1 = MonBattleAppearanceReference {
            player: "player-1".to_owned(),
            mon_index: 1,
            battle_appearance_index: 0,
        };
        let inactive_mon_player_1 =
            Mon::new(MonReference::State(&inactive_mon_player_1), &state, &data);

        let active_mon_player_2 = MonBattleAppearanceReference {
            player: "player-2".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let active_mon_player_2 =
            Mon::new(MonReference::State(&active_mon_player_2), &state, &data);

        let inactive_mon_player_2 = MonBattleAppearanceReference {
            player: "player-2".to_owned(),
            mon_index: 1,
            battle_appearance_index: 0,
        };
        let inactive_mon_player_2 =
            Mon::new(MonReference::State(&inactive_mon_player_2), &state, &data);

        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&active_mon_player_1),
            Ok(true)
        );
        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&active_mon_player_2),
            Ok(false)
        );
        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&inactive_mon_player_1),
            Ok(false)
        );
        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&inactive_mon_player_2),
            Ok(false)
        );

        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&active_mon_player_1),
            Ok(false)
        );
        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&active_mon_player_2),
            Ok(false)
        );
        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&inactive_mon_player_1),
            Ok(true)
        );
        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&inactive_mon_player_2),
            Ok(false)
        );
    }

    #[test]
    fn is_same_with_mixed() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let state = BattleState {
            field: Field {
                max_side_length: 1,
                sides: Vec::from_iter([
                    Side {
                        players: BTreeMap::from_iter([("player-1".to_owned(), Player::default())]),
                        active: Vec::from_iter([Some(MonBattleAppearanceReference {
                            player: "player-1".to_owned(),
                            mon_index: 0,
                            battle_appearance_index: 0,
                        })]),
                        ..Default::default()
                    },
                    Side {
                        players: BTreeMap::from_iter([("player-2".to_owned(), Player::default())]),
                        active: Vec::from_iter([Some(MonBattleAppearanceReference {
                            player: "player-2".to_owned(),
                            mon_index: 0,
                            battle_appearance_index: 0,
                        })]),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            },
            ..Default::default()
        };

        let active_mon_player_1 = MonBattleData {
            side_position: Some(0),
            ..Default::default()
        };
        let active_mon_player_1 = Mon::new(
            MonReference::Battle {
                side: 0,
                player: "player-1".to_owned(),
                battle_data: &active_mon_player_1,
            },
            &state,
            &data,
        );

        let inactive_mon_player_1 = MonBattleData {
            ..Default::default()
        };
        let inactive_mon_player_1 = Mon::new(
            MonReference::Battle {
                side: 0,
                player: "player-1".to_owned(),
                battle_data: &inactive_mon_player_1,
            },
            &state,
            &data,
        );

        let active_mon_player_2 = MonBattleAppearanceReference {
            player: "player-2".to_owned(),
            mon_index: 0,
            battle_appearance_index: 0,
        };
        let active_mon_player_2 =
            Mon::new(MonReference::State(&active_mon_player_2), &state, &data);

        let inactive_mon_player_2 = MonBattleAppearanceReference {
            player: "player-2".to_owned(),
            mon_index: 1,
            battle_appearance_index: 0,
        };
        let inactive_mon_player_2 =
            Mon::new(MonReference::State(&inactive_mon_player_2), &state, &data);

        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&active_mon_player_1),
            Ok(true)
        );
        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&active_mon_player_2),
            Ok(false)
        );
        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&inactive_mon_player_1),
            Ok(false)
        );
        assert_matches::assert_matches!(
            active_mon_player_1.is_same(&inactive_mon_player_2),
            Ok(false)
        );

        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&active_mon_player_1),
            Ok(false)
        );
        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&active_mon_player_2),
            Ok(false)
        );
        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&inactive_mon_player_1),
            Ok(true)
        );
        assert_matches::assert_matches!(
            inactive_mon_player_1.is_same(&inactive_mon_player_2),
            Ok(false)
        );
    }
}
