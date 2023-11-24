use std::{
    cmp,
    mem,
};

use ahash::HashSetExt;
use itertools::Itertools;
use serde::{
    Deserialize,
    Serialize,
};
use zone_alloc::ElementRef;

use crate::{
    battle::{
        action::SwitchAction,
        Action,
        Battle,
        BattleRegistry,
        BattleType,
        Mon,
        MonHandle,
        MonTeamRequestData,
        MoveAction,
        MoveActionInput,
        PlayerContext,
        Request,
        RequestType,
        SwitchActionInput,
        TeamAction,
        TeamActionInput,
    },
    battler_error,
    common::{
        split_once_optional,
        Captures,
        Error,
        FastHashSet,
        Id,
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
pub struct ChoiceState {
    /// Can the choice be undone?
    pub undo_allowed: bool,
    /// Is the request fulfilled?
    pub fulfilled: bool,
    /// Actions associated with the choice.
    ///
    /// There should always be one action per active Mon for a request to be fulfilled (even if the
    /// Mon's action is "Pass").
    pub actions: Vec<Action>,
    /// Number of switch actions that must be made.
    pub forced_switches_left: usize,
    /// Number of pass actions that must be made.
    ///
    /// Passes are forced when the player does not have enough active Mons to replace all Mons that
    /// must be switched out. For example, if two Mons faint in the same turn (Doubles) but the
    /// player only has one Mon remaining.
    pub forced_passes_left: usize,
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
            forced_switches_left: 0,
            forced_passes_left: 0,
            switch_ins: FastHashSet::new(),
            mega: false,
        }
    }
}

/// A move choice for a single Mon on a single turn.
#[derive(Debug, PartialEq, Eq)]
struct MoveChoice {
    pub move_slot: usize,
    pub target: Option<isize>,
    pub mega: bool,
}

