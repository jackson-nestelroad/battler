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
        BattleRegistry,
        BattleType,
        CoreBattle,
        EscapeAction,
        EscapeActionInput,
        ForfeitAction,
        LearnMoveAction,
        Mon,
        MonBattleRequestData,
        MonHandle,
        MoveAction,
        MoveActionInput,
        PlayerContext,
        Request,
        RequestType,
        SwitchAction,
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

/// Options for a wild [`Player`].
///
/// For use on [`PlayerType`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WildPlayerOptions {
    /// Are the Mons catchable?
    pub catchable: bool,
    /// Can other players escape?
    pub escapable: bool,
    /// Can this player escape?
    ///
    /// Important for scripted battles, where escaping moves (like Teleport) should not succeed.
    pub can_escape: bool,
}

impl Default for WildPlayerOptions {
    fn default() -> Self {
        Self {
            catchable: true,
            escapable: true,
            can_escape: true,
        }
    }
}

/// The type of the [`Player`], which controls some of the operations that can be done in the
/// battle.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PlayerType {
    /// A trainer in a competitive battle.
    #[default]
    #[serde(rename = "trainer")]
    Trainer,
    /// A wild Mon.
    ///
    /// If this type is used, each player should have exactly one Mon to emulate wild battles
    /// (where each wild Mon can escape separately). If a "wild" player has multiple Mons,
    /// switch-ins can occur (logged as "appearances").
    #[serde(rename = "wild")]
    Wild(WildPlayerOptions),
    /// The protagonist, who can gain experience.
    ///
    /// Only use this type when you intend to simulate single-player battles, where the player can
    /// gain experience. When you do not wish to simulate experience, simply use `Trainer`.
    #[serde(rename = "protagonist")]
    Protagonist,
}

impl PlayerType {
    /// Does the player gain experience points?
    pub fn gains_experience(&self) -> bool {
        match self {
            Self::Protagonist => true,
            _ => false,
        }
    }

    /// Are the player's Mons wild?
    pub fn wild(&self) -> bool {
        match self {
            Self::Wild(_) => true,
            _ => false,
        }
    }

    /// Are the player's Mons catchable?
    pub fn catchable(&self) -> bool {
        match self {
            Self::Wild(wild) => wild.catchable,
            _ => false,
        }
    }

    /// Can other players escape from this player?
    pub fn escapable(&self) -> bool {
        match self {
            Self::Wild(wild) => wild.escapable,
            _ => false,
        }
    }

    /// Can other players forfeit against this player?
    pub fn forfeitable(&self) -> bool {
        match self {
            Self::Trainer | Self::Protagonist => true,
            _ => false,
        }
    }

    /// Can this player escape?
    ///
    /// If true, other checks are performed before an escape succeeds. For instance, all foe players
    /// must be [`escapable`][`Self::escapable`].
    pub fn can_escape(&self) -> bool {
        match self {
            Self::Wild(wild) => wild.can_escape,
            Self::Protagonist => true,
            _ => false,
        }
    }

    /// Can this player forfeit?
    pub fn can_forfeit(&self) -> bool {
        match self {
            Self::Trainer | Self::Protagonist => true,
            _ => false,
        }
    }
}

/// Options for the player that are not specific to any player type.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PlayerOptions {
    /// If the player has affection mechanics enabled.
    #[serde(default)]
    pub has_affection: bool,
}

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
    /// Player type.
    #[serde(default)]
    pub player_type: PlayerType,
    #[serde(default)]
    pub player_options: PlayerOptions,
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

/// A choice to learn a move for a single Mon.
#[derive(Debug, PartialEq, Eq)]
struct LearnMoveChoice {
    pub forget_move_slot: usize,
}

impl LearnMoveChoice {
    pub fn new(data: &str) -> Result<LearnMoveChoice, Error> {
        let move_slot = data
            .trim()
            .parse()
            .wrap_error_with_message("invalid move slot")?;
        Ok(Self {
            forget_move_slot: move_slot,
        })
    }
}

/// Request data for a single player in a battle.
///
/// Contains all information for a player in a battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerBattleRequestData {
    pub name: String,
    pub id: String,
    pub side: usize,
    pub position: usize,
    pub mons: Vec<MonBattleRequestData>,
}

