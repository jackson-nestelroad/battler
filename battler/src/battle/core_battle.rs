use std::{
    marker::PhantomPinned,
    mem,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use uuid::Uuid;
use zone_alloc::{
    ElementRef,
    ElementRefMut,
};

use crate::{
    battle::{
        core_battle_actions,
        Action,
        ActiveMoveContext,
        Battle,
        BattleEngineOptions,
        BattleOptions,
        BattleQueue,
        BattleRegistry,
        Context,
        CoreBattleOptions,
        Field,
        Mon,
        MonContext,
        MonHandle,
        MoveHandle,
        Player,
        PlayerContext,
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
};

/// The core implementation of a [`Battle`].
///
/// All battle logic lives here.
pub struct CoreBattle<'d> {
    id: Uuid,
    log: EventLog,

    pub prng: PseudoRandomNumberGenerator,
    pub dex: Dex<'d>,
    pub registry: BattleRegistry,
    pub queue: BattleQueue,
    pub format: Format,
    pub field: Field,
    pub sides: [Side; 2],
    pub players: Vec<Player>,

    player_ids: FastHashMap<String, usize>,

    engine_options: BattleEngineOptions,
    turn: u64,
    request: Option<RequestType>,
    mid_turn: bool,
    started: bool,
    ended: bool,
    next_ability_priority: u32,

    active_mon: Option<MonHandle>,

    input_log: FastHashMap<usize, Vec<String>>,

    _pin: PhantomPinned,
}

// Additional constructors.
impl<'d> CoreBattle<'d> {
    /// Constructs a new [`CoreBattle`] from a [`BattleBuilder`][`crate::battle::BattleBuilder`].
    pub(crate) fn from_builder(
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
                .map(|(index, _)| (index, Vec::<String>::new())),
        );

        let mut battle = Self {
            id,
            log,
            prng,
            dex,
            registry,
            queue,
            format,
            field,
            sides: [side_1, side_2],
            players,
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
        battle.initialize()?;
        Ok(battle)
    }
}

// Block for all basic getters.
impl<'d> CoreBattle<'d> {
    fn context<'b>(&'b mut self) -> Context<'b, 'd> {
        Context::new(self)
    }

    fn player_context<'b>(
        &'b mut self,
        player: usize,
    ) -> Result<PlayerContext<'_, '_, 'b, 'd>, Error> {
        PlayerContext::new(self.context().into(), player)
    }

    fn mon_context<'b>(
        &'b mut self,
        mon: MonHandle,
    ) -> Result<MonContext<'_, '_, '_, '_, 'b, 'd>, Error> {
        MonContext::new(self.context().into(), mon)
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

    pub fn mon(&self, mon: MonHandle) -> Result<ElementRef<Mon>, Error> {
        self.registry
            .mon(mon)
            .wrap_error_with_format(format_args!("mon {mon} does not exist"))
    }

    pub fn mon_mut(&self, mon: MonHandle) -> Result<ElementRefMut<Mon>, Error> {
        self.registry
            .mon_mut(mon)
            .wrap_error_with_format(format_args!("mon {mon} does not exist"))
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

    pub fn all_mon_handles<'b>(&'b self) -> impl Iterator<Item = MonHandle> + 'b {
        self.sides()
            .map(|side| self.all_mon_handles_on_side(side.index))
            .flatten()
    }

    pub fn all_mons(&self) -> impl Iterator<Item = Result<ElementRef<Mon>, Error>> {
        self.sides()
            .map(|side| self.all_mons_on_side(side.index))
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

    pub fn all_mons_on_side(
        &self,
        side: usize,
    ) -> impl Iterator<Item = Result<ElementRef<Mon>, Error>> {
        self.players_on_side(side)
            .map(|player| player.mons.iter())
            .flatten()
            .map(|mon| self.mon(*mon))
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

    pub fn active_mons_on_side<'b>(
        &'b self,
        side: usize,
    ) -> impl Iterator<Item = Result<ElementRef<Mon>, Error>> + 'b {
        self.active_mon_handles_on_side(side)
            .map(|mon| self.mon(mon))
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

    pub fn active_mon(&self) -> Result<ElementRef<Mon>, Error> {
        self.mon(
            self.active_mon_handle()
                .wrap_error_with_message("no active mon")?,
        )
    }

    pub fn active_mon_mut(&self) -> Result<ElementRefMut<Mon>, Error> {
        self.mon_mut(
            self.active_mon_handle()
                .wrap_error_with_message("no active mon")?,
        )
    }
}