impl MoveChoice {
    /// Parses a new [`MoveChoice`] from a string.
    ///
    /// For example, `move 0, 2, mega` says to use the move in slot 0 against the Mon in position
    /// 2 while also Mega Evolving.
    ///
    /// The `move` prefix should already be trimmed off.
    pub fn new(data: &str) -> Result<MoveChoice, Error> {
        let args = data.split(',').map(|str| str.trim()).collect::<Vec<&str>>();
        let mut index = 0;
        let move_slot = args
            .get(index)
            .wrap_error_with_message("missing move slot")?;
        let move_slot = move_slot
            .parse()
            .wrap_error_with_message("invalid move slot")?;
        let mut choice = Self {
            move_slot,
            target: None,
            mega: false,
        };
        index += 1;

        if let Some(target) = args.get(index).and_then(|target| target.parse().ok()) {
            choice.target = Some(target);
            index += 1;
        }

        match args.get(index).cloned() {
            Some("mega") => {
                choice.mega = true;
            }
            Some(str) => return Err(battler_error!("invalid option in move choice: {str}")),
            None => (),
        }

        Ok(choice)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerRequestData {
    pub name: String,
    pub id: String,
    pub mons: Vec<MonTeamRequestData>,
}

/// A single player of a battle.
///
/// See [`PlayerData`] for an explanation of what a player represents.
pub struct Player {
    pub id: String,
    pub name: String,
    pub side: usize,
    pub position: usize,
    pub index: usize,
    pub choice: ChoiceState,
    pub request: Option<Request>,
    pub mons_left: usize,

    pub mons: Vec<MonHandle>,
    pub active: Vec<Option<MonHandle>>,
}

// Construction and initialization logic.
impl Player {
    /// Creates a new [`Player`]` instance from [`PlayerData`].
    pub fn new(
        data: PlayerData,
        side: usize,
        position: usize,
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
        let active = vec![None; battle_type.active_per_player()];
        Ok(Self {
            id: data.id,
            name: data.name,
            side,
            position,
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
        let side_index = context.player().side;
        for mon in &context.player().mons {
            let mut mon = context.battle().mon_mut(*mon)?;
            mon.player = index;
            mon.side = side_index;
        }
        Ok(())
    }
}

// Basic getters.
impl Player {
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

    pub fn request_type(&self) -> Option<RequestType> {
        self.request.as_ref().map(|req| req.request_type()).clone()
    }

    pub fn active_mon_handle<'b>(context: &'b PlayerContext, position: usize) -> Option<MonHandle> {
        context.player().active.get(position).cloned().flatten()
    }

    pub fn active_mon<'b>(
        context: &'b PlayerContext,
        position: usize,
    ) -> Result<ElementRef<'b, Mon>, Error> {
        context.battle().registry.mon(
            Self::active_mon_handle(context, position).wrap_error_with_format(format_args!(
                "player does not have an active Mon in position {position}"
            ))?,
        )
    }

    pub fn active_mons<'p, 's, 'c, 'b, 'd>(
        context: &'p PlayerContext<'s, 'c, 'b, 'd>,
    ) -> impl Iterator<Item = Result<ElementRef<'p, Mon>, Error>>
           + Captures<'d>
           + Captures<'b>
           + Captures<'c>
           + Captures<'s>
           + 'p {
        context
            .player()
            .active
            .iter()
            .filter_map(move |active| match active {
                Some(mon_handle) => Some(context.battle().mon(*mon_handle)),
                None => None,
            })
    }

    pub fn active_mon_handles<'p, 's, 'c, 'b, 'd>(
        context: &'p PlayerContext<'s, 'c, 'b, 'd>,
    ) -> impl Iterator<Item = MonHandle> + Captures<'d> + Captures<'b> + Captures<'c> + Captures<'s> + 'p
    {
        context
            .player()
            .active
            .iter()
            .filter_map(|mon_handle| *mon_handle)
    }

    pub fn inactive_mons<'p, 's, 'c, 'b, 'd>(
        context: &'p PlayerContext<'s, 'c, 'b, 'd>,
    ) -> impl Iterator<Item = Result<ElementRef<'p, Mon>, Error>>
           + Captures<'d>
           + Captures<'b>
           + Captures<'c>
           + Captures<'s>
           + 'p {
        context
            .player()
            .mons
            .iter()
            .cloned()
            .filter(|mon_handle| !context.player().active.contains(&Some(*mon_handle)))
            .map(|mon_handle| context.battle().mon(mon_handle))
    }

    pub fn mons<'p, 's, 'c, 'b, 'd>(
        context: &'p PlayerContext<'s, 'c, 'b, 'd>,
    ) -> impl Iterator<Item = Result<ElementRef<'p, Mon>, Error>>
           + Captures<'d>
           + Captures<'b>
           + Captures<'c>
           + Captures<'s>
           + 'p {
        context
            .player()
            .mons
            .iter()
            .cloned()
            .map(|mon_handle| context.battle().mon(mon_handle))
    }

    pub fn switchable_mons<'p, 's, 'c, 'b, 'd>(
        context: &'p PlayerContext<'s, 'c, 'b, 'd>,
    ) -> impl Iterator<Item = Result<ElementRef<'p, Mon>, Error>>
           + Captures<'d>
           + Captures<'b>
           + Captures<'c>
           + Captures<'s>
           + 'p {
        Self::mons(context).filter_ok(|mon| !mon.fainted)
    }

    pub fn request_data(context: &PlayerContext) -> Result<PlayerRequestData, Error> {
        let player = context.player();
        Ok(PlayerRequestData {
            name: player.name.clone(),
            id: player.id.clone(),
            mons: Self::mons(context)
                .map_ok(|mon| mon.team_request_data())
                .collect::<Result<_, _>>()?,
        })
    }

    /// Is the player's choice done?
    pub fn choice_done(context: &PlayerContext) -> bool {
        let player = context.player();
        match player.request_type() {
            None => true,
            Some(RequestType::TeamPreview) => {
                player.choice.actions.len() >= Self::picked_team_size(context)
            }
            _ => {
                if player.choice.forced_switches_left > 0 {
                    return false;
                }
                player.choice.actions.len() >= player.active.len()
            }
        }
    }
}

// Battle logic.
impl Player {
    /// Makes a new request on the player.
    pub fn make_request(&mut self, request: Request) {
        self.request = Some(request);
    }

    /// Clears any active request.
    pub fn clear_request(&mut self) {
        self.request = None;
    }