/// A single player of a battle.
///
/// See [`PlayerData`] for an explanation of what a player represents.
pub struct Player {
    pub id: String,
    pub name: String,
    pub player_type: PlayerType,
    pub player_options: PlayerOptions,
    pub side: usize,
    pub position: usize,
    pub index: usize,
    pub choice: ChoiceState,
    pub request: Option<Request>,
    pub mons_left: usize,

    pub mons: Vec<MonHandle>,
    active: Vec<Option<MonHandle>>,
    /// A mirror of the above list, but fainted Mons are not unset.
    ///
    /// This is helpful for locating and switching out fainted Mons.
    active_or_fainted: Vec<Option<MonHandle>>,

    pub escape_attempts: u16,
    pub escaped: bool,
}

// Construction and initialization logic.
impl Player {
    /// Creates a new player.
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
            .enumerate()
            .map(|(team_position, mon_data)| {
                Ok(registry.register_mon(Mon::new(mon_data, team_position, dex)?))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let active = vec![None; battle_type.active_per_player()];
        Ok(Self {
            id: data.id,
            name: data.name,
            player_type: data.player_type,
            player_options: data.player_options,
            side,
            position,
            index: usize::MAX,
            mons,
            choice: ChoiceState::new(),
            active: active.clone(),
            active_or_fainted: active,
            request: None,
            mons_left: 0,
            escape_attempts: 0,
            escaped: false,
        })
    }

    /// Sets the index of the player, so that the player can safely reference itself.
    pub(crate) fn set_index(context: &mut PlayerContext, index: usize) -> Result<(), Error> {
        context.player_mut().index = index;
        let side_index = context.player().side;
        for mon_handle in context.player().mons.clone() {
            let mon = context.mon_mut(mon_handle)?;
            mon.player = index;
            mon.side = side_index;
        }
        Ok(())
    }
}

// Basic getters.
impl Player {
    /// The active request for the player.
    pub fn active_request(&self) -> Option<Request> {
        if !self.choice.fulfilled {
            self.request.clone()
        } else {
            None
        }
    }

    /// The active request type for the player.
    pub fn request_type(&self) -> Option<RequestType> {
        self.request.as_ref().map(|req| req.request_type()).clone()
    }

    /// The total number of active positions for the player.
    pub fn total_active_positions(&self) -> usize {
        self.active.len()
    }

    /// Sets the active position.
    pub fn set_active_position(
        &mut self,
        position: usize,
        mon: Option<MonHandle>,
    ) -> Result<(), Error> {
        *self
            .active
            .get_mut(position)
            .wrap_error_with_format(format_args!(
                "mon cannot be in active position {position}"
            ))? = mon;
        if mon.is_some() {
            // Keep track of fainted Mons for switching.
            *self
                .active_or_fainted
                .get_mut(position)
                .wrap_error_with_format(format_args!(
                    "mon cannot be in active position {position}"
                ))? = mon;
        }
        Ok(())
    }

    /// The active [`MonHandle`] for the player's position.
    pub fn active_mon_handle(&self, position: usize) -> Option<MonHandle> {
        self.active.get(position).cloned().flatten()
    }

    /// Creates an iterator over all active Mons owned by the player.
    ///
    /// All Mons are guaranteed to be active and non-fainted. Their HP may be 0 if they have not
    /// fainted yet.
    pub fn active_mon_handles(&self) -> impl Iterator<Item = &MonHandle> {
        self.active
            .iter()
            .filter_map(|mon_handle| mon_handle.as_ref())
    }

    /// Creates an iterator over all active or fainted Mons owned by the player.
    ///
    /// Fainted Mons will continue to be associated with the active position until switched out.
    pub fn active_or_fainted_mon_handles(&self) -> impl Iterator<Item = &MonHandle> {
        self.active_or_fainted
            .iter()
            .filter_map(|mon_handle| mon_handle.as_ref())
    }

    /// Creates an iterator over all positions used by the player.
    pub fn field_positions(&self) -> impl Iterator<Item = (usize, Option<&MonHandle>)> {
        self.active
            .iter()
            .enumerate()
            .map(|(i, slot)| (i, slot.as_ref()))
    }

    /// Creates an iterator over all positions used by the player with an active or fainted Mon. See
    /// [`active_or_fainted_mon_handles`][`Self::active_or_fainted_mon_handles`].
    pub fn field_positions_with_active_or_fainted_mon(
        &self,
    ) -> impl Iterator<Item = (usize, &MonHandle)> {
        self.active_or_fainted
            .iter()
            .enumerate()
            .filter_map(|(i, slot)| slot.as_ref().and_then(|slot| Some((i, slot))))
    }

