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
        PlayerContext,
        Request,
        RequestType,
        TeamAction,
    },
    battler_error,
    common::{
        split_once_optional,
        Captures,
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
struct MoveChoice<'s> {
    pub name: &'s str,
    pub target: Option<usize>,
    pub mega: bool,
}

impl<'s> MoveChoice<'s> {
    /// Parses a new [`MoveChoice`] from a string.
    ///
    /// For example, `move Tackle, 2, mega` says to use the move Tackle against the Mon in position
    /// 2 while also Mega Evolving.
    ///
    /// The `move` prefix should already be trimmed off.
    pub fn new(data: &'s str) -> Result<MoveChoice<'s>, Error> {
        let args = data.split(',').map(|str| str.trim()).collect::<Vec<&str>>();
        let mut index = 0;
        let name = args
            .get(index)
            .wrap_error_with_message("missing move name")?;
        if name.is_empty() {
            return Err(battler_error!("missing move name"))?;
        }
        let mut choice = Self {
            name,
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
    id: String,
    name: String,
    side: usize,
    position: usize,
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

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn index(&self) -> usize {
        self.index
    }
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
        let active = vec![None; battle_type.active_per_side()];
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
        for mon in &context.player().mons {
            context.battle().registry.mon_mut(*mon)?.player = index;
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

    pub fn active_mon<'b>(context: &'b PlayerContext, slot: usize) -> Result<&'b Mon, Error> {
        let handle = context
            .player()
            .active
            .get(slot)
            .wrap_error_with_format(format_args!("player does not have any active slot {slot}"))?
            .wrap_error_with_format(format_args!(
                "player does not have an active Mon in slot {slot}"
            ))?;
        context.battle().registry.mon(handle)
    }

    pub fn active_mons<'b, 'd>(
        context: &'b PlayerContext<'b, 'd>,
    ) -> impl Iterator<Item = Result<&'b Mon, Error>> + Captures<'d> + 'b {
        context
            .player()
            .active
            .iter()
            .filter_map(move |active_slot| match active_slot {
                Some(mon_handle) => Some(context.battle().registry.mon(*mon_handle)),
                None => None,
            })
    }

    pub fn active_mon_handles<'b, 'd>(
        context: &'b PlayerContext<'b, 'd>,
    ) -> impl Iterator<Item = MonHandle> + Captures<'d> + 'b {
        context
            .player()
            .active
            .iter()
            .filter_map(|mon_handle| *mon_handle)
    }

    pub fn inactive_mons<'b, 'd>(
        context: &'b PlayerContext<'b, 'd>,
    ) -> impl Iterator<Item = Result<&'b Mon, Error>> + Captures<'d> + 'b {
        context
            .player()
            .mons
            .iter()
            .cloned()
            .filter(|mon_handle| !context.player().active.contains(&Some(*mon_handle)))
            .map(|mon_handle| context.battle().registry.mon(mon_handle))
    }

    pub fn mons<'b, 'd>(
        context: &'b PlayerContext<'b, 'd>,
    ) -> impl Iterator<Item = Result<&'b Mon, Error>> + Captures<'d> + 'b {
        context
            .player()
            .mons
            .iter()
            .cloned()
            .map(|mon_handle| context.battle().registry.mon(mon_handle))
    }

    pub fn switchable_mons<'b, 'd>(
        context: &'b PlayerContext<'b, 'd>,
    ) -> impl Iterator<Item = Result<&'b Mon, Error>> + Captures<'d> + 'b {
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
        let active_slot = Self::get_active_slot_for_next_choice(context, false)?;
        if active_slot >= context.player().active.len() {
            return match context.player().request_type() {
                Some(RequestType::Switch) => Err(battler_error!(
                    "you sent more switches than Mons that need to switch"
                )),
                _ => Err(battler_error!("you sent more choices than active Mons")),
            };
        }
        let active_mon = Self::active_mon(context, active_slot).wrap_error_with_format(
            format_args!("expected player to have active Mon in slot {active_slot}"),
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

        let target_mon = context.battle().registry.mon(target_mon_handle)?;
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
        player.choice.actions.push(Action::Switch(SwitchAction {
            instant: false,
            mon: target_mon_handle,
            position: active_mon_position,
        }));
        Ok(())
    }

    fn get_active_slot_for_next_choice(
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
        let active_index = Self::get_active_slot_for_next_choice(context, true)?;
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
            Some(RequestType::Turn) => return Err(battler_error!("you cannot move out of turn")),
            _ => (),
        }
        let choice = MoveChoice::new(data.wrap_error_with_message("missing move data")?)?;
        let active_slot = Self::get_active_slot_for_next_choice(context, false)?;
        if active_slot >= context.player().active.len() {
            return Err(battler_error!("you sent more choices than active Mons"));
        }
        let mon = Self::active_mon(context, active_slot).wrap_error_with_format(format_args!(
            "expected player to have active Mon in slot {active_slot}"
        ))?;
        todo!()
    }
}

#[cfg(test)]
mod move_choice_tests {
    use crate::{
        battle::player::MoveChoice,
        common::assert_error_message,
    };

    #[test]
    fn parses_move_target() {
        assert_eq!(
            MoveChoice::new("Tackle, 0"),
            Ok(MoveChoice {
                name: "Tackle",
                target: Some(0),
                mega: false
            })
        );
    }

    #[test]
    fn parses_move_target_mega() {
        assert_eq!(
            MoveChoice::new("Tackle, 0, mega"),
            Ok(MoveChoice {
                name: "Tackle",
                target: Some(0),
                mega: true,
            })
        );
    }

    #[test]
    fn parses_move_no_target() {
        assert_eq!(
            MoveChoice::new("Surf"),
            Ok(MoveChoice {
                name: "Surf",
                target: None,
                mega: false,
            })
        );
    }

    #[test]
    fn parses_move_mega() {
        assert_eq!(
            MoveChoice::new("Earthquake, mega"),
            Ok(MoveChoice {
                name: "Earthquake",
                target: None,
                mega: true,
            })
        );
    }

    #[test]
    fn fails_missing_name() {
        assert_error_message(MoveChoice::new(""), "missing move name");
    }
}
