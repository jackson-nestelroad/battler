use std::{
    marker::PhantomPinned,
    mem,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use ahash::HashMapExt;
use uuid::Uuid;
use zone_alloc::{
    ElementRef,
    ElementRefMut,
};

use crate::{
    battle::{
        core_battle_actions,
        Action,
        Battle,
        BattleEngineOptions,
        BattleOptions,
        BattleQueue,
        BattleRegistry,
        Context,
        CoreBattleOptions,
        Field,
        Mon,
        MonHandle,
        MoveHandle,
        Player,
        PseudoRandomNumberGenerator,
        Request,
        RequestType,
        Side,
        TeamPreviewRequest,
        TurnRequest,
    },
    battle_event,
    battler_error,
    common::{
        Error,
        FastHashMap,
        Id,
        Identifiable,
        WrapResultError,
    },
    config::Format,
    dex::{
        DataStore,
        Dex,
    },
    log::{
        BattleEvent,
        EventLog,
    },
    mons::{
        Type,
        TypeEffectiveness,
    },
    moves::Move,
};

/// The public interface for a [`CoreBattle`].
///
/// Intended to separate public methods used by library users (a.k.a., battle operators) and public
/// methods used by internal battle logic.
pub struct PublicCoreBattle<'d> {
    /// The internal [`CoreBattle`], which contains all battle objects and logic.
    pub internal: CoreBattle<'d>,
}

impl<'d> PublicCoreBattle<'d> {
    /// Constructs a new [`PublicCoreBattle`] from a
    /// [`BattleBuilder`][`crate::battle::BattleBuilder`].
    pub(crate) fn from_builder(
        options: CoreBattleOptions,
        dex: Dex<'d>,
        format: Format,
        engine_options: BattleEngineOptions,
    ) -> Result<Self, Error> {
        let internal = CoreBattle::from_builder(options, dex, format, engine_options)?;
        Ok(Self { internal })
    }
}

impl<'d> Battle<'d, CoreBattleOptions> for PublicCoreBattle<'d> {
    fn new(
        options: CoreBattleOptions,
        data: &'d dyn DataStore,
        engine_options: BattleEngineOptions,
    ) -> Result<Self, Error> {
        let internal = CoreBattle::new(options, data, engine_options)?;
        Ok(Self { internal })
    }

    fn started(&self) -> bool {
        self.internal.started
    }

    fn ended(&self) -> bool {
        self.internal.ended
    }

    fn has_new_logs(&self) -> bool {
        self.internal.has_new_logs()
    }

    fn all_logs(&self) -> impl Iterator<Item = &str> {
        self.internal.all_logs()
    }

    fn new_logs(&mut self) -> impl Iterator<Item = &str> {
        self.internal.new_logs()
    }

    fn log(&mut self, event: BattleEvent) {
        self.internal.log(event)
    }

    fn log_many<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = BattleEvent>,
    {
        self.internal.log_many(events)
    }

    fn start(&mut self) -> Result<(), Error> {
        self.internal.start()
    }

    fn ready_to_continue(&mut self) -> Result<bool, Error> {
        self.internal.ready_to_continue()
    }

    fn continue_battle(&mut self) -> Result<(), Error> {
        self.internal.continue_battle()
    }

    fn active_requests<'b>(&'b self) -> impl Iterator<Item = (String, Request)> + 'b {
        self.internal.active_requests()
    }

    fn set_player_choice(&mut self, player_id: &str, input: &str) -> Result<(), Error> {
        self.internal.set_player_choice(player_id, input)
    }
}

/// The core implementation of a [`Battle`].
///
/// All battle logic lives here.
pub struct CoreBattle<'d> {
    id: Uuid,
    log: EventLog,

    // SAFETY: None of the objects below should be overwritten or destroyed for the lifetime of the
    // battle.
    //
    // We could PinBox these, but that would complicate our code quite a bit.
    pub prng: PseudoRandomNumberGenerator,
    pub dex: Dex<'d>,
    pub queue: BattleQueue,
    pub format: Format,
    pub field: Field,
    pub sides: [Side; 2],
    pub players: Vec<Player>,

    registry: BattleRegistry,
    player_ids: FastHashMap<String, usize>,

    engine_options: BattleEngineOptions,
    turn: u64,
    request: Option<RequestType>,
    mid_turn: bool,
    started: bool,
    ended: bool,
    next_ability_priority: u32,

    active_mon: Option<MonHandle>,

    input_log: FastHashMap<usize, FastHashMap<u64, String>>,

    _pin: PhantomPinned,
}

