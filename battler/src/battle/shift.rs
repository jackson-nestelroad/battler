use alloc::vec::Vec;

use itertools::Itertools;

/// A shift of a player and/or Mon to a certain battle position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shift {
    /// Side index.
    pub side: usize,
    /// Player position (NOT index).
    pub player: usize,
    /// Target player position, if a shift is required.
    pub shift_player: Option<usize>,
    /// Mon to shift, if a shift is required.
    ///
    /// Tuple of `(source_active_position, target_active_position)`.
    pub shift_mon: Option<(usize, usize)>,
}

/// A snapshot of the side of a battle for shifting logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Side {
    /// Side index.
    pub index: usize,
    /// Players, indexed by position.
    pub players: Vec<Option<Player>>,
}

/// A snapshot of a player of a battle for shifting logic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    /// Active Mons, indexed by position.
    ///
    /// `true` indicates a Mon is active in the position.
    pub active: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Mon {
    side: usize,
    player: usize,
    active_position: usize,
    side_position: usize,
    side_position_mapped: usize,
}

fn calculate_best_shift(
    mons: &[Mon],
    active_per_player: usize,
    target_side_position: usize,
) -> Option<Shift> {
    // Determine which player owns the target position.
    let target_player_position = target_side_position / active_per_player;
    let target_active_position = target_side_position % active_per_player;

    // Sort Mons by distance to the target position and prepare to shift the closest one.
    let mon = mons
        .iter()
        .sorted_by_cached_key(|mon| mon.side_position.abs_diff(target_side_position))
        .next()?;

    let mut shift_player = None;
    let mut shift_mon = None;

    // If the player does not own the target position, the player must shift.
    if mon.player != target_player_position {
        shift_player = Some(target_player_position);
    }

    // The Mon must shift to be in the center.
    if mon.active_position != target_active_position {
        shift_mon = Some((mon.active_position, target_active_position));
    }

    // No shift required.
    if shift_player.is_none() && shift_mon.is_none() {
        return None;
    }

    Some(Shift {
        side: mon.side,
        player: mon.player,
        shift_player,
        shift_mon,
    })
}

/// Calculates shifts that should occur to ensure at least two opposing Mons are adjacent to one
/// another.
///
/// This logic is a generalization of what happens in Triples battles when two Mons remain on
/// opposite edges. The battle automatically shifts them to the center.
///
/// This logic is designed to also work for battles with more Mons per side and different adjacency
/// reaches. Additionally, entire players may need to shift positions in the case of complex Multi-
/// battles (where the center position is owned by a different player).
pub fn shift_to_ensure_adjacency(
    active_per_player: usize,
    adjacency_reach: usize,
    side_1: Side,
    side_2: Side,
) -> Vec<Shift> {
    let players_per_side = side_1.players.len().max(side_2.players.len());
    let side_length = players_per_side * active_per_player;
    let max_side_position = side_length - 1;

    // Create a list of active Mons from a side.
    let active_mons = |side: &Side, invert_side_position_mapped: bool| -> Vec<Mon> {
        side.players
            .iter()
            .enumerate()
            .filter_map(|(i, player)| player.as_ref().map(|player| (i, player)))
            .flat_map(|(player_position, player)| {
                player
                    .active
                    .iter()
                    .enumerate()
                    .take(active_per_player)
                    .filter_map(move |(i, active)| {
                        active.then(|| {
                            let side_position = player_position * active_per_player + i;
                            Mon {
                                side: side.index,
                                player: player_position,
                                active_position: i,
                                side_position,
                                side_position_mapped: if invert_side_position_mapped {
                                    max_side_position - side_position
                                } else {
                                    side_position
                                },
                            }
                        })
                    })
            })
            .collect()
    };

    // Create a list of active Mons for both sides.
    //
    // Map all active side positions to Side 1 coordinates. This makes the adjacency check very
    // easy (one-dimensional distance).
    let side_1_mons = active_mons(&side_1, false);
    let side_2_mons = active_mons(&side_2, true);

    // Check if any opposing active Mons are adjacent.
    for (a, b) in side_1_mons.iter().cartesian_product(side_2_mons.iter()) {
        if a.side_position_mapped.abs_diff(b.side_position_mapped) < adjacency_reach {
            return Vec::default();
        }
    }

    // If we need to shift Mons, we want to shift them to the center. If there are an even number of
    // Mons per side, there is no true center, but we still want Mons to be directly across from one
    // another. Round down on one side and round up on the other to ensure the centers are across
    // from each other.
    let side_1_center = max_side_position / 2;
    let side_2_center = max_side_position.div_ceil(2);

    let mut shifts = Vec::default();
    if let Some(shift) = calculate_best_shift(&side_1_mons, active_per_player, side_1_center) {
        shifts.push(shift);
    }
    if let Some(shift) = calculate_best_shift(&side_2_mons, active_per_player, side_2_center) {
        shifts.push(shift);
    }

    shifts
}

#[cfg(test)]
mod shift_test {
    use alloc::vec::Vec;

    use crate::battle::shift::{
        Player,
        Shift,
        Side,
        shift_to_ensure_adjacency,
    };

    #[test]
    fn shifts_mon_for_triples() {
        pretty_assertions::assert_eq!(
            shift_to_ensure_adjacency(
                3,
                2,
                Side {
                    index: 0,
                    players: Vec::from_iter([Some(Player {
                        active: Vec::from_iter([true, false, false]),
                    })])
                },
                Side {
                    index: 1,
                    players: Vec::from_iter([Some(Player {
                        active: Vec::from_iter([true, false, false]),
                    })])
                },
            ),
            Vec::from_iter([
                Shift {
                    side: 0,
                    player: 0,
                    shift_player: None,
                    shift_mon: Some((0, 1))
                },
                Shift {
                    side: 1,
                    player: 0,
                    shift_player: None,
                    shift_mon: Some((0, 1))
                },
            ])
        );
    }