    /// Creates an iterator over all Mons.
    pub fn mon_handles(&self) -> impl Iterator<Item = &MonHandle> {
        self.mons.iter()
    }

    /// Creates an iterator over all inactive Mons.
    pub fn inactive_mon_handles<'p, 'c, 'b, 'd>(
        context: &'p PlayerContext<'_, 'c, 'b, 'd>,
    ) -> impl Iterator<Item = &'p MonHandle> + Captures<'d> + Captures<'b> + Captures<'c> {
        context.player().mons.iter().filter_map(|mon_handle| {
            context
                .mon(*mon_handle)
                .is_ok_and(|mon| !mon.active)
                .then_some(mon_handle)
        })
    }
    /// Creates an iterator over all Mons that can be switched in.
    pub fn switchable_mon_handles<'p, 'c, 'b, 'd>(
        context: &'p PlayerContext<'_, 'c, 'b, 'd>,
    ) -> impl Iterator<Item = &'p MonHandle> + Captures<'d> + Captures<'b> + Captures<'c> {
        Self::inactive_mon_handles(context)
            .filter(|mon_handle| context.mon(**mon_handle).is_ok_and(|mon| !mon.fainted))
    }

    /// Request data for the player in a battle.
    pub fn request_data(context: &mut PlayerContext) -> Result<PlayerBattleRequestData, Error> {
        let mon_handles = context.player().mon_handles().cloned().collect::<Vec<_>>();
        Ok(PlayerBattleRequestData {
            name: context.player().name.clone(),
            id: context.player().id.clone(),
            side: context.player().side,
            position: context.player().position,
            mons: mon_handles
                .into_iter()
                .map(|mon_handle| Mon::battle_request_data(&mut context.mon_context(mon_handle)?))
                .collect::<Result<_, _>>()?,
        })
    }