// Block for constructors.
impl<'d> CoreBattle<'d> {
    fn new(
        mut options: CoreBattleOptions,
        data: &'d dyn DataStore,
        engine_options: BattleEngineOptions,
    ) -> Result<Self, Error> {
        options
            .validate()
            .wrap_error_with_message("battle options are invalid")?;
        let dex = Dex::new(data)?;
        let format_data = mem::replace(&mut options.format, None);
        let format = Format::new(
            format_data.wrap_error_with_message("missing format field for new battle")?,
            &dex,
        )?;
        Self::from_builder(options, dex, format, engine_options)
    }

    fn from_builder(
        options: CoreBattleOptions,
        dex: Dex<'d>,
        format: Format,
        engine_options: BattleEngineOptions,
    ) -> Result<Self, Error> {
        let id = Uuid::new_v4();
        let prng = match options.seed {
            Some(seed) => PseudoRandomNumberGenerator::new_with_seed(seed),
            None => PseudoRandomNumberGenerator::new(),
        };
        let log = EventLog::new();
        let registry = BattleRegistry::new();
        let queue = BattleQueue::new();
        let field = Field::new();
        let (side_1, mut players) =
            Side::new(options.side_1, 0, &format.battle_type, &dex, &registry)?;
        let (side_2, side_2_players) =
            Side::new(options.side_2, 1, &format.battle_type, &dex, &registry)?;
        players.extend(side_2_players);

        let player_ids = players
            .iter()
            .enumerate()
            .map(move |(player_index, player)| (player.id.to_owned(), player_index))
            .collect();
        let input_log = FastHashMap::from_iter(
            players
                .iter()
                .enumerate()
                .map(|(index, _)| (index, FastHashMap::new())),
        );

        let mut battle = Self {
            id,
            log,
            prng,
            dex,
            queue,
            format,
            field,
            sides: [side_1, side_2],
            players,
            registry,
            player_ids,
            engine_options,
            turn: 0,
            request: None,
            mid_turn: false,
            started: false,
            ended: false,
            next_ability_priority: 0,
            active_mon: None,
            input_log,
            _pin: PhantomPinned,
        };
        Self::initialize(&mut battle.context())?;
        Ok(battle)
    }
}