// Block for all public methods.
impl<'d> Battle<'d, CoreBattleOptions> for CoreBattle<'d> {
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

    fn started(&self) -> bool {
        self.started
    }

    fn ended(&self) -> bool {
        self.ended
    }

    fn has_new_logs(&self) -> bool {
        self.log.has_new_messages()
    }

    fn all_logs(&self) -> impl Iterator<Item = &str> {
        self.log.logs()
    }

    fn new_logs(&mut self) -> impl Iterator<Item = &str> {
        self.log.read_out()
    }

    fn log(&mut self, event: BattleEvent) {
        self.log.push(event)
    }

    fn hint(&mut self, message: &str) {
        self.log(battle_event!("-hint", message))
    }

    fn log_many<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = BattleEvent>,
    {
        self.log.push_extend(events)
    }

    fn start(&mut self) -> Result<(), Error> {
        if self.started {
            return Err(battler_error!("battle already started"));
        }
        self.started = true;

        self.log(battle_event!("battletype", self.format.battle_type));

        // Extract and sort all rule logs.
        //
        // We sort to keep the battle log stable.
        let mut rule_logs = self
            .format
            .rules
            .clauses(&self.dex)
            .filter_map(|clause| {
                clause.data.rule_log.as_ref().map(|rule_log| {
                    let value = self
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
        self.log_many(
            rule_logs
                .into_iter()
                .map(|rule_log| battle_event!("rule", rule_log)),
        );

        let player_logs = self
            .players()
            .map(|player| battle_event!("player", player.id, player.side, player.position))
            .collect::<Vec<_>>();
        self.log_many(player_logs);

        if self.has_team_preview() {
            self.log_team_sizes();
            self.start_team_preview()?;
        }

        let mut context = self.context();
        BattleQueue::add_action(&mut context, Action::Start)?;
        self.mid_turn = true;

        if self.request.is_none() && self.engine_options.auto_continue {
            self.continue_battle_internal()?;
        }

        Ok(())
    }

    fn ready_to_continue(&mut self) -> Result<bool, Error> {
        self.all_player_choices_done()
    }

    fn continue_battle(&mut self) -> Result<(), Error> {
        if !self.ready_to_continue()? {
            return Err(battler_error!("battle is not ready to continue"));
        }
        self.continue_battle_internal()
    }

    fn active_requests<'b>(&'b self) -> impl Iterator<Item = (String, Request)> + 'b {
        self.players().filter_map(|player| {
            player
                .active_request()
                .map(|request| (player.id.to_owned(), request))
        })
    }

    fn set_player_choice(&mut self, player_id: &str, input: &str) -> Result<(), Error> {
        let player = self.player_index_by_id(player_id)?;
        let mut context = self.player_context(player)?;
        Player::make_choice(&mut context, input)?;
        context
            .battle_mut()
            .input_log
            .get_mut(&player)
            .wrap_error_with_format(format_args!("input_log for player {player} does not exist"))?
            .push(input.to_owned());

        if self.all_player_choices_done()? {
            self.commit_choices()?;
        }

        Ok(())
    }
}

// Block for battle logic.
impl<'d> CoreBattle<'d> {
    fn initialize(&mut self) -> Result<(), Error> {
        for player in 0..self.players.len() {
            let mut context = self.player_context(player)?;
            Player::set_index(&mut context, player)?;
        }
        let mon_handles = self.all_mon_handles().collect::<Vec<_>>();
        for mon_handle in mon_handles {
            let mut context = self.mon_context(mon_handle)?;
            Mon::initialize(&mut context)?;
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

    fn start_team_preview(&mut self) -> Result<(), Error> {
        self.log(battle_event!("teampreviewstart"));
        let events = self
            .all_mons()
            .map(|res| match res {
                Err(err) => Err(err),
                Ok(mon) => Ok(battle_event!(
                    "mon",
                    self.player(mon.player)?.id,
                    mon.public_details()
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.log_many(events);
        match self.format.rules.numeric_rules.picked_team_size {
            Some(picked_team_size) => self
                .log
                .push(battle_event!("teampreview", picked_team_size)),
            None => self.log(battle_event!("teampreview")),
        }
        self.make_request(RequestType::TeamPreview)?;
        Ok(())
    }

    fn get_request_for_player(
        &mut self,
        player: usize,
        request_type: RequestType,
    ) -> Result<Request, Error> {
        match request_type {
            RequestType::TeamPreview => {
                let max_team_size = self
                    .format
                    .rules
                    .numeric_rules
                    .picked_team_size
                    .map(|size| size as usize);
                let context = self.player_context(player)?;
                Ok(Request::TeamPreview(TeamPreviewRequest {
                    max_team_size,
                    player: Player::request_data(&context)?,
                }))
            }
            RequestType::Turn => {
                let context = self.player_context(player)?;
                let active = Player::active_mon_handles(&context).collect::<Vec<_>>();
                let active = active
                    .into_iter()
                    .map(|mon| {
                        let context = self.mon_context(mon)?;
                        Mon::move_request(&context)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let context = self.player_context(player)?;
                Ok(Request::Turn(TurnRequest {
                    active,
                    player: Player::request_data(&context)?,
                }))
            }
            RequestType::Switch => todo!("switch requests are not yet implemented"),
        }
    }

    fn make_request(&mut self, request_type: RequestType) -> Result<(), Error> {
        self.request = Some(request_type);
        self.clear_requests()?;

        for player in 0..self.players.len() {
            let request = self.get_request_for_player(player, request_type)?;
            let mut context = self.player_context(player)?;
            context.player_mut().make_request(request);
        }
        Ok(())
    }

    fn all_player_choices_done(&mut self) -> Result<bool, Error> {
        for player in 0..self.players.len() {
            let context = self.player_context(player)?;
            if !Player::choice_done(&context) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn clear_requests(&mut self) -> Result<(), Error> {
        self.request = None;
        for player in 0..self.players.len() {
            let mut context = self.player_context(player)?;
            context.player_mut().clear_request();
            Player::clear_choice(&mut context);
        }
        Ok(())
    }

    fn commit_choices(&mut self) -> Result<(), Error> {
        let mut context = Context::new(self);
        // Take all player actions and insert them into the battle queue.
        let choices = context
            .players_mut()
            .map(|player| player.take_choice())
            .collect::<Vec<_>>();
        for choice in choices {
            BattleQueue::add_actions(&mut context, choice.actions.into_iter())?;
        }
        self.clear_requests()?;

        if self.engine_options.auto_continue {
            self.continue_battle_internal()?;
        }
        Ok(())
    }

    fn continue_battle_internal(&mut self) -> Result<(), Error> {
        self.log_current_time();

        self.request = None;

        if !self.mid_turn {
            BattleQueue::add_action(&mut self.context(), Action::BeforeTurn)?;
            BattleQueue::add_action(&mut self.context(), Action::Residual)?;
            self.mid_turn = true;
        }

        // Sort the new actions and continue the battle.
        self.queue.sort();

        // Run actions as long as possible.
        while let Some(action) = self.queue.pop_front() {
            self.run_action(action)?;
            // This action ended the game.
            if self.request.is_some() || self.ended {
                return Ok(());
            }
        }

        self.next_turn()?;
        self.mid_turn = false;
        Ok(())
    }

    fn run_action(&mut self, action: Action) -> Result<(), Error> {
        match action {
            Action::Start => {
                self.log_team_sizes();
                for player in self.players_mut() {
                    player.start_battle();
                }
                self.log(battle_event!("start"));

                let switch_ins =
                    self.players()
                        .filter(|player| player.mons_left() > 0)
                        .flat_map(|player| {
                            player.active.iter().enumerate().filter_map(|(i, _)| {
                                player.mons.get(i).cloned().map(|mon| (i, mon))
                            })
                        })
                        .collect::<Vec<_>>();
                for (position, mon) in switch_ins {
                    let mut context = self.mon_context(mon)?;
                    core_battle_actions::switch_in(&mut context, position)?;
                }
                self.mid_turn = true;
            }
            Action::Team(action) => {
                let mut context = self.mon_context(action.mon_action.mon)?;
                if action.index == 0 {
                    context.player_mut().mons.clear();
                }
                context.player_mut().mons.push(action.mon_action.mon);
            }
            Action::Switch(action) => {
                let mut context = self.mon_context(action.mon_action.mon)?;
                core_battle_actions::switch_in(&mut context, action.position)?;
            }
            Action::Move(action) => {
                let mut context = self.mon_context(action.mon_action.mon)?;
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
            Action::MegaEvo(action) => todo!("mega evolution is not implemented"),
            Action::Pass => (),
            Action::BeforeTurn => (),
            Action::Residual => {
                self.log(battle_event!("residual"));
            }
        }
        Ok(())
    }

    fn next_turn(&mut self) -> Result<(), Error> {
        self.turn += 1;
        self.log(battle_event!("turn", self.turn));

        if self.turn >= 1000 {
            self.log(battle_event!(
                "message",
                "It is turn 1000. You have hit the turn limit!"
            ));
            self.tie()?;
        }

        self.make_request(RequestType::Turn)?;
        Ok(())
    }

    fn tie(&mut self) -> Result<(), Error> {
        self.win(None)
    }

    fn win(&mut self, side: Option<usize>) -> Result<(), Error> {
        if self.ended {
            return Ok(());
        }

        self.log(battle_event!());
        match side {
            None => self.log(battle_event!("tie")),
            Some(side) => {
                let side = self.side(side)?;
                self.log(battle_event!("win", side.name));
            }
        }

        self.ended = true;
        self.clear_requests()?;
        Ok(())
    }

    fn calculate_action_priority(&mut self, action: &mut Action) -> Result<(), Error> {
        if let Action::Move(action) = action {
            let mov = self.dex.moves.get_by_id(&action.id).into_result()?;
            action.priority = mov.data.priority as i32;
            // TODO: Run priority modification events for the move and Mon.
        }
        if let Some(mon_action) = action.mon_action_mut() {
            let mut context = self.mon_context(mon_action.mon)?;
            mon_action.speed = Mon::action_speed(&mut context)? as u32;
        }
        Ok(())
    }

    pub fn resolve_action(&mut self, action: &mut Action) -> Result<(), Error> {
        if let Action::Move(action) = action {
            let mut context = self.mon_context(action.mon_action.mon)?;
            if let Some(target) = action.target {
                action.original_target = Mon::get_target(&mut context, target)?;
            }
        }
        self.calculate_action_priority(action)?;
        Ok(())
    }

    pub fn random_target(
        &mut self,
        mon: MonHandle,
        move_id: &Id,
    ) -> Result<Option<MonHandle>, Error> {
        let mov = self.dex.moves.get_by_id(move_id).into_result()?;
        let target = mov.data.target.clone();

        if target.can_target_user() {
            // Target the user if possible.
            return Ok(Some(mon));
        }

        let mut context = self.mon_context(mon)?;
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

        Ok(self
            .prng
            .sample_slice(&mons)
            .cloned()
            .map(|mon| Some(mon))
            .unwrap_or(None))
    }

    pub fn get_target(
        &mut self,
        mon: MonHandle,
        move_id: &Id,
        target: Option<isize>,
        original_target: Option<MonHandle>,
    ) -> Result<Option<MonHandle>, Error> {
        let mov = self.dex.moves.get_by_id(move_id).into_result()?;
        let tracks_target = mov.data.tracks_target;
        let smart_target = mov.data.smart_target;
        let move_target = mov.data.target.clone();

        if tracks_target {
            if let Some(original_target) = original_target {
                let context = self.mon_context(original_target)?;
                if context.mon().active {
                    // Move's original target is on the field.
                    return Ok(Some(original_target));
                }
            }
        }

        if smart_target {
            let mut context = self.mon_context(mon)?;
            if let Some(target) = target {
                if let Some(target) = Mon::get_target(&mut context, target)? {
                    return Ok(Some(target));
                }
            }
        }

        if let Some(target) = target {
            if !move_target.is_random() && move_target.valid_target(target) {
                let mut context = self.mon_context(mon)?;
                if let Some(target_mon_handle) = Mon::get_target(&mut context, target)? {
                    let target_mon = context.battle().mon(target_mon_handle)?;
                    if !target_mon.fainted || target_mon.is_ally(context.mon()) {
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
            self.random_target(mon, move_id)
        }
    }

    pub fn set_active_target(&mut self, target: Option<MonHandle>) -> Result<(), Error> {
        self.mon_mut(
            self.active_mon
                .wrap_error_with_message("cannot set an active target when no active mon is set")?,
        )?
        .active_target = target;
        Ok(())
    }

    pub fn set_active_move(
        &mut self,
        move_handle: MoveHandle,
        user: MonHandle,
        target: Option<MonHandle>,
    ) -> Result<(), Error> {
        self.active_mon = Some(user);
        self.mon_mut(user)?.set_active_move(move_handle, target);
        Ok(())
    }

    pub fn clear_active_move(&mut self) -> Result<(), Error> {
        if let Some(active_mon) = self.active_mon {
            self.mon_mut(active_mon)?.clear_active_move();
            self.active_mon = None;
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
}