    /// Is the player's choice done?
    pub fn choice_done(context: &mut PlayerContext) -> Result<bool, Error> {
        match context.player().request_type() {
            None => Ok(true),
            Some(RequestType::TeamPreview) => {
                Ok(context.player().choice.actions.len() >= Self::picked_team_size(context))
            }
            Some(RequestType::LearnMove) => {
                Self::get_position_for_next_choice(context, false)?;
                Ok(context.player().choice.actions.len() >= context.player().mons.len())
            }
            _ => {
                if context.player().escaped || context.player().mons_left == 0 {
                    return Ok(true);
                }
                if context.player().choice.forced_switches_left > 0 {
                    return Ok(false);
                }
                // Choose passes for as many Mons as we can.
                Self::get_position_for_next_choice(context, false)?;
                Ok(context.player().choice.actions.len() >= context.player().active.len())
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

    /// Counts the number of Mons that must switch out.
    pub fn count_must_switch_out(context: &mut PlayerContext) -> usize {
        context
            .player()
            .active_or_fainted_mon_handles()
            .filter(|mon_handle| {
                context
                    .mon(**mon_handle)
                    .is_ok_and(|mon| mon.needs_switch.is_some())
            })
            .count()
    }

    /// Counts the number of Mons that can switch in.
    pub fn count_can_switch_in(context: &mut PlayerContext) -> usize {
        Self::switchable_mon_handles(context)
            .collect::<Vec<_>>()
            .into_iter()
            .map(|mon_handle| context.mon(*mon_handle).is_ok_and(|mon| !mon.fainted))
            .count()
    }

    /// Clears any active choice.
    pub fn clear_choice(context: &mut PlayerContext) {
        let mut choice = ChoiceState::new();
        if let Some(RequestType::Switch) = context.player().request_type() {
            let must_switch_out = Self::count_must_switch_out(context);
            let can_switch_in = Self::count_can_switch_in(context);
            let switches = must_switch_out.min(can_switch_in);
            let passes = must_switch_out - switches;
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
                "pass" => match Self::choose_pass(context) {
                    Err(error) => {
                        return Self::emit_choice_error(context, Error::wrap("cannot pass", error))
                    }
                    _ => (),
                },
                "learnmove" => match Self::choose_learn_move(context, data) {
                    Err(error) => {
                        return Self::emit_choice_error(
                            context,
                            Error::wrap("cannot learn move", error),
                        )
                    }
                    _ => (),
                },
                "escape" => match Self::choose_escape(context) {
                    Err(error) => {
                        return Self::emit_choice_error(
                            context,
                            Error::wrap("cannot escape", error),
                        );
                    }
                    _ => (),
                },
                "forfeit" => match Self::choose_forfeit(context) {
                    Err(error) => {
                        return Self::emit_choice_error(
                            context,
                            Error::wrap("cannot forfeit", error),
                        )
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

        if !Self::choice_done(context)? {
            return Self::emit_choice_error(
                context,
                battler_error!("incomplete choice: missing actions for Mons"),
            );
        }

        context.player_mut().choice.fulfilled = true;
        Ok(())
    }

    /// Modifies the player for the start of the battle.
    pub fn start_battle(&mut self) {
        self.mons_left = self.mons.len();
    }

    fn choose_switch(context: &mut PlayerContext, data: Option<&str>) -> Result<(), Error> {
        match context.player().request_type() {
            Some(RequestType::Turn | RequestType::Switch) => (),
            _ => return Err(battler_error!("you cannot switch out of turn")),
        };
        let active_position = Self::get_position_for_next_choice(context, false)?;
        if active_position >= context.player().active.len() {
            return match context.player().request_type() {
                Some(RequestType::Switch) => Err(battler_error!(
                    "you sent more switches than Mons that need to switch"
                )),
                _ => Err(battler_error!("you sent more choices than active Mons")),
            };
        }
        let active_mon_handle = context
            .player()
            .active_or_fainted
            .get(active_position)
            .cloned()
            .flatten()
            .wrap_error_with_format(format_args!(
                "expected player to have active Mon in position {active_position}"
            ))?;
        let active_mon = context.mon(active_mon_handle)?;
        let active_mon_position = active_mon
            .active_position
            .or(active_mon.old_active_position)
            .wrap_error_with_message("mon to switch out is not in an active position")?;
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

        let target_context = context
            .as_battle_context_mut()
            .mon_context(target_mon_handle)?;
        if target_context.mon().fainted {
            return Err(battler_error!("you cannot switch to a fainted Mon"));
        }

        let active_mon = context.mon(active_mon_handle)?;
        match context.player().request_type() {
            Some(RequestType::Turn) => {
                if active_mon.trapped {
                    return Err(battler_error!("{} is trapped", active_mon.name));
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

        context.player_mut().choice.switch_ins.insert(slot);
        let instant = context
            .player()
            .request_type()
            .is_some_and(|request_type| request_type == RequestType::Switch);
        context
            .player_mut()
            .choice
            .actions
            .push(Action::Switch(SwitchAction::new(SwitchActionInput {
                instant,
                mon: target_mon_handle,
                switching_out: active_mon_handle,
                position: active_mon_position,
            })));
        Ok(())
    }

    fn get_position_for_next_choice(
        context: &mut PlayerContext,
        pass: bool,
    ) -> Result<usize, Error> {
        if context.player().escaped {
            return Err(battler_error!(
                "you {} the battle",
                if Self::can_escape(context) {
                    "escaped from"
                } else {
                    "left"
                }
            ));
        }

        // Choices generate a single action, so there should be once choice for each active Mon.
        let mut next_mon = context.player().choice.actions.len();
        if !pass {
            match context.player().request_type() {
                Some(RequestType::Turn) => {
                    while context.player().active.get(next_mon).is_some_and(|mon| {
                        mon.is_none()
                            || mon.is_some_and(|mon| context.mon(mon).is_ok_and(|mon| mon.fainted))
                    }) {
                        Self::choose_pass(context)?;
                        next_mon += 1;
                    }
                }
                Some(RequestType::Switch) => {
                    while context
                        .player()
                        .active_or_fainted
                        .get(next_mon)
                        .is_some_and(|mon| {
                            mon.is_none()
                                || mon.is_some_and(|mon| {
                                    context
                                        .mon(mon)
                                        .is_ok_and(|mon| !mon.needs_switch.is_some())
                                })
                        })
                    {
                        Self::choose_pass(context)?;
                        next_mon += 1;
                    }
                }
                Some(RequestType::LearnMove) => {
                    while context.player().mons.get(next_mon).is_some_and(|mon| {
                        context
                            .mon(*mon)
                            .is_ok_and(|mon| mon.learnable_moves.is_empty())
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
        let position = Self::get_position_for_next_choice(context, true)?;
        match context.player().request_type() {
            Some(RequestType::Switch) => {
                if let Some(mon) = context.player().active_mon_handle(position) {
                    let mut context = context.mon_context(mon)?;
                    if context.mon().needs_switch.is_some() {
                        if context.player().choice.forced_passes_left == 0 {
                            return Err(battler_error!(
                                "cannot pass: you must select a Mon to replace {}",
                                context.mon().name,
                            ));
                        }
                        context.player_mut().choice.forced_passes_left -= 1;
                    }
                }
            }
            Some(RequestType::Turn) => {
                if let Some(mon) = context.player().active_mon_handle(position) {
                    let context = context.mon_context(mon)?;
                    if !context.mon().fainted
                        && !context.battle().engine_options.allow_pass_for_unfainted_mon
                    {
                        return Err(battler_error!(
                            "cannot pass: your {} must make a move or switch",
                            context.mon().name,
                        ));
                    };
                }
            }
            Some(RequestType::LearnMove) => (),
            _ => {
                return Err(battler_error!(
                    "cannot pass: only a move or switch can be passed"
                ));
            }
        }

        context.player_mut().choice.actions.push(Action::Pass);
        Ok(())
    }

    fn choose_move(context: &mut PlayerContext, data: Option<&str>) -> Result<(), Error> {
        match context.player().request_type() {
            Some(RequestType::Turn) => (),
            _ => return Err(battler_error!("you cannot move out of turn")),
        }
        let mut choice = MoveChoice::new(data.wrap_error_with_message("missing move choice")?)?;
        let active_position = Self::get_position_for_next_choice(context, false)?;
        if active_position >= context.player().active.len() {
            return Err(battler_error!("you sent more choices than active Mons"));
        }
        let mon_handle = context
            .player()
            .active_mon_handle(active_position)
            .wrap_error_with_format(format_args!(
                "expected an active Mon in position {active_position}"
            ))?;

        // This becomes our new context for the rest of the choice.
        let mut context = context
            .mon_context(mon_handle)
            .wrap_error_with_format(format_args!(
                "expected Mon to exist for handle {mon_handle}"
            ))?;

        let request = Mon::move_request(&mut context)?;
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

        let locked_move = Mon::locked_move(&mut context)?;
        if let Some(locked_move) = locked_move {
            let locked_move_target = context.mon().last_move_target_location;
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
        }

        let moves = Mon::moves(&mut context)?;

        let mov = context
            .battle()
            .dex
            .moves
            .get_by_id(&move_id)
            .into_result()
            .wrap_error_with_format(format_args!("expected move id {} to exist", move_slot.id))?;
        // Clone these to avoid borrow errors.
        //
        // We could find away around this if we're clever, but this keeps things simple for now.
        let move_name = mov.data.name.clone();
        let move_target = move_slot.target.unwrap_or(mov.data.target);

        if moves.is_empty() {
            // No moves, the Mon must use Struggle.
            move_id = Id::from_known("struggle");
        } else {
            // Make sure the selected move is not disabled.
            let move_slot = moves
                .get(choice.move_slot)
                .wrap_error_with_format(format_args!(
                    "expected move in slot {}",
                    choice.move_slot,
                ))?;
            if move_slot.disabled {
                return Err(battler_error!(
                    "{}'s {} is disabled",
                    context.mon().name,
                    move_name
                ));
            }
        }

        // Choosing 0 is the same as no target at all.
        if choice.target.is_some_and(|target| target == 0) {
            choice.target = None;
        }

        let target_required = context.battle().format.battle_type.active_per_player() > 1;
        match (mov.data.target.choosable(), choice.target) {
            (true, None) => {
                if target_required {
                    return Err(battler_error!("{} requires a target", move_name));
                }
            }
            (true, Some(target)) => {
                if !CoreBattle::valid_target(&mut context, move_target, target)? {
                    return Err(battler_error!("invalid target for {}", move_name));
                }
            }
            (false, Some(_)) => {
                return Err(battler_error!(
                    "you cannot choose a target for {}",
                    move_name
                ))
            }
            _ => (),
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

    fn choose_learn_move(context: &mut PlayerContext, data: Option<&str>) -> Result<(), Error> {
        match context.player().request_type() {
            Some(RequestType::LearnMove) => (),
            _ => return Err(battler_error!("you cannot learn move out of turn")),
        }

        let choice =
            LearnMoveChoice::new(data.wrap_error_with_message("missing learn move choice")?)?;
        let team_position = Self::get_position_for_next_choice(context, false)?;
        if team_position >= context.player().mons.len() {
            return Err(battler_error!("you sent more choices than Mons"));
        }
        let mon_handle = context
            .player()
            .mons
            .get(team_position)
            .wrap_error_with_format(format_args!("expected a Mon in position {team_position}"))?
            .clone();
        context
            .player_mut()
            .choice
            .actions
            .push(Action::LearnMove(LearnMoveAction {
                mon: mon_handle,
                forget_move_slot: choice.forget_move_slot,
            }));
        Ok(())
    }

    fn all_mons_can_escape(context: &mut PlayerContext) -> Result<bool, Error> {
        for mon in context
            .player()
            .active_mon_handles()
            .cloned()
            .collect::<Vec<_>>()
        {
            let can_escape = Mon::can_escape(&mut context.mon_context(mon)?)?;
            if !can_escape {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn choose_escape(context: &mut PlayerContext) -> Result<(), Error> {
        match context.player().request_type() {
            Some(RequestType::Turn) => (),
            _ => return Err(battler_error!("you cannot escape out of turn")),
        }

        let active_position = Self::get_position_for_next_choice(context, false)?;
        if active_position >= context.player().active.len() {
            return Err(battler_error!("you sent more choices than active Mons"));
        }
        let mon_handle = context
            .player()
            .active_mon_handle(active_position)
            .wrap_error_with_format(format_args!(
                "expected an active Mon in position {active_position}"
            ))?;

        let can_escape = Self::can_escape(context) && Self::all_mons_can_escape(context)?;
        if !can_escape {
            return Err(battler_error!("you cannot escape"));
        }

        context
            .player_mut()
            .choice
            .actions
            .push(Action::Escape(EscapeAction::new(EscapeActionInput {
                mon: mon_handle,
            })));

        Ok(())
    }

    fn choose_forfeit(context: &mut PlayerContext) -> Result<(), Error> {
        match context.player().request_type() {
            Some(RequestType::Turn) => (),
            _ => return Err(battler_error!("you cannot forfeit out of turn")),
        }

        if Self::get_position_for_next_choice(context, false)? >= context.player().active.len() {
            return Err(battler_error!("you sent more choices than active Mons"));
        }
        if !Self::can_forfeit(context) {
            return Err(battler_error!("you cannot forfeit"));
        }

        let action = Action::Forfeit(ForfeitAction {
            player: context.player().index,
            order: context.battle_mut().next_forfeit_order(),
        });
        context.player_mut().choice.actions.push(action);

        Ok(())
    }

    /// Checks if the player needs to switch a Mon out.
    pub fn needs_switch(context: &PlayerContext) -> Result<bool, Error> {
        for mon in context.player().active_or_fainted_mon_handles() {
            if context.mon(*mon)?.needs_switch.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Checks if the player can switch.
    pub fn can_switch(context: &PlayerContext) -> bool {
        Self::switchable_mon_handles(context).count() > 0
    }

    /// Checks if the player can escape, irrespective of individual Mons.
    pub fn can_escape(context: &PlayerContext) -> bool {
        context.player().player_type.can_escape()
            && (context.player().player_type.wild()
                || context
                    .battle()
                    .players_on_side(context.foe_side().index)
                    .all(|foe| foe.player_type.escapable()))
    }

    /// Checks if the player can forfeit.
    pub fn can_forfeit(context: &PlayerContext) -> bool {
        context.player().player_type.can_forfeit()
            && context
                .battle()
                .players_on_side(context.foe_side().index)
                .all(|foe| foe.player_type.forfeitable())
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

#[cfg(test)]
mod player_type_tests {
    use crate::battle::{
        PlayerType,
        WildPlayerOptions,
    };

    #[test]
    fn deserializes() {
        assert_eq!(
            serde_json::from_str::<PlayerType>(
                r#"{
                    "type": "wild",
                    "catchable": true,
                    "escapable": false,
                    "can_escape": false
                }"#
            )
            .unwrap(),
            PlayerType::Wild(WildPlayerOptions {
                catchable: true,
                escapable: false,
                can_escape: false
            })
        );
        assert_eq!(
            serde_json::from_str::<PlayerType>(
                r#"{
                    "type": "trainer"
                }"#
            )
            .unwrap(),
            PlayerType::Trainer
        );
        assert_eq!(
            serde_json::from_str::<PlayerType>(
                r#"{
                    "type": "protagonist"
                }"#
            )
            .unwrap(),
            PlayerType::Protagonist
        );
    }
}