// Block for all basic getters.
impl<'d> CoreBattle<'d> {
    pub fn context<'b>(&'b mut self) -> Context<'b, 'd> {
        Context::new(self)
    }

    pub fn sides(&self) -> impl Iterator<Item = &Side> {
        self.sides.iter()
    }

    pub fn sides_mut(&mut self) -> impl Iterator<Item = &mut Side> {
        self.sides.iter_mut()
    }

    pub fn side(&self, side: usize) -> Result<&Side, Error> {
        self.sides
            .get(side)
            .wrap_error_with_format(format_args!("side {side} does not exist"))
    }

    pub fn side_mut(&mut self, side: usize) -> Result<&mut Side, Error> {
        self.sides
            .get_mut(side)
            .wrap_error_with_format(format_args!("side {side} does not exist"))
    }

    pub fn players(&self) -> impl Iterator<Item = &Player> {
        self.players.iter()
    }

    pub fn players_mut(&mut self) -> impl Iterator<Item = &mut Player> {
        self.players.iter_mut()
    }

    pub fn player(&self, player: usize) -> Result<&Player, Error> {
        self.players
            .get(player)
            .wrap_error_with_format(format_args!("player {player} does not exist"))
    }

    pub fn player_mut(&mut self, player: usize) -> Result<&mut Player, Error> {
        self.players
            .get_mut(player)
            .wrap_error_with_format(format_args!("player {player} does not exist"))
    }

    pub fn players_on_side(&self, side: usize) -> impl Iterator<Item = &Player> {
        self.players().filter(move |player| player.side == side)
    }

    fn player_index_by_id(&self, player_id: &str) -> Result<usize, Error> {
        self.player_ids
            .get(player_id)
            .wrap_error_with_format(format_args!("{player_id} does not exist"))
            .cloned()
    }

    pub unsafe fn mon<'b>(&'b self, mon_handle: MonHandle) -> Result<ElementRef<'b, Mon>, Error> {
        self.registry.mon(mon_handle)
    }

    pub unsafe fn mon_mut<'b>(
        &'b self,
        mon_handle: MonHandle,
    ) -> Result<ElementRefMut<'b, Mon>, Error> {
        self.registry.mon_mut(mon_handle)
    }

    pub unsafe fn this_turn_move<'b>(
        &'b self,
        move_handle: MoveHandle,
    ) -> Result<ElementRef<'b, Move>, Error> {
        self.registry.this_turn_move(move_handle)
    }

    pub unsafe fn this_turn_move_mut<'b>(
        &'b self,
        move_handle: MoveHandle,
    ) -> Result<ElementRefMut<'b, Move>, Error> {
        self.registry.this_turn_move_mut(move_handle)
    }

    pub fn all_mon_handles<'b>(&'b self) -> impl Iterator<Item = MonHandle> + 'b {
        self.sides()
            .map(|side| self.all_mon_handles_on_side(side.index))
            .flatten()
    }

    pub fn all_mon_handles_on_side<'b>(
        &'b self,
        side: usize,
    ) -> impl Iterator<Item = MonHandle> + 'b {
        self.players_on_side(side)
            .map(|player| player.mons.iter())
            .flatten()
            .cloned()
    }

    pub fn active_positions_on_side<'b>(
        &'b self,
        side: usize,
    ) -> impl Iterator<Item = Option<MonHandle>> + 'b {
        self.players_on_side(side)
            .map(|player| player.active.iter())
            .flatten()
            .cloned()
    }

    pub fn active_mon_handles_on_side<'b>(
        &'b self,
        side: usize,
    ) -> impl Iterator<Item = MonHandle> + 'b {
        self.active_positions_on_side(side)
            .filter_map(|position| position)
    }

    pub fn next_ability_priority(&mut self) -> u32 {
        let next = self.next_ability_priority;
        self.next_ability_priority += 1;
        next
    }

    pub fn side_length(&self, side: &Side) -> usize {
        self.players_on_side(side.index).count() * self.format.battle_type.active_per_player()
    }

    pub fn max_side_length(&self) -> usize {
        self.sides()
            .map(|side| self.side_length(side))
            .max()
            .unwrap_or(0)
    }

    pub fn active_mon_handle(&self) -> Option<MonHandle> {
        self.active_mon.clone()
    }
}

// Block for methods that are only called from the public interface.
impl<'d> CoreBattle<'d> {
    fn has_new_logs(&self) -> bool {
        self.log.has_new_messages()
    }

    fn all_logs(&self) -> impl Iterator<Item = &str> {
        self.log.logs()
    }

    fn new_logs(&mut self) -> impl Iterator<Item = &str> {
        self.log.read_out()
    }

    fn start(&mut self) -> Result<(), Error> {
        Self::start_internal(&mut self.context())
    }

    fn ready_to_continue(&mut self) -> Result<bool, Error> {
        Self::all_player_choices_done(&mut self.context())
    }

    fn continue_battle(&mut self) -> Result<(), Error> {
        if !self.ready_to_continue()? {
            return Err(battler_error!("battle is not ready to continue"));
        }
        Self::continue_battle_internal(&mut self.context())
    }

    fn active_requests<'b>(&'b self) -> impl Iterator<Item = (String, Request)> + 'b {
        self.players().filter_map(|player| {
            player
                .active_request()
                .map(|request| (player.id.to_owned(), request))
        })
    }

    fn set_player_choice(&mut self, player_id: &str, input: &str) -> Result<(), Error> {
        Self::set_player_choice_internal(&mut self.context(), player_id, input)
    }
}

