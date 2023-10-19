use std::{
    marker::PhantomPinned,
    mem,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use itertools::Itertools;
use uuid::Uuid;

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
        MonContext,
        Player,
        PlayerContext,
        PseudoRandomNumberGenerator,
        Request,
        RequestType,
        Side,
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
            .map(move |(player_index, player)| (player.id().to_owned(), player_index))
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

    pub(crate) fn sides(&self) -> impl Iterator<Item = &Side> {
        self.sides.iter()
    }

    pub(crate) fn sides_mut(&mut self) -> impl Iterator<Item = &mut Side> {
        self.sides.iter_mut()
    }

    pub(crate) fn side(&self, side: usize) -> Result<&Side, Error> {
        self.sides
            .get(side)
            .wrap_error_with_format(format_args!("side {side} does not exist"))
    }

    pub(crate) fn side_mut(&mut self, side: usize) -> Result<&mut Side, Error> {
        self.sides
            .get_mut(side)
            .wrap_error_with_format(format_args!("side {side} does not exist"))
    }

    pub(crate) fn players(&self) -> impl Iterator<Item = &Player> {
        self.players.iter()
    }

    pub(crate) fn players_mut(&mut self) -> impl Iterator<Item = &mut Player> {
        self.players.iter_mut()
    }

    pub(crate) fn player(&self, player: usize) -> Result<&Player, Error> {
        self.players
            .get(player)
            .wrap_error_with_format(format_args!("player {player} does not exist"))
    }

    pub(crate) fn player_mut(&mut self, player: usize) -> Result<&mut Player, Error> {
        self.players
            .get_mut(player)
            .wrap_error_with_format(format_args!("player {player} does not exist"))
    }

    pub(crate) fn players_on_side(&self, side: usize) -> impl Iterator<Item = &Player> {
        self.players().filter(move |player| player.side() == side)
    }

    fn player_index_by_id(&self, player_id: &str) -> Result<usize, Error> {
        self.player_ids
            .get(player_id)
            .wrap_error_with_format(format_args!("{player_id} does not exist"))
            .cloned()
    }

    pub(crate) fn all_mons_on_side(
        &self,
        side: &Side,
    ) -> impl Iterator<Item = Result<&Mon, Error>> {
        self.players_on_side(side.index())
            .map(|player| player.mons.iter())
            .flatten()
            .map(|mon| self.registry.mon(*mon))
    }

    pub(crate) fn all_mons(&self) -> impl Iterator<Item = Result<&Mon, Error>> {
        self.sides()
            .map(|side| self.all_mons_on_side(side))
            .flatten()
    }

    pub(crate) fn all_mons_checked(&self) -> Result<Vec<&Mon>, Error> {
        self.all_mons().collect()
    }

    pub(crate) fn next_ability_priority(&mut self) -> u32 {
        let next = self.next_ability_priority;
        self.next_ability_priority += 1;
        next
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
        let dex = Dex::new(data);
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
            .map(|player| battle_event!("player", player.id(), player.side(), player.position()))
            .collect::<Vec<_>>();
        self.log_many(player_logs);

        if self.has_team_preview() {
            self.log_team_sizes();
            self.start_team_preview()?;
        }

        let mut context = self.context();
        BattleQueue::add_action(&mut context, Action::Start);
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
                .map(|request| (player.id().to_owned(), request))
        })
    }

    fn set_player_choice(&mut self, player_id: &str, input: &str) -> Result<(), Error> {
        let player = self.player_index_by_id(player_id)?;
        let mut context = PlayerContext::new(self.context(), player)?;
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
            let mut context = PlayerContext::new(self.context(), player)?;
            Player::set_index(&mut context, player)?;
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
            .map(|player| battle_event!("teamsize", player.id(), player.mons.len()))
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
                Ok(mon) => Ok((mon.public_details(), self.player(mon.player)?.id())),
            })
            .map_ok(|(details, player_id)| battle_event!("mon", player_id, details))
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

    fn get_request_for_player(_: &mut PlayerContext, request_type: RequestType) -> Request {
        match request_type {
            RequestType::TeamPreview => Request::TeamPreview,
            RequestType::Turn => Request::Turn,
            RequestType::Switch => todo!("switch requests are not yet implemented"),
        }
    }

    fn make_request(&mut self, request_type: RequestType) -> Result<(), Error> {
        self.request = Some(request_type);
        for player in self.players_mut() {
            player.clear_request();
            player.clear_choice();
        }

        for player in 0..self.players.len() {
            let mut context = PlayerContext::new(self.context(), player)?;
            let request = Self::get_request_for_player(&mut context, request_type);
            context.player_mut().make_request(request);
        }
        Ok(())
    }

    fn all_player_choices_done(&mut self) -> Result<bool, Error> {
        for player in 0..self.players.len() {
            let context = PlayerContext::new(self.context(), player)?;
            if !Player::choice_done(&context) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn clear_requests(&mut self) {
        self.request = None;
        for player in self.players_mut() {
            player.clear_request();
            player.clear_choice();
        }
    }

    fn commit_choices(&mut self) -> Result<(), Error> {
        let mut context = Context::new(self);
        // Take all player actions and insert them into the battle queue.
        let choices = context
            .players_mut()
            .map(|player| player.take_choice())
            .collect::<Vec<_>>();
        for choice in choices {
            BattleQueue::add_actions(&mut context, choice.actions.into_iter());
        }
        self.clear_requests();

        if self.engine_options.auto_continue {
            self.continue_battle_internal()?;
        }
        Ok(())
    }

    fn continue_battle_internal(&mut self) -> Result<(), Error> {
        self.log(battle_event!());
        self.log_current_time();

        self.request = None;

        if !self.mid_turn {
            BattleQueue::add_action(&mut self.context(), Action::BeforeTurn);
            BattleQueue::add_action(&mut self.context(), Action::Residual);
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
                    let mut context = MonContext::new(self.context(), mon)?;
                    core_battle_actions::switch_in(&mut context, position)?;
                }
                self.mid_turn = true;
            }
            Action::Team(action) => {
                let mut context = MonContext::new(self.context(), action.mon)?;
                if action.index == 0 {
                    context.player_mut().mons.clear();
                }
                context.player_mut().mons.push(action.mon);
            }
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
                self.log(battle_event!("win", side.name()));
            }
        }

        self.ended = true;
        self.clear_requests();
        Ok(())
    }
}
