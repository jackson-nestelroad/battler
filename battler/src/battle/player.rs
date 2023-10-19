use std::{
    cmp,
    mem,
};

use ahash::HashSetExt;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        Action,
        Battle,
        BattleRegistry,
        BattleType,
        Mon,
        MonHandle,
        PlayerContext,
        Request,
        TeamAction,
    },
    battler_error,
    common::{
        split_once_optional,
        Error,
        FastHashSet,
        WrapResultError,
    },
    dex::Dex,
    teams::TeamData,
};

/// Data for a single player of a battle.
///
/// A player is exactly what it sounds like: a single participant in a battle. A player brings their
/// own team of Mons to the battle and is responsible for controlling their Mons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Team.
    pub team: TeamData,
}

/// What the player has chosen to happen in the current turn.
///
/// Shared state for multiple choices in a turn (think of a double battle) ensures that choices do
/// not overlap with one another in a conflicting way (such as switching a Mon in twice).
pub(crate) struct ChoiceState {
    /// Can the choice be undone?
    pub undo_allowed: bool,
    /// Is the request fulfilled?
    pub fulfilled: bool,
    /// Actions associated with the choice.
    pub actions: Vec<Action>,
    /// Mons chosen to switch in.
    pub switch_ins: FastHashSet<usize>,
    /// Did the Player choose to Mega Evolve?
    pub mega: bool,
}

impl ChoiceState {
    /// Creates a new [`ChoiceState`] instance, with valid initial state.
    pub fn new() -> Self {
        Self {
            undo_allowed: true,
            fulfilled: false,
            actions: Vec::new(),
            switch_ins: FastHashSet::new(),
            mega: false,
        }
    }
}

/// A single player of a battle.
///
/// See [`PlayerData`] for an explanation of what a player represents.
pub struct Player {
    id: String,
    name: String,
    side: usize,
    index: usize,
    choice: ChoiceState,
    request: Option<Request>,
    mons_left: usize,

    pub mons: Vec<MonHandle>,
    pub active: Vec<Option<MonHandle>>,
}

