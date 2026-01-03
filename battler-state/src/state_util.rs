use alloc::{
    boxed::Box,
    vec::Vec,
};
use core::str::FromStr;

use anyhow::{
    Error,
    Result,
};
use battler::{
    Boost,
    BoostTable,
    DataStoreByName,
    Type,
};

use crate::{
    BattleState,
    Mon,
    MonBattleAppearance,
    MonBattleAppearanceReference,
    Player,
    Side,
};

/// The weather on the field.
pub fn field_weather<'s>(state: &'s BattleState) -> Option<&'s str> {
    state.field.weather.as_ref().map(|s| s.as_str())
}

/// The terrain on the field.
pub fn field_terrain<'s>(state: &'s BattleState) -> Option<&'s str> {
    state
        .field
        .conditions
        .keys()
        .map(|s| s.as_str())
        .find(|name| name.ends_with("Terrain"))
}

/// The conditions on the field.
pub fn field_conditions<'s>(state: &'s BattleState) -> impl Iterator<Item = &'s str> {
    state
        .field
        .conditions
        .keys()
        .map(|s| s.as_str())
        .filter(|s| !s.ends_with("Terrain"))
}

/// A side of a battle.
pub fn side_or_else<'s>(state: &'s BattleState, side: usize) -> Result<&'s Side> {
    state
        .field
        .sides
        .get(side)
        .ok_or_else(|| Error::msg("side not found"))
}

/// A side of a battle.
pub fn side<'s>(state: &'s BattleState, side: usize) -> Option<&'s Side> {
    side_or_else(state, side).ok()
}

/// The conditions on a side of a battle.
pub fn side_conditions<'s>(
    state: &'s BattleState,
    side: usize,
) -> Result<impl Iterator<Item = &'s str>> {
    Ok(side_or_else(state, side)?
        .conditions
        .keys()
        .map(|s| s.as_str()))
}

fn side_and_player_or_else<'s>(
    state: &'s BattleState,
    player: &str,
) -> Result<(usize, &'s Player)> {
    state
        .field
        .sides
        .iter()
        .enumerate()
        .flat_map(|(i, side)| side.players.get(player).map(|player| (i, player)))
        .next()
        .ok_or_else(|| Error::msg("player not found"))
}

/// The side for a Mon.
pub fn side_for_mon(state: &BattleState, mon: &MonBattleAppearanceReference) -> Result<usize> {
    side_and_player_or_else(state, &mon.player).map(|(side, _)| side)
}

/// The side for a player.
pub fn side_for_player(state: &BattleState, player: &str) -> Result<usize> {
    side_and_player_or_else(state, &player).map(|(side, _)| side)
}

/// A player on a side.
pub fn player_or_else<'s>(state: &'s BattleState, player: &str) -> Result<&'s Player> {
    side_and_player_or_else(state, player).map(|(_, player)| player)
}

/// A player on a side.
pub fn player<'s>(state: &'s BattleState, player: &str) -> Option<&'s Player> {
    player_or_else(state, player).ok()
}

/// A Mon owned by a player.
pub fn mon_or_else<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<&'s Mon> {
    player_or_else(state, &mon.player)?
        .mons
        .get(mon.mon_index)
        .ok_or_else(|| Error::msg("mon not found"))
}

/// A Mon owned by a player.
pub fn mon<'s>(state: &'s BattleState, mon: &MonBattleAppearanceReference) -> Option<&'s Mon> {
    mon_or_else(state, mon).ok()
}

/// The battle appearance of a Mon.
pub fn mon_battle_appearance_or_else<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<&'s MonBattleAppearance> {
    Ok(mon_or_else(state, &mon)?
        .battle_appearances
        .get(mon.battle_appearance_index)
        .ok_or_else(|| Error::msg("mon battle appearance not found"))?
        .primary())
}

/// The battle appearance of a Mon.
pub fn mon_battle_appearance<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Option<&'s MonBattleAppearance> {
    mon_battle_appearance_or_else(state, mon).ok()
}

/// The level of a Mon.
pub fn mon_level(state: &BattleState, mon: &MonBattleAppearanceReference) -> Result<Option<u64>> {
    Ok(mon_battle_appearance_or_else(state, mon)?
        .level
        .known()
        .cloned())
}