    #[test]
    fn does_not_shift_adjacent_mon_in_triples() {
        pretty_assertions::assert_eq!(
            shift_to_ensure_adjacency(
                3,
                2,
                Side {
                    index: 0,
                    players: Vec::from_iter([Some(Player {
                        active: Vec::from_iter([true, false, false]),
                    })])
                },
                Side {
                    index: 1,
                    players: Vec::from_iter([Some(Player {
                        active: Vec::from_iter([false, true, false]),
                    })])
                },
            ),
            Vec::from_iter([])
        );
    }

    #[test]
    fn never_shifts_mons_in_doubles() {
        for (player_1_active, player_2_active) in [
            ([false, true], [false, true]),
            ([false, true], [true, false]),
            ([true, false], [false, true]),
            ([true, false], [true, false]),
            ([true, true], [true, false]),
        ] {
            pretty_assertions::assert_eq!(
                shift_to_ensure_adjacency(
                    2,
                    2,
                    Side {
                        index: 0,
                        players: Vec::from_iter([Some(Player {
                            active: Vec::from_iter(player_1_active),
                        })])
                    },
                    Side {
                        index: 1,
                        players: Vec::from_iter([Some(Player {
                            active: Vec::from_iter(player_2_active),
                        })])
                    },
                ),
                Vec::from_iter([])
            );
        }
    }

    #[test]
    fn shifts_mon_for_quadruples() {
        pretty_assertions::assert_eq!(
            shift_to_ensure_adjacency(
                4,
                2,
                Side {
                    index: 0,
                    players: Vec::from_iter([Some(Player {
                        active: Vec::from_iter([true, false, false, false]),
                    })])
                },
                Side {
                    index: 1,
                    players: Vec::from_iter([Some(Player {
                        active: Vec::from_iter([false, true, false, false]),
                    })])
                },
            ),
            Vec::from_iter([
                Shift {
                    side: 0,
                    player: 0,
                    shift_player: None,
                    shift_mon: Some((0, 1))
                },
                Shift {
                    side: 1,
                    player: 0,
                    shift_player: None,
                    shift_mon: Some((1, 2))
                },
            ])
        );

        // Extended adjacency reach prevents shifts.
        pretty_assertions::assert_eq!(
            shift_to_ensure_adjacency(
                4,
                3,
                Side {
                    index: 0,
                    players: Vec::from_iter([Some(Player {
                        active: Vec::from_iter([true, false, false, false]),
                    })])
                },
                Side {
                    index: 1,
                    players: Vec::from_iter([Some(Player {
                        active: Vec::from_iter([false, true, false, false]),
                    })])
                },
            ),
            Vec::from_iter([])
        );
    }

    #[test]
    fn shifts_players_for_3v3() {
        pretty_assertions::assert_eq!(
            shift_to_ensure_adjacency(
                1,
                2,
                Side {
                    index: 0,
                    players: Vec::from_iter([
                        Some(Player {
                            active: Vec::from_iter([true]),
                        }),
                        Some(Player {
                            active: Vec::from_iter([false]),
                        }),
                        Some(Player {
                            active: Vec::from_iter([false]),
                        })
                    ])
                },
                Side {
                    index: 1,
                    players: Vec::from_iter([
                        Some(Player {
                            active: Vec::from_iter([true]),
                        }),
                        Some(Player {
                            active: Vec::from_iter([false]),
                        }),
                        Some(Player {
                            active: Vec::from_iter([false]),
                        })
                    ])
                },
            ),
            Vec::from_iter([
                Shift {
                    side: 0,
                    player: 0,
                    shift_player: Some(1),
                    shift_mon: None,
                },
                Shift {
                    side: 1,
                    player: 0,
                    shift_player: Some(1),
                    shift_mon: None,
                },
            ])
        );
    }

    #[test]
    fn shifts_single_player_for_multi_doubles() {
        pretty_assertions::assert_eq!(
            shift_to_ensure_adjacency(
                2,
                2,
                Side {
                    index: 0,
                    players: Vec::from_iter([
                        Some(Player {
                            active: Vec::from_iter([true, true]),
                        }),
                        Some(Player {
                            active: Vec::from_iter([false, false]),
                        })
                    ])
                },
                Side {
                    index: 1,
                    players: Vec::from_iter([
                        Some(Player {
                            active: Vec::from_iter([true, false]),
                        }),
                        Some(Player {
                            active: Vec::from_iter([false, false]),
                        })
                    ])
                },
            ),
            Vec::from_iter([Shift {
                side: 1,
                player: 0,
                shift_player: Some(1),
                shift_mon: None,
            }])
        );

        // First player requires a Mon shift to the center.
        pretty_assertions::assert_eq!(
            shift_to_ensure_adjacency(
                2,
                2,
                Side {
                    index: 0,
                    players: Vec::from_iter([
                        Some(Player {
                            active: Vec::from_iter([true, false]),
                        }),
                        Some(Player {
                            active: Vec::from_iter([false, false]),
                        })
                    ])
                },
                Side {
                    index: 1,
                    players: Vec::from_iter([
                        Some(Player {
                            active: Vec::from_iter([true, false]),
                        }),
                        Some(Player {
                            active: Vec::from_iter([false, false]),
                        })
                    ])
                },
            ),
            Vec::from_iter([
                Shift {
                    side: 0,
                    player: 0,
                    shift_player: None,
                    shift_mon: Some((0, 1)),
                },
                Shift {
                    side: 1,
                    player: 0,
                    shift_player: Some(1),
                    shift_mon: None,
                },
            ])
        );
    }
}