impl<'d> CoreBattle<'d> {
    pub fn log(&mut self, event: BattleEvent) {
        self.log.push(event)
    }

    pub fn log_many<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = BattleEvent>,
    {
        self.log.push_extend(events)
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn ended(&self) -> bool {
        self.ended
    }

    fn initialize(context: &mut Context) -> Result<(), Error> {
        for player in 0..context.battle().players.len() {
            let mut context = context.player_context(player)?;
            Player::set_index(&mut context, player)?;
        }
        let mon_handles = context.battle().all_mon_handles().collect::<Vec<_>>();
        for mon_handle in mon_handles {
            let mut context = context.mon_context(mon_handle)?;
            Mon::initialize(&mut context)?;
        }
        Ok(())
    }

    fn start_internal(context: &mut Context) -> Result<(), Error> {
        if context.battle().started {
            return Err(battler_error!("battle already started"));
        }
        context.battle_mut().started = true;

        let battle_type_event = battle_event!("battletype", context.battle().format.battle_type);
        context.battle_mut().log(battle_type_event);

        // Extract and sort all rule logs.
        //
        // We sort to keep the battle log stable.
        let mut rule_logs = context
            .battle()
            .format
            .rules
            .clauses(&context.battle().dex)
            .filter_map(|clause| {
                clause.data.rule_log.as_ref().map(|rule_log| {
                    let value = context
                        .battle()
                        .format
                        .rules
                        .value(clause.id())
                        .wrap_error_with_format(format_args!(
                            "expected {} to be present in ruleset",
                            clause.data.name
                        ))?;
                    let rule_log = rule_log.replace("{}", value);
                    Ok(rule_log)
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        rule_logs.sort();
        context.battle_mut().log_many(
            rule_logs
                .into_iter()
                .map(|rule_log| battle_event!("rule", rule_log)),
        );

        let player_logs = context
            .battle()
            .players()
            .map(|player| battle_event!("player", player.id, player.side, player.position))
            .collect::<Vec<_>>();
        context.battle_mut().log_many(player_logs);

        if context.battle().has_team_preview() {
            context.battle_mut().log_team_sizes();
            Self::start_team_preview(context)?;
        }

        BattleQueue::add_action(context, Action::Start)?;
        context.battle_mut().mid_turn = true;

        if context.battle().request.is_none() && context.battle().engine_options.auto_continue {
            Self::continue_battle_internal(context)?;
        }

        Ok(())
    }

    fn log_current_time(&mut self) {
        self.log(battle_event!(
            "time",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
    }

    fn log_team_sizes(&mut self) {
        let team_size_events = self
            .players()
            .map(|player| battle_event!("teamsize", player.id, player.mons.len()))
            .collect::<Vec<_>>();
        self.log_many(team_size_events);
    }

    fn has_team_preview(&self) -> bool {
        self.format.rules.has_rule(&Id::from_known("teampreview"))
    }

    fn start_team_preview(context: &mut Context) -> Result<(), Error> {
        context.battle_mut().log(battle_event!("teampreviewstart"));
        let events = context
            .battle()
            .all_mon_handles()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|mon_handle| {
                let context = context.mon_context(mon_handle)?;
                Ok(battle_event!(
                    "mon",
                    context.player().id,
                    context.mon().public_details(),
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        context.battle_mut().log_many(events);
        match context.battle().format.rules.numeric_rules.picked_team_size {
            Some(picked_team_size) => context
                .battle_mut()
                .log
                .push(battle_event!("teampreview", picked_team_size)),
            None => context.battle_mut().log(battle_event!("teampreview")),
        }
        Self::make_request(context, RequestType::TeamPreview)?;
        Ok(())
    }

    fn get_request_for_player(
        context: &mut Context,
        player: usize,
        request_type: RequestType,
    ) -> Result<Request, Error> {
        match request_type {
            RequestType::TeamPreview => {
                let max_team_size = context
                    .battle()
                    .format
                    .rules
                    .numeric_rules
                    .picked_team_size
                    .map(|size| size as usize);
                let mut context = context.player_context(player)?;
                Ok(Request::TeamPreview(TeamPreviewRequest {
                    max_team_size,
                    player: Player::request_data(&mut context)?,
                }))
            }
            RequestType::Turn => {
                let mut context = context.player_context(player)?;
                let active = Player::active_mon_handles(&context)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|mon| {
                        let mut context = context.mon_context(mon)?;
                        Mon::move_request(&mut context)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Request::Turn(TurnRequest {
                    active,
                    player: Player::request_data(&mut context)?,
                }))
            }
            RequestType::Switch => todo!("switch requests are not yet implemented"),
        }
    }

    fn set_player_choice_internal(
        context: &mut Context,
        player_id: &str,
        input: &str,
    ) -> Result<(), Error> {
        let player = context.battle().player_index_by_id(player_id)?;
        Player::make_choice(&mut context.player_context(player)?, input)?;
        let turn = context.battle().turn;
        context
            .battle_mut()
            .input_log
            .get_mut(&player)
            .wrap_error_with_format(format_args!("input_log for player {player} does not exist"))?
            .insert(turn, input.to_owned());

        if Self::all_player_choices_done(context)? {
            Self::commit_choices(context)?;
        }

        Ok(())
    }

    fn make_request(context: &mut Context, request_type: RequestType) -> Result<(), Error> {
        context.battle_mut().request = Some(request_type);
        Self::clear_requests(context)?;

        for player in 0..context.battle().players.len() {
            let request = Self::get_request_for_player(context, player, request_type)?;
            let mut context = context.player_context(player)?;
            context.player_mut().make_request(request);
        }
        Ok(())
    }

    fn all_player_choices_done(context: &mut Context) -> Result<bool, Error> {
        for player in 0..context.battle().players.len() {
            let context = context.player_context(player)?;
            if !Player::choice_done(&context) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn clear_requests(context: &mut Context) -> Result<(), Error> {
        context.battle_mut().request = None;
        for player in 0..context.battle().players.len() {
            let mut context = context.player_context(player)?;
            context.player_mut().clear_request();
            Player::clear_choice(&mut context);
        }
        Ok(())
    }

    fn commit_choices(context: &mut Context) -> Result<(), Error> {
        // Take all player actions and insert them into the battle queue.
        let choices = context
            .battle_mut()
            .players_mut()
            .map(|player| player.take_choice())
            .collect::<Vec<_>>();
        for choice in choices {
            BattleQueue::add_actions(context, choice.actions.into_iter())?;
        }
        Self::clear_requests(context)?;

        if context.battle().engine_options.auto_continue {
            Self::continue_battle_internal(context)?;
        }
        Ok(())
    }

    fn continue_battle_internal(context: &mut Context) -> Result<(), Error> {
        context.battle_mut().log_current_time();

        context.battle_mut().request = None;

        if !context.battle().mid_turn {
            BattleQueue::add_action(context, Action::BeforeTurn)?;
            BattleQueue::add_action(context, Action::Residual)?;
            context.battle_mut().mid_turn = true;
        }

        // Sort the new actions and continue the battle.
        context.battle_mut().queue.sort();

        // Run actions as long as possible.
        while let Some(action) = context.battle_mut().queue.pop_front() {
            Self::run_action(context, action)?;
            // This action ended the game.
            if context.battle().request.is_some() || context.battle().ended {
                return Ok(());
            }
        }

        Self::next_turn(context)?;
        context.battle_mut().mid_turn = false;
        Ok(())
    }

    fn run_action(context: &mut Context, action: Action) -> Result<(), Error> {
        match action {
            Action::Start => {
                context.battle_mut().log_team_sizes();
                for player in context.battle_mut().players_mut() {
                    player.start_battle();
                }
                context.battle_mut().log(battle_event!("start"));

                let switch_ins =
                    context
                        .battle()
                        .players()
                        .filter(|player| player.mons_left() > 0)
                        .flat_map(|player| {
                            player.active.iter().enumerate().filter_map(|(i, _)| {
                                player.mons.get(i).cloned().map(|mon| (i, mon))
                            })
                        })
                        .collect::<Vec<_>>();
                for (position, mon) in switch_ins {
                    let mut context = context.mon_context(mon)?;
                    core_battle_actions::switch_in(&mut context, position)?;
                }
                context.battle_mut().mid_turn = true;
            }
            Action::Team(action) => {
                let mut context = context.mon_context(action.mon_action.mon)?;
                if action.index == 0 {
                    context.player_mut().mons.clear();
                }
                context.player_mut().mons.push(action.mon_action.mon);
            }
            Action::Switch(action) => {
                let mut context = context.mon_context(action.mon_action.mon)?;
                core_battle_actions::switch_in(&mut context, action.position)?;
            }
            Action::Move(action) => {
                let mut context = context.mon_context(action.mon_action.mon)?;
                if !context.mon().active || context.mon().fainted {
                    return Ok(());
                }
                core_battle_actions::do_move(
                    &mut context,
                    &action.id,
                    action.target,
                    action.original_target,
                    false,
                )?;
            }
            Action::MegaEvo(_) => todo!("mega evolution is not implemented"),
            Action::Pass => (),
            Action::BeforeTurn => (),
            Action::Residual => {
                context.battle_mut().log(battle_event!("residual"));
            }
        }
        Ok(())
    }

    fn next_turn(context: &mut Context) -> Result<(), Error> {
        context.battle_mut().turn += 1;
        let turn_event = battle_event!("turn", context.battle().turn);
        context.battle_mut().log(turn_event);

        if context.battle().turn >= 1000 {
            context.battle_mut().log(battle_event!(
                "message",
                "It is turn 1000. You have hit the turn limit!"
            ));
            Self::tie(context)?;
        }

        Self::make_request(context, RequestType::Turn)?;
        Ok(())
    }

    fn tie(context: &mut Context) -> Result<(), Error> {
        Self::win(context, None)
    }

    fn win(context: &mut Context, side: Option<usize>) -> Result<(), Error> {
        if context.battle().ended {
            return Ok(());
        }

        context.battle_mut().log(battle_event!());
        match side {
            None => context.battle_mut().log(battle_event!("tie")),
            Some(side) => {
                let side = context.battle().side(side)?;
                let win_event = battle_event!("win", side.name);
                context.battle_mut().log(win_event);
            }
        }

        context.battle_mut().ended = true;
        Self::clear_requests(context)?;
        Ok(())
    }

    fn calculate_action_priority(context: &mut Context, action: &mut Action) -> Result<(), Error> {
        if let Action::Move(action) = action {
            let mov = context
                .battle()
                .dex
                .moves
                .get_by_id(&action.id)
                .into_result()?;
            action.priority = mov.data.priority as i32;
            // TODO: Run priority modification events for the move and Mon.
        }
        if let Some(mon_action) = action.mon_action_mut() {
            let mut context = context.mon_context(mon_action.mon)?;
            mon_action.speed = Mon::action_speed(&mut context)? as u32;
        }
        Ok(())
    }

    pub fn resolve_action(context: &mut Context, action: &mut Action) -> Result<(), Error> {
        if let Action::Move(action) = action {
            let mut context = context.mon_context(action.mon_action.mon)?;
            if let Some(target) = action.target {
                action.original_target = Mon::get_target(&mut context, target)?;
            }
        }
        Self::calculate_action_priority(context, action)?;
        Ok(())
    }

    pub fn random_target(
        context: &mut Context,
        mon: MonHandle,
        move_id: &Id,
    ) -> Result<Option<MonHandle>, Error> {
        let mov = context
            .battle()
            .dex
            .moves
            .get_by_id(move_id)
            .into_result()?;
        let target = mov.data.target.clone();

        if target.can_target_user() {
            // Target the user if possible.
            return Ok(Some(mon));
        }

        let mut context = context.mon_context(mon)?;
        let mons = if !target.can_target_foes() {
            // Cannot target foes, so only consider allies.
            Mon::adjacent_allies(&mut context)?
                .filter_map(|ally| ally)
                .collect::<Vec<_>>()
        } else if target.is_adjacent_only() {
            // Consider adjacent foes. Allies are excluded, so that a move will never randomly
            // target an ally if it doesn't need to.
            Mon::adjacent_foes(&mut context)?
                .filter_map(|foe| foe)
                .collect::<Vec<_>>()
        } else {
            // Consider all foes.
            Mon::active_foes(&mut context).collect::<Vec<_>>()
        };

        Ok(context
            .battle_mut()
            .prng
            .sample_slice(&mons)
            .cloned()
            .map(|mon| Some(mon))
            .unwrap_or(None))
    }

    pub fn get_target(
        context: &mut Context,
        mon: MonHandle,
        move_id: &Id,
        target: Option<isize>,
        original_target: Option<MonHandle>,
    ) -> Result<Option<MonHandle>, Error> {
        let mov = context
            .battle()
            .dex
            .moves
            .get_by_id(move_id)
            .into_result()?;
        let tracks_target = mov.data.tracks_target;
        let smart_target = mov.data.smart_target;
        let move_target = mov.data.target.clone();

        if tracks_target {
            if let Some(original_target) = original_target {
                let context = context.mon_context(original_target)?;
                if context.mon().active {
                    // Move's original target is on the field.
                    return Ok(Some(original_target));
                }
            }
        }

        if smart_target {
            let mut context = context.mon_context(mon)?;
            if let Some(target) = target {
                if let Some(target) = Mon::get_target(&mut context, target)? {
                    return Ok(Some(target));
                }
            }
        }

        if let Some(target) = target {
            if !move_target.is_random() && move_target.valid_target(target) {
                let mut context = context.mon_context(mon)?;
                if let Some(target_mon_handle) = Mon::get_target(&mut context, target)? {
                    let target_context = context
                        .as_battle_context_mut()
                        .mon_context(target_mon_handle)?;
                    if !target_context.mon().fainted
                        || target_context
                            .mon()
                            .is_ally(target_context.as_battle_context().mon(mon)?)
                    {
                        // Target is unfainted or an ally, so the chosen target is still valid.
                        return Ok(Some(target_mon_handle));
                    }
                }
            }
        }

        // The chosen target is not valid.
        if !move_target.requires_target() {
            Ok(None)
        } else {
            Self::random_target(context, mon, move_id)
        }
    }

    pub fn set_active_target(
        context: &mut Context,
        target: Option<MonHandle>,
    ) -> Result<(), Error> {
        context
            .mon_context(context.battle().active_mon.wrap_error_with_message(
                "cannot set an active target when no active mon is set",
            )?)?
            .mon_mut()
            .active_target = target;
        Ok(())
    }

    pub fn register_move(&self, mov: Move) -> MoveHandle {
        self.registry.register_move(mov)
    }

    pub fn set_active_move(
        context: &mut Context,
        move_handle: MoveHandle,
        user: MonHandle,
        target: Option<MonHandle>,
    ) -> Result<(), Error> {
        context.battle_mut().active_mon = Some(user);
        context
            .mon_context(user)?
            .mon_mut()
            .set_active_move(move_handle, target);
        Ok(())
    }

    pub fn clear_active_move(context: &mut Context) -> Result<(), Error> {
        if let Some(active_mon) = context.battle().active_mon {
            context
                .mon_context(active_mon)?
                .mon_mut()
                .clear_active_move();
            context.battle_mut().active_mon = None;
        }
        Ok(())
    }

    pub fn check_type_immunity(&self, offense: Type, defense: &[Type]) -> bool {
        defense
            .iter()
            .map(|defense| {
                self.dex
                    .type_chart()
                    .get(defense)
                    .and_then(|row| row.get(&offense))
                    .unwrap_or(&TypeEffectiveness::Normal)
            })
            .any(|effectiveness| effectiveness == &TypeEffectiveness::None)
    }

    pub fn check_type_effectiveness(&self, offense: Type, defense: Type) -> i8 {
        match self
            .dex
            .type_chart()
            .get(&defense)
            .and_then(|row| row.get(&offense))
            .unwrap_or(&TypeEffectiveness::Normal)
        {
            TypeEffectiveness::Strong => 1,
            TypeEffectiveness::Weak => -1,
            _ => 0,
        }
    }

    pub fn randomize_base_damage(&mut self, base_damage: u32) -> u32 {
        base_damage * (100 - (self.prng.range(0, 16) as u32)) / 100
    }
}