/// The health of a Mon.
pub fn mon_health(
    state: &BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<Option<(u64, u64)>> {
    Ok(mon_battle_appearance_or_else(state, mon)?
        .health
        .known()
        .cloned())
}

/// The status of a Mon.
pub fn mon_status<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<Option<&'s str>> {
    Ok(mon_battle_appearance_or_else(state, mon)?
        .status
        .known()
        .map(|s| s.as_str()))
}

/// The ability of a Mon.
pub fn mon_ability<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<Option<&'s str>> {
    if let Some(ability) = mon_or_else(state, mon)?.volatile_data.ability.as_ref() {
        return Ok(Some(ability));
    }
    Ok(mon_battle_appearance_or_else(state, mon)?
        .ability
        .known()
        .map(|s| s.as_str()))
}

/// The moves of a Mon.
///
/// May be volatile moves.
pub fn mon_moves<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
    include_possible: bool,
) -> Result<Box<dyn Iterator<Item = &'s str> + 's>> {
    let volatile_moves = &mon_or_else(state, mon)?.volatile_data.moves;
    if !volatile_moves.is_empty() {
        Ok(Box::new(volatile_moves.iter().map(|s| s.as_str())))
    } else {
        if include_possible {
            Ok(Box::new(mon_all_possible_non_volatile_moves(state, mon)?))
        } else {
            Ok(Box::new(mon_known_non_volatile_moves(state, mon)?))
        }
    }
}

/// The known non-volatile moves of a Mon.
pub fn mon_known_non_volatile_moves<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<impl Iterator<Item = &'s str> + 's> {
    Ok(mon_battle_appearance_or_else(state, mon)?
        .moves
        .known()
        .iter()
        .map(|s| s.as_str()))
}

/// All known and possible non-volatile moves of a Mon.
pub fn mon_all_possible_non_volatile_moves<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<impl Iterator<Item = &'s str> + 's> {
    let moves = &mon_battle_appearance_or_else(state, mon)?.moves;
    Ok(moves
        .known()
        .iter()
        .chain(moves.possible_values())
        .map(|s| s.as_str()))
}

/// The item of a Mon.
pub fn mon_item<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<Option<&'s str>> {
    Ok(mon_battle_appearance_or_else(state, mon)?
        .item
        .known()
        .map(|s| s.as_str()))
}

/// The species of a Mon.
pub fn mon_species<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<&'s str> {
    let mon = &mon_or_else(state, mon)?;
    if let Some((appearance, _)) = &mon.volatile_data.transformed {
        Ok(&appearance.species)
    } else if let Some(species) = &mon.volatile_data.forme_change {
        Ok(&species)
    } else {
        Ok(&mon.physical_appearance.species)
    }
}

/// The types of a Mon.
///
/// Looks up species type if types are not known from the battle.
pub fn mon_types(
    state: &BattleState,
    mon: &MonBattleAppearanceReference,
    data: &dyn DataStoreByName,
) -> Result<Vec<Type>> {
    let volatile_types = &mon_or_else(state, mon)?.volatile_data.types;
    if !volatile_types.is_empty() {
        return volatile_types
            .iter()
            .map(|typ| Type::from_str(&typ).map_err(Error::msg))
            .collect::<Result<Vec<_>>>();
    }

    let species = data
        .get_species_by_name(mon_species(state, mon)?)?
        .ok_or_else(|| Error::msg("species not found"))?;
    let mut types = Vec::from_iter([species.primary_type]);
    if let Some(typ) = species.secondary_type {
        types.push(typ);
    }
    Ok(types)
}

/// The stat boosts of a Mon.
pub fn mon_boosts(state: &BattleState, mon: &MonBattleAppearanceReference) -> Result<BoostTable> {
    Ok(mon_or_else(state, mon)?
        .volatile_data
        .stat_boosts
        .iter()
        .fold(BoostTable::default(), |mut table, (boost, val)| {
            if let Ok(boost) = Boost::from_str(boost) {
                table.set(boost, (*val).min(i8::MAX as i64).max(i8::MIN as i64) as i8);
            }
            table
        }))
}

/// The conditions on a Mon.
pub fn mon_conditions<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<impl Iterator<Item = &'s str> + 's> {
    Ok(mon_or_else(state, mon)?
        .volatile_data
        .conditions
        .keys()
        .map(|s| s.as_str()))
}