    /// Clears any active choice.
    pub fn clear_choice(context: &mut PlayerContext) {
        let mut choice = ChoiceState::new();
        if let Some(RequestType::Switch) = context.player().request_type() {
            let can_switch_out = Self::active_mons(context)
                .filter_ok(|mon| mon.needs_switch)
                .count();
            let can_switch_in = Self::inactive_mons(context)
                .filter_ok(|mon| !mon.fainted)
                .count();
            let switches = can_switch_out.min(can_switch_in);
            let passes = can_switch_out - switches;
            choice.forced_switches_left = switches;
            choice.forced_passes_left = passes;
        }
        context.player_mut().choice = choice;
    }

    /// Takes the choice from the player, resetting it in the process.
    pub fn take_choice(&mut self) -> ChoiceState {
        mem::replace(&mut self.choice, ChoiceState::new())
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
        match player.request_type() {
            Some(RequestType::TeamPreview) => (),
            _ => return Err(battler_error!("you are not in a team preview phase")),
        }

        let picked_team_size = Self::picked_team_size(context);
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
                .push(Action::Team(TeamAction::new(TeamActionInput {
                    mon: mon_handle,
                    index: i,
                    priority: -(i as i32),
                })))
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
            return Self::emit_choice_error(
                context,
                battler_error!("you cannot do anything: {reason}"),
            );
        }

        if !player.choice.undo_allowed {
            return Self::emit_choice_error(
                context,
                battler_error!("player choice cannot be undone"),
            );
        }

        Self::clear_choice(context);

        for choice in input.split(";").map(|str| str.trim()) {
            let (choice, data) = split_once_optional(choice, " ");
            match choice {
                "team" => match Self::choose_team(context, data) {
                    Err(error) => {
                        return Self::emit_choice_error(
                            context,
                            Error::wrap("team preview choice failed", error),
                        );
                    }
                    _ => (),
                },
                "switch" => match Self::choose_switch(context, data) {
                    Err(error) => {
                        return Self::emit_choice_error(
                            context,
                            Error::wrap("cannot switch", error),
                        );
                    }
                    _ => (),
                },
                "move" => match Self::choose_move(context, data) {
                    Err(error) => {
                        return Self::emit_choice_error(context, Error::wrap("cannot move", error))
                    }
                    _ => (),
                },
                _ => {
                    return Self::emit_choice_error(
                        context,
                        battler_error!("unrecognized choice: {choice}"),
                    )
                }
            }
        }