// Block for getters.
impl Player {
    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn side(&self) -> usize {
        self.side
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

impl Player {
    /// Creates a new [`Player`]` instance from [`PlayerData`].
    pub fn new(
        data: PlayerData,
        side: usize,
        battle_type: &BattleType,
        dex: &Dex,
        registry: &BattleRegistry,
    ) -> Result<Self, Error> {
        let mons = data
            .team
            .members
            .into_iter()
            .map(|mon_data| Ok(registry.register_mon(Mon::new(mon_data, dex)?)))
            .collect::<Result<Vec<_>, _>>()?;
        let active = vec![None; battle_type.active_per_side()];
        Ok(Self {
            id: data.id,
            name: data.name,
            side,
            index: usize::MAX,
            mons,
            choice: ChoiceState::new(),
            active,
            request: None,
            mons_left: 0,
        })
    }

    /// Sets the index of the player, so that the player can safely reference itself.
    pub(crate) fn set_index(context: &mut PlayerContext, index: usize) -> Result<(), Error> {
        context.player_mut().index = index;
        for mon in &context.player().mons {
            context.battle().registry.mon_mut(*mon)?.player = index;
        }
        Ok(())
    }

    /// Returns the active request for the player.
    pub fn active_request(&self) -> Option<Request> {
        if !self.choice.fulfilled {
            self.request.clone()
        } else {
            None
        }
    }

    /// Returns the number of Mons left on the Player.
    pub fn mons_left(&self) -> usize {
        self.mons_left
    }

    /// Makes a new request on the player.
    pub fn make_request(&mut self, request: Request) {
        self.request = Some(request);
    }

    /// Clears any active request.
    pub fn clear_request(&mut self) {
        self.request = None;
    }

    /// Clears any active choice.
    pub fn clear_choice(&mut self) {
        self.choice = ChoiceState::new();
    }

    /// Takes the choice from the player, resetting it in the process.
    pub(crate) fn take_choice(&mut self) -> ChoiceState {
        mem::replace(&mut self.choice, ChoiceState::new())
    }

    /// Is the player's choice done?
    pub fn choice_done(context: &PlayerContext) -> bool {
        let player = context.player();
        match &player.request {
            None => true,
            Some(request) => match request {
                Request::TeamPreview => {
                    player.choice.actions.len() >= Player::picked_team_size(context)
                }
                _ => false,
            },
        }
    }

    fn emit_choice_error(_: &mut PlayerContext, error: Error) -> Result<(), Error> {
        Err(error)
    }

    fn picked_team_size(context: &PlayerContext) -> usize {
        cmp::min(
            context.player().mons.len(),
            context
                .battle()
                .format
                .rules
                .numeric_rules
                .picked_team_size
                .map(|n| n as usize)
                .unwrap_or(usize::MAX),
        )
    }

    fn choose_team(context: &mut PlayerContext, input: Option<&str>) -> Result<(), Error> {
        let player = context.player_mut();
        match player.request {
            Some(Request::TeamPreview) => (),
            _ => return Err(battler_error!("you are not in a team preview phase")),
        }

        let picked_team_size = Player::picked_team_size(context);
        let selected: Vec<usize> = match input {
            // No input, automatically choose Mons.
            None => (0..picked_team_size).collect(),
            Some(input) => {
                let mut selected: Vec<usize> = input
                    .split(" ")
                    .map(|str| str.trim())
                    .map(|str| str.parse::<usize>())
                    .collect::<Result<_, _>>()
                    .wrap_error_with_message("invalid team preview selection")?;
                let selected_len = selected.len();
                if selected_len > picked_team_size {
                    // Too many Mons, truncate the list.
                    selected.truncate(picked_team_size);
                } else if selected_len < picked_team_size {
                    // Not enough Mons, automatically choose Mons that are not yet selected.
                    let mut next_position = 0;
                    for _ in selected_len..picked_team_size {
                        for i in next_position..context.player().mons.len() {
                            if !selected.contains(&i) {
                                selected.push(i);
                                next_position = i + 1;
                                break;
                            }
                        }
                    }
                }
                selected
            }
        };

        for (i, mon_index) in selected.iter().enumerate() {
            if mon_index >= &context.player().mons.len() {
                return Err(battler_error!("you do not have a Mon in slot {mon_index}"));
            }
            // `position` returns the position of the first `mon_index` in the user's input. If this
            // is not equal to the position we are currently validating, that means the same
            // `mon_index` appears earlier in the vector, so the input is invalid.
            if selected.iter().position(|i| i == mon_index) != Some(i) {
                return Err(battler_error!(
                    "the Mon in slot {mon_index} can only be selected once"
                ));
            }
        }

        // Add a single action for each selected Mon.
        for (i, mon_index) in selected.iter().enumerate() {
            let mon_handle = context.player().mons.get(*mon_index).cloned().unwrap();
            context
                .player_mut()
                .choice
                .actions
                .push(Action::Team(TeamAction {
                    mon: mon_handle,
                    index: i,
                    priority: -(i as i32),
                }))
        }

        Ok(())
    }

    /// Makes a choice on the player.
    pub fn make_choice(context: &mut PlayerContext, input: &str) -> Result<(), Error> {
        let player = context.player_mut();
        if player.request.is_none() {
            let reason = if context.battle().ended() {
                "the battle is over"
            } else {
                "no action requested"
            };
            return Player::emit_choice_error(
                context,
                battler_error!("you cannot do anything: {reason}"),
            );
        }

        if !player.choice.undo_allowed {
            return Player::emit_choice_error(
                context,
                battler_error!("player choice cannot be undone"),
            );
        }

        player.clear_choice();

        for choice in input.split(",").map(|str| str.trim()) {
            let (choice, data) = split_once_optional(choice, " ");
            match choice {
                "team" => match Player::choose_team(context, data) {
                    Err(error) => {
                        return Player::emit_choice_error(
                            context,
                            error.prefix("team preview choice failed"),
                        );
                    }
                    _ => (),
                },
                _ => {
                    return Player::emit_choice_error(
                        context,
                        battler_error!("unrecognized choice: {choice}"),
                    )
                }
            }
        }

        if !Player::choice_done(context) {
            return Player::emit_choice_error(
                context,
                battler_error!("incomplete choice: {input} - missing actions for Mons"),
            );
        }

        context.player_mut().choice.fulfilled = true;
        Ok(())
    }

    pub fn start_battle(&mut self) {
        self.mons_left = self.mons.len();
    }
}