/// The active position of a Mon.
pub fn mon_active_position<'s>(
    state: &'s BattleState,
    mon: &MonBattleAppearanceReference,
) -> Result<Option<usize>> {
    let side = side_for_mon(state, mon)?;
    let side = side_or_else(state, side)?;
    Ok(side
        .active
        .iter()
        .position(|active| active.as_ref().is_some_and(|active| active == mon)))
}

#[cfg(test)]
mod state_util_test {
    use alloc::{
        borrow::ToOwned,
        collections::{
            BTreeMap,
            BTreeSet,
            VecDeque,
        },
        vec::Vec,
    };

    use battler::{
        BoostTable,
        Type,
    };
    use battler_test_utils::static_local_data_store;

    use crate::{
        discovery::DiscoveryRequiredSet,
        state::{
            BattleState,
            ConditionData,
            Field,
            Mon,
            MonBattleAppearance,
            MonBattleAppearanceReference,
            MonPhysicalAppearance,
            MonVolatileData,
            Player,
            Side,
        },
        state_util::{
            field_conditions,
            field_terrain,
            field_weather,
            mon_ability,
            mon_active_position,
            mon_all_possible_non_volatile_moves,
            mon_boosts,
            mon_conditions,
            mon_health,
            mon_item,
            mon_known_non_volatile_moves,
            mon_level,
            mon_moves,
            mon_species,
            mon_status,
            mon_types,
            side_conditions,
        },
    };