        if !Self::choice_done(context) {
            return Self::emit_choice_error(
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

    fn choose_switch(context: &mut PlayerContext, data: Option<&str>) -> Result<(), Error> {
        match context.player().request_type() {
            Some(RequestType::Turn | RequestType::Switch) => (),
            _ => return Err(battler_error!("you cannot switch out of turn")),
        };
        let active_position = Self::get_active_position_for_next_choice(context, false)?;
        if active_position >= context.player().active.len() {
            return match context.player().request_type() {
                Some(RequestType::Switch) => Err(battler_error!(
                    "you sent more switches than Mons that need to switch"
                )),
                _ => Err(battler_error!("you sent more choices than active Mons")),
            };
        }
        let active_mon = Self::active_mon(context, active_position).wrap_error_with_format(
            format_args!("expected player to have active Mon in position {active_position}"),
        )?;
        let active_mon_position = active_mon.position;
        let data = data.wrap_error_with_message("you must select a Mon to switch in")?;
        let slot = data
            .parse::<usize>()
            .wrap_error_with_message("switch argument is not an integer")?;

        let target_mon_handle = context
            .player()
            .mons
            .get(slot)
            .cloned()
            .wrap_error_with_format(format_args!(
                "you do not have a Mon in slot {slot} to switch to"
            ))?;
        if context.player().active.contains(&Some(target_mon_handle)) {
            return Err(battler_error!("you cannot switch to an active Mon"));
        }
        if context.player().choice.switch_ins.contains(&slot) {
            return Err(battler_error!(
                "the Mon in slot {slot} can only switch in once"
            ));
        }

        let target_mon = context.battle().mon(target_mon_handle)?;
        if target_mon.fainted {
            return Err(battler_error!("you cannot switch to a fainted Mon"));
        }

        match context.player().request_type() {
            Some(RequestType::Turn) => {
                if active_mon.trapped {
                    return Err(battler_error!("the active Mon is trapped"));
                }
            }
            Some(RequestType::Switch) => {
                let player = context.player_mut();
                if player.choice.forced_switches_left == 0 {
                    return Err(battler_error!("player switched too many Mons"));
                }
                player.choice.forced_switches_left -= 1;
            }
            _ => (),
        }

        let player = context.player_mut();
        player.choice.switch_ins.insert(slot);
        player
            .choice
            .actions
            .push(Action::Switch(SwitchAction::new(SwitchActionInput {
                instant: false,
                mon: target_mon_handle,
                position: active_mon_position,
            })));
        Ok(())
    }

    fn get_active_position_for_next_choice(
        context: &mut PlayerContext,
        pass: bool,
    ) -> Result<usize, Error> {
        // Choices generate a single action, so there should be once choice for each active Mon.
        let mut next_mon = context.player().choice.actions.len();
        if !pass {
            match context.player().request_type() {
                Some(RequestType::Turn) => {
                    while context.player().active.get(next_mon).is_some_and(|mon| {
                        mon.is_some_and(|mon| {
                            context
                                .battle()
                                .registry
                                .mon(mon)
                                .is_ok_and(|mon| mon.fainted)
                        })
                    }) {
                        Self::choose_pass(context)?;
                        next_mon += 1;
                    }
                }
                Some(RequestType::Switch) => {
                    while context.player().active.get(next_mon).is_some_and(|mon| {
                        mon.is_some_and(|mon| {
                            context
                                .battle()
                                .registry
                                .mon(mon)
                                .is_ok_and(|mon| !mon.needs_switch)
                        })
                    }) {
                        Self::choose_pass(context)?;
                        next_mon += 1;
                    }
                }
                _ => (),
            }
        }
        Ok(next_mon)
    }

    fn choose_pass(context: &mut PlayerContext) -> Result<(), Error> {
        let active_index = Self::get_active_position_for_next_choice(context, true)?;
        let mon = Self::active_mon(context, active_index)?;
        match context.player().request_type() {
            Some(RequestType::Switch) => {
                if context.player().choice.forced_passes_left == 0 {
                    return Err(battler_error!(
                        "cannot pass: you must select a Mon to replace {}",
                        mon.name
                    ));
                }
                context.player_mut().choice.forced_passes_left -= 1;
            }
            Some(RequestType::Turn) => {
                if !mon.fainted {
                    return Err(battler_error!(
                        "cannot pass: your {} must make a move or switch",
                        mon.name
                    ));
                };
            }
            _ => {
                return Err(battler_error!(
                    "cannot pass: only a move or switch can be passed"
                ));
            }
        };
        context.player_mut().choice.actions.push(Action::Pass);
        Ok(())
    }

    fn choose_move(context: &mut PlayerContext, data: Option<&str>) -> Result<(), Error> {
        match context.player().request_type() {
            Some(RequestType::Turn) => (),
            _ => return Err(battler_error!("you cannot move out of turn")),
        }
        let choice = MoveChoice::new(data.wrap_error_with_message("missing move choice")?)?;
        let active_position = Self::get_active_position_for_next_choice(context, false)?;
        if active_position >= context.player().active.len() {
            return Err(battler_error!("you sent more choices than active Mons"));
        }
        let mon_handle = Self::active_mon_handle(context, active_position).wrap_error_with_format(
            format_args!("expected an active Mon in position {active_position}"),
        )?;

        // This becomes our new context for the rest of the choice.
        let mut context = context
            .mon_context(mon_handle)
            .wrap_error_with_format(format_args!(
                "expected Mon to exist for handle {mon_handle}"
            ))?;

        let request = Mon::move_request(&context)?;
        let move_slot =
            request
                .moves
                .get(choice.move_slot)
                .wrap_error_with_format(format_args!(
                    "{} does not have a move in slot {}",
                    context.mon().name,
                    choice.move_slot
                ))?;
        let mut move_id = move_slot.id.clone();
        let mov = context
            .battle()
            .dex
            .moves
            .get_by_id(&move_id)
            .into_result()
            .wrap_error_with_format(format_args!("expected move id {} to exist", move_slot.id))?;
        let target_required = context.battle().format.battle_type.active_per_player() > 1;
        match (mov.data.target.choosable(), choice.target) {
            (true, None) => {
                if target_required {
                    return Err(battler_error!("{} requires a target", mov.data.name));
                }
            }
            (true, Some(target)) => {
                if target == 0 && target_required {
                    return Err(battler_error!("target cannot be 0"));
                }
                let target_side = if target > 0 {
                    context.foe_side().index
                } else {
                    context.side().index
                };
                let target_position = target.abs() as usize;
                let target_position = target_position - 1;
                if !Mon::relative_location_of_target(&context, target_side, target_position)
                    .map_or(false, |relative_location| {
                        mov.data.target.valid_target(relative_location)
                    })
                {
                    return Err(battler_error!("invalid target for {}", mov.data.name));
                }
            }
            (false, Some(_)) => {
                return Err(battler_error!(
                    "you cannot choose a target for {}",
                    mov.data.name
                ))
            }
            _ => (),
        }

        let moves = Mon::moves(&context)?;
        let locked_move = Mon::locked_move(&context)?;
        if let Some(locked_move) = locked_move {
            let locked_move_target = context.mon().last_move_target;
            context
                .player_mut()
                .choice
                .actions
                .push(Action::Move(MoveAction::new(MoveActionInput {
                    id: Id::from(locked_move),
                    mon: mon_handle,
                    target: locked_move_target,
                    mega: false,
                })));
            // Locked move, the Mon cannot do anything else.
            return Ok(());
        } else if moves.is_empty() {
            // No moves, the Mon must use Struggle.
            move_id = Id::from_known("struggle");
        } else {
            // Make sure the selected move is not disabled.
            let move_slot = moves
                .iter()
                .find(|mov| mov.id == move_id)
                .wrap_error_with_format(format_args!(
                    "expected move {move_id} to be in Mon's moveset"
                ))?;
            if move_slot.disabled {
                return Err(battler_error!(
                    "{}'s {} is disabled",
                    context.mon().name,
                    mov.data.name
                ));
            }
        }

        // Mega evoution.
        if choice.mega && !context.mon().can_mega_evo {
            return Err(battler_error!("{} cannot mega evolve", context.mon().name));
        }
        if choice.mega && context.player().choice.mega {
            return Err(battler_error!("you can only mega evolve once per battle"));
        }

        context
            .player_mut()
            .choice
            .actions
            .push(Action::Move(MoveAction::new(MoveActionInput {
                id: move_id,
                mon: mon_handle,
                target: choice.target,
                mega: choice.mega,
            })));

        if choice.mega {
            context.player_mut().choice.mega = true;
        }

        Ok(())
    }
}

#[cfg(test)]
mod move_choice_tests {
    use crate::{
        battle::player::MoveChoice,
        common::assert_error_message_contains,
    };

    #[test]
    fn parses_move_target() {
        assert_eq!(
            MoveChoice::new("0, 0"),
            Ok(MoveChoice {
                move_slot: 0,
                target: Some(0),
                mega: false
            })
        );
    }

    #[test]
    fn parses_move_target_mega() {
        assert_eq!(
            MoveChoice::new("1, 0, mega"),
            Ok(MoveChoice {
                move_slot: 1,
                target: Some(0),
                mega: true,
            })
        );
    }

    #[test]
    fn parses_move_no_target() {
        assert_eq!(
            MoveChoice::new("2"),
            Ok(MoveChoice {
                move_slot: 2,
                target: None,
                mega: false,
            })
        );
    }

    #[test]
    fn parses_move_mega() {
        assert_eq!(
            MoveChoice::new("3, mega"),
            Ok(MoveChoice {
                move_slot: 3,
                target: None,
                mega: true,
            })
        );
    }

    #[test]
    fn fails_empty_string() {
        assert_error_message_contains(MoveChoice::new(""), "invalid move slot");
    }
}