    #[test]
    fn returns_field_weather() {
        let state = BattleState {
            field: Field {
                weather: Some("Rain".to_owned()),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(field_weather(&state), Some(weather) => {
            assert_eq!(weather, "Rain");
        });
    }

    #[test]
    fn returns_field_terrain() {
        let state = BattleState {
            field: Field {
                conditions: BTreeMap::from_iter([
                    ("Gravity".to_owned(), ConditionData::default()),
                    ("Grassy Terrain".to_owned(), ConditionData::default()),
                ]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(field_terrain(&state), Some(terrain) => {
            assert_eq!(terrain, "Grassy Terrain");
        });
    }

    #[test]
    fn returns_field_conditions() {
        let state = BattleState {
            field: Field {
                conditions: BTreeMap::from_iter([
                    ("Gravity".to_owned(), ConditionData::default()),
                    ("Grassy Terrain".to_owned(), ConditionData::default()),
                ]),
                ..Default::default()
            },
            ..Default::default()
        };
        pretty_assertions::assert_eq!(
            field_conditions(&state).collect::<Vec<_>>(),
            Vec::from_iter(["Gravity"])
        );
    }

    #[test]
    fn returns_side_conditions() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    conditions: BTreeMap::from_iter([
                        ("Light Screen".to_owned(), ConditionData::default()),
                        ("Reflect".to_owned(), ConditionData::default()),
                    ]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            side_conditions(&state, 0).map(|iter| iter.collect::<Vec<_>>()),
            Ok(conditions) => {
                pretty_assertions::assert_eq!(
                    conditions,
                    Vec::from_iter([
                        "Light Screen",
                        "Reflect",
                    ])
                );
            }
        );
    }

    #[test]
    fn returns_mon_level() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([MonBattleAppearance {
                                    level: 50.into(),
                                    ..Default::default()
                                }
                                .into()]),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_level(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }),
            Ok(Some(level)) => {
                assert_eq!(level, 50);
            }
        );
    }

    #[test]
    fn returns_mon_health() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([MonBattleAppearance {
                                    health: (75, 100).into(),
                                    ..Default::default()
                                }
                                .into()]),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_health(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }),
            Ok(Some(health)) => {
                assert_eq!(health, (75, 100));
            }
        );
    }

    #[test]
    fn returns_mon_status() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([MonBattleAppearance {
                                    status: "Burn".to_owned().into(),
                                    ..Default::default()
                                }
                                .into()]),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_status(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }),
            Ok(Some(status)) => {
                assert_eq!(status, "Burn");
            }
        );
    }

    #[test]
    fn returns_mon_ability() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([MonBattleAppearance {
                                    ability: "Intimidate".to_owned().into(),
                                    ..Default::default()
                                }
                                .into()]),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_ability(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }),
            Ok(Some(ability)) => {
                assert_eq!(ability, "Intimidate");
            }
        );
    }

    #[test]
    fn returns_mon_volatile_ability() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([MonBattleAppearance {
                                    ability: "Intimidate".to_owned().into(),
                                    ..Default::default()
                                }
                                .into()]),
                                volatile_data: MonVolatileData {
                                    ability: Some("Normalize".to_owned()),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_ability(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }),
            Ok(Some(ability)) => {
                assert_eq!(ability, "Normalize");
            }
        );
    }

    #[test]
    fn returns_mon_moves() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([MonBattleAppearance {
                                    moves: DiscoveryRequiredSet::new(
                                        ["Tackle".to_owned()],
                                        ["Scratch".to_owned()],
                                    ),
                                    ..Default::default()
                                }
                                .into()]),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_moves(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }, false).map(|iter| iter.collect::<Vec<_>>()),
            Ok(moves) => {
                pretty_assertions::assert_eq!(
                    moves,
                    Vec::from_iter([
                        "Tackle"
                    ]),
                );
            }
        );
        assert_matches::assert_matches!(
            mon_moves(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }, true).map(|iter| iter.collect::<Vec<_>>()),
            Ok(moves) => {
                pretty_assertions::assert_eq!(
                    moves,
                    Vec::from_iter([
                        "Tackle",
                        "Scratch",
                    ]),
                );
            }
        );
    }

    #[test]
    fn returns_mon_volatile_moves() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([MonBattleAppearance {
                                    moves: DiscoveryRequiredSet::new(
                                        ["Tackle".to_owned()],
                                        ["Scratch".to_owned()],
                                    ),
                                    ..Default::default()
                                }
                                .into()]),
                                volatile_data: MonVolatileData {
                                    moves: BTreeSet::from_iter(["Pound".to_owned()]),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_moves(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }, true).map(|iter| iter.collect::<Vec<_>>()),
            Ok(moves) => {
                pretty_assertions::assert_eq!(
                    moves,
                    Vec::from_iter([
                        "Pound"
                    ]),
                );
            }
        );
        assert_matches::assert_matches!(
            mon_known_non_volatile_moves(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }).map(|iter| iter.collect::<Vec<_>>()),
            Ok(moves) => {
                pretty_assertions::assert_eq!(
                    moves,
                    Vec::from_iter([
                        "Tackle",
                    ]),
                );
            }
        );
        assert_matches::assert_matches!(
            mon_all_possible_non_volatile_moves(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }).map(|iter| iter.collect::<Vec<_>>()),
            Ok(moves) => {
                pretty_assertions::assert_eq!(
                    moves,
                    Vec::from_iter([
                        "Tackle",
                        "Scratch",
                    ]),
                );
            }
        );
    }

    #[test]
    fn returns_mon_item() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([MonBattleAppearance {
                                    item: "Quick Claw".to_owned().into(),
                                    ..Default::default()
                                }
                                .into()]),
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_item(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }),
            Ok(Some(item)) => {
                assert_eq!(item, "Quick Claw");
            }
        );
    }

    #[test]
    fn returns_mon_species() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([
                                    MonBattleAppearance::default().into(),
                                ]),
                                physical_appearance: MonPhysicalAppearance {
                                    species: "Bulbasaur".to_owned(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_species(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }),
            Ok(species) => {
                assert_eq!(species, "Bulbasaur");
            }
        );
    }

    #[test]
    fn returns_mon_volatile_species() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([
                                    MonBattleAppearance::default().into(),
                                ]),
                                physical_appearance: MonPhysicalAppearance {
                                    species: "Bulbasaur".to_owned(),
                                    ..Default::default()
                                },
                                volatile_data: MonVolatileData {
                                    forme_change: Some("Bulbasaur-Alt".to_owned()),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_species(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }),
            Ok(species) => {
                assert_eq!(species, "Bulbasaur-Alt");
            }
        );
    }

    #[test]
    fn returns_mon_transformed_species() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([
                                    MonBattleAppearance::default().into(),
                                ]),
                                physical_appearance: MonPhysicalAppearance {
                                    species: "Bulbasaur".to_owned(),
                                    ..Default::default()
                                },
                                volatile_data: MonVolatileData {
                                    forme_change: Some("Bulbasaur-Alt".to_owned()),
                                    transformed: Some((
                                        MonPhysicalAppearance {
                                            species: "Charmander".to_owned(),
                                            ..Default::default()
                                        },
                                        MonBattleAppearanceReference::default(),
                                    )),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_species(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }),
            Ok(species) => {
                assert_eq!(species, "Charmander");
            }
        );
    }

    #[test]
    fn returns_mon_types() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([
                                    MonBattleAppearance::default().into(),
                                ]),
                                physical_appearance: MonPhysicalAppearance {
                                    species: "Bulbasaur".to_owned(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_types(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }, static_local_data_store()),
            Ok(types) => {
                pretty_assertions::assert_eq!(
                    types,
                    Vec::from_iter([
                        Type::Grass,
                        Type::Poison,
                    ])
                );
            }
        );
    }

    #[test]
    fn returns_mon_volatile_types() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([
                                    MonBattleAppearance::default().into(),
                                ]),
                                physical_appearance: MonPhysicalAppearance {
                                    species: "Bulbasaur".to_owned(),
                                    ..Default::default()
                                },
                                volatile_data: MonVolatileData {
                                    types: Vec::from_iter(["Bug".to_owned()]),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_types(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }, static_local_data_store()),
            Ok(types) => {
                pretty_assertions::assert_eq!(
                    types,
                    Vec::from_iter([
                        Type::Bug,
                    ])
                );
            }
        );
    }

    #[test]
    fn returns_mon_boosts() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([
                                    MonBattleAppearance::default().into(),
                                ]),
                                volatile_data: MonVolatileData {
                                    stat_boosts: BTreeMap::from_iter([
                                        ("atk".to_owned(), -6),
                                        ("def".to_owned(), 1),
                                        ("eva".to_owned(), 2),
                                    ]),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_boosts(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }),
            Ok(boosts) => {
                pretty_assertions::assert_eq!(
                    boosts,
                    BoostTable {
                        atk: -6,
                        def: 1,
                        eva: 2,
                        ..Default::default()
                    }
                );
            }
        );
    }

    #[test]
    fn returns_mon_conditions() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([(
                        "player-1".to_owned(),
                        Player {
                            mons: Vec::from_iter([Mon {
                                battle_appearances: VecDeque::from_iter([
                                    MonBattleAppearance::default().into(),
                                ]),
                                volatile_data: MonVolatileData {
                                    conditions: BTreeMap::from_iter([(
                                        "Focus Energy".to_owned(),
                                        ConditionData::default(),
                                    )]),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }]),
                            ..Default::default()
                        },
                    )]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_matches::assert_matches!(
            mon_conditions(&state, &MonBattleAppearanceReference{
                player: "player-1".to_owned(),
                mon_index: 0,
                battle_appearance_index: 0,
            }).map(|iter| iter.collect::<Vec<_>>()),
            Ok(conditions) => {
                pretty_assertions::assert_eq!(
                    conditions,
                   Vec::from_iter(["Focus Energy"]),
                );
            }
        );
    }

    #[test]
    fn returns_mon_active_position() {
        let state = BattleState {
            field: Field {
                sides: Vec::from_iter([Side {
                    players: BTreeMap::from_iter([("player-1".to_owned(), Player::default())]),
                    active: Vec::from_iter([
                        None,
                        Some(MonBattleAppearanceReference {
                            player: "player-1".to_owned(),
                            mon_index: 0,
                            battle_appearance_index: 0,
                        }),
                    ]),
                    ..Default::default()
                }]),
                ..Default::default()
            },
            ..Default::default()
        };

        assert_matches::assert_matches!(
            mon_active_position(
                &state,
                &MonBattleAppearanceReference {
                    player: "player-1".to_owned(),
                    mon_index: 0,
                    battle_appearance_index: 0,
                }
            ),
            Ok(Some(1))
        );

        assert_matches::assert_matches!(
            mon_active_position(
                &state,
                &MonBattleAppearanceReference {
                    player: "player-1".to_owned(),
                    mon_index: 1,
                    battle_appearance_index: 0,
                }
            ),
            Ok(None)
        );
    }
}
