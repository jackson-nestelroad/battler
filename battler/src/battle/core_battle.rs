use std::{
    cmp::Ordering,
    collections::VecDeque,
    marker::PhantomPinned,
    mem,
    time::{
        SystemTime,
        UNIX_EPOCH,
    },
};

use ahash::HashMapExt;
use itertools::Itertools;
use zone_alloc::{
    ElementRef,
    ElementRefMut,
};

use crate::{
    battle::{
        core_battle_actions,
        core_battle_effects,
        core_battle_logs,
        speed_sort,
        Action,
        BattleOptions,
        BattleQueue,
        BattleRegistry,
        Context,
        CoreBattleEngineOptions,
        CoreBattleEngineRandomizeBaseDamage,
        CoreBattleOptions,
        EndAction,
        Field,
        LearnMoveRequest,
        Mon,
        MonContext,
        MonExitType,
        MonHandle,
        MoveHandle,
        Player,
        PlayerBattleData,
        PlayerContext,
        Request,
        RequestType,
        Side,
        SpeedOrderable,
        SwitchRequest,
        TeamPreviewRequest,
        TurnRequest,
    },
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
    effect::{
        fxlang,
        Effect,
        EffectHandle,
        EffectManager,
        LinkedEffectsManager,
    },
    items::ItemTarget,
    log::{
        Event,
        EventLog,
        EventLogEntryMut,
    },
    log_event,
    mons::{
        Type,
        TypeEffectiveness,
    },
    moves::{
        Move,
        MoveTarget,
        SwitchType,
    },
    rng::{
        rand_util,
        PseudoRandomNumberGenerator,
    },
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
        engine_options: CoreBattleEngineOptions,
    ) -> Result<Self, Error> {
        let internal = CoreBattle::from_builder(options, dex, format, engine_options)?;
        Ok(Self { internal })
    }

    /// Creates a new battle.
    pub fn new(
        options: CoreBattleOptions,
        data: &'d dyn DataStore,
        engine_options: CoreBattleEngineOptions,
    ) -> Result<Self, Error> {
        let internal = CoreBattle::new(options, data, engine_options)?;
        Ok(Self { internal })
    }

    /// Has the battle started?
    pub fn started(&self) -> bool {
        self.internal.started
    }

    /// Has the battle ended?
    pub fn ended(&self) -> bool {
        self.internal.ended
    }

    /// Does the battle have new battle logs since the last call to [`Self::new_logs`]?
    pub fn has_new_logs(&self) -> bool {
        self.internal.has_new_logs()
    }

    /// Returns all battle logs.
    pub fn all_logs(&self) -> impl Iterator<Item = &str> {
        self.internal.all_logs()
    }

    /// Returns new battle logs since the last call to [`Self::new_logs`].
    pub fn new_logs(&mut self) -> impl Iterator<Item = &str> {
        self.internal.new_logs()
    }

    /// Logs a new battle event to the battle log.
    pub fn log(&mut self, event: Event) {
        self.internal.log(event)
    }

    /// Logs many battle events to the battle log.
    pub fn log_many<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = Event>,
    {
        self.internal.log_many(events)
    }

    /// Starts the battle.
    pub fn start(&mut self) -> Result<(), Error> {
        self.internal.start()
    }

    /// Is the battle ready to continue?
    pub fn ready_to_continue(&mut self) -> Result<bool, Error> {
        self.internal.ready_to_continue()
    }

    /// Continues the battle.
    ///
    /// [`Self::ready_to_continue`] should return `Ok(true)` before this method
    /// is called.
    pub fn continue_battle(&mut self) -> Result<(), Error> {
        self.internal.continue_battle()
    }

    /// Returns the player data for the battle by player ID.
    ///
    /// Individual requests to players also contain this data, but this method can be useful for
    /// viewing for the player's team at other points in the battle and even after the battle ends.
    pub fn player_data(&mut self, player: &str) -> Result<PlayerBattleData, Error> {
        self.internal.player_data(player)
    }

    /// Returns all active requests for the battle, indexed by player ID.
    pub fn active_requests<'b>(&'b self) -> impl Iterator<Item = (String, Request)> + 'b {
        self.internal.active_requests()
    }

    /// Returns the active request for the player ID.
    pub fn request_for_player(&self, player: &str) -> Option<Request> {
        self.internal.request_for_player(player)
    }

    /// Sets the player's choice for their active request.
    pub fn set_player_choice(&mut self, player_id: &str, input: &str) -> Result<(), Error> {
        self.internal.set_player_choice(player_id, input)
    }
}

/// An entry in the faint queue.
pub struct FaintEntry {
    pub target: MonHandle,
    pub source: Option<MonHandle>,
    pub effect: Option<EffectHandle>,
}

/// An entry in the catch queue.
pub struct CatchEntry {
    pub target: MonHandle,
    pub player: usize,
    pub item: Id,
    pub shakes: u8,
    pub critical: bool,
}

/// An instance of a battle.
///
/// A battle has the following properties:
/// - Takes place on a single [`Field`][`crate::battle::Field`].
/// - Takes place between two [`Side`][`crate::battle::Side`]s.
/// - Receives input for a single [`Player`][`crate::battle::Player`].
/// - Features [`Mon`][`crate::battle::Mon`]s attacking one another in a turn-based manner.
/// - Adheres to a [`Format`][`crate::config::Format`].
///
/// All of the core battle logic runs through this object.
pub struct CoreBattle<'d> {
    log: EventLog,

    // SAFETY: None of the objects below should be overwritten or destroyed for the lifetime of the
    // battle.
    //
    // We could PinBox these, but that would complicate our code quite a bit.
    pub prng: Box<dyn PseudoRandomNumberGenerator>,
    pub dex: Dex<'d>,
    pub queue: BattleQueue,
    pub faint_queue: VecDeque<FaintEntry>,
    pub catch_queue: VecDeque<CatchEntry>,
    pub engine_options: CoreBattleEngineOptions,
    pub format: Format,
    pub field: Field,
    pub sides: [Side; 2],
    pub players: Vec<Player>,
    pub effect_manager: EffectManager,
    pub linked_effects_manager: LinkedEffectsManager,

    registry: BattleRegistry,
    player_ids: FastHashMap<String, usize>,
    effect_handle_cache: FastHashMap<Id, EffectHandle>,

    turn: u64,
    request: Option<RequestType>,
    mid_turn: bool,
    started: bool,
    in_pre_battle: bool,
    ending: bool,
    ended: bool,
    next_ability_order: u32,
    next_forfeit_order: u32,
    last_move_log: Option<usize>,
    last_exited: Option<MonHandle>,

    input_log: FastHashMap<usize, FastHashMap<u64, String>>,

    _pin: PhantomPinned,
}

// Block for constructors.
impl<'d> CoreBattle<'d> {
    fn new(
        mut options: CoreBattleOptions,
        data: &'d dyn DataStore,
        engine_options: CoreBattleEngineOptions,
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
        engine_options: CoreBattleEngineOptions,
    ) -> Result<Self, Error> {
        let prng = (engine_options.rng_factory)(options.seed);
        let log = EventLog::new();
        let registry = BattleRegistry::new();
        let queue = BattleQueue::new();
        let faint_queue = VecDeque::new();
        let catch_queue = VecDeque::new();
        let field = Field::new(options.field);
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

        let effect_manager = EffectManager::new();
        let linked_effects_manager = LinkedEffectsManager::new();

        let mut battle = Self {
            log,
            prng,
            dex,
            queue,
            faint_queue,
            catch_queue,
            engine_options,
            format,
            field,
            sides: [side_1, side_2],
            players,
            effect_manager,
            linked_effects_manager,
            registry,
            player_ids,
            effect_handle_cache: FastHashMap::new(),
            turn: 0,
            request: None,
            mid_turn: false,
            started: false,
            in_pre_battle: false,
            ending: false,
            ended: false,
            next_ability_order: 0,
            next_forfeit_order: 0,
            last_move_log: None,
            last_exited: None,
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

    pub fn side_indices(&self) -> impl Iterator<Item = usize> {
        0..self.sides.len()
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

    pub fn player_indices(&self) -> impl Iterator<Item = usize> {
        0..self.players.len()
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

    pub fn player_indices_on_side<'b>(&'b self, side: usize) -> impl Iterator<Item = usize> + 'b {
        (0..self.players.len()).filter(move |player| {
            self.players
                .get(*player)
                .is_some_and(|player| player.side == side)
        })
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

    pub unsafe fn active_move<'b>(
        &'b self,
        move_handle: MoveHandle,
    ) -> Result<ElementRef<'b, Move>, Error> {
        self.registry.active_move(move_handle)
    }

    pub unsafe fn active_move_mut<'b>(
        &'b self,
        move_handle: MoveHandle,
    ) -> Result<ElementRefMut<'b, Move>, Error> {
        self.registry.active_move_mut(move_handle)
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

    fn active_positions_on_side<'b>(
        &'b self,
        side: usize,
    ) -> impl Iterator<Item = Option<MonHandle>> + 'b {
        self.players_on_side(side)
            .map(|player| player.field_positions().map(|(_, pos)| pos.cloned()))
            .flatten()
    }

    pub fn active_mon_handles_on_side<'b>(
        &'b self,
        side: usize,
    ) -> impl Iterator<Item = MonHandle> + 'b {
        self.active_positions_on_side(side)
            .filter_map(|position| position)
    }

    pub fn all_active_mon_handles<'b>(&'b self) -> impl Iterator<Item = MonHandle> + 'b {
        self.side_indices()
            .map(|side| self.active_mon_handles_on_side(side))
            .flatten()
    }

    pub fn all_active_mon_handles_in_speed_order(
        context: &mut Context,
    ) -> Result<Vec<MonHandle>, Error> {
        let active_mons = context
            .battle()
            .all_active_mon_handles()
            .collect::<Vec<_>>();
        let mut active_mons_with_speed = Vec::with_capacity(active_mons.len());
        for mon in active_mons {
            active_mons_with_speed.push(Mon::speed_orderable(&context.mon_context(mon)?));
        }
        Self::speed_sort(context, active_mons_with_speed.as_mut_slice());
        Ok(active_mons_with_speed
            .into_iter()
            .map(|mon| mon.mon_handle)
            .collect())
    }

    pub fn next_ability_order(&mut self) -> u32 {
        let next = self.next_ability_order;
        self.next_ability_order += 1;
        next
    }

    pub fn next_forfeit_order(&mut self) -> u32 {
        let next = self.next_forfeit_order;
        self.next_forfeit_order += 1;
        next
    }

    pub fn side_length(&self, side: &Side) -> usize {
        self.players_on_side(side.index).count() * self.format.battle_type.active_per_player()
    }

    // A.k.a., `mons_per_side`.
    pub fn max_side_length(&self) -> usize {
        self.sides()
            .map(|side| self.side_length(side))
            .max()
            .unwrap_or(0)
    }

    pub fn turn(&self) -> u64 {
        self.turn
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

    fn player_data(&mut self, player: &str) -> Result<PlayerBattleData, Error> {
        let player = self.player_index_by_id(player)?;
        Player::request_data(&mut self.context().player_context(player)?)
    }

    fn active_requests<'b>(&'b self) -> impl Iterator<Item = (String, Request)> + 'b {
        self.players().filter_map(|player| {
            player
                .active_request()
                .map(|request| (player.id.to_owned(), request))
        })
    }

    fn request_for_player(&self, player: &str) -> Option<Request> {
        let player = self.player_index_by_id(player).ok()?;
        self.player(player).ok()?.active_request()
    }

    fn set_player_choice(&mut self, player_id: &str, input: &str) -> Result<(), Error> {
        Self::set_player_choice_internal(&mut self.context(), player_id, input)
    }
}

impl<'d> CoreBattle<'d> {
    pub fn log_private_public(&mut self, side: usize, private: Event, public: Event) {
        self.log
            .push_extend([log_event!("split", ("side", side)), private, public])
    }

    pub fn log(&mut self, event: Event) {
        self.log.push(event)
    }

    pub fn log_many<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = Event>,
    {
        self.log.push_extend(events)
    }

    pub fn log_move(&mut self, event: Event) {
        self.last_move_log = Some(self.log.len());
        self.log(event)
    }

    pub fn add_attribute_to_last_move(&mut self, attribute: &str) {
        if let Some(EventLogEntryMut::Uncommitted(event)) =
            self.last_move_log.and_then(|index| self.log.get_mut(index))
        {
            event.add_flag(attribute);
            if attribute == "noanim" {
                event.remove("target");
                event.remove("spread");
            }
        }
    }

    pub fn add_attribute_value_to_last_move(&mut self, attribute: &str, value: String) {
        if let Some(EventLogEntryMut::Uncommitted(event)) =
            self.last_move_log.and_then(|index| self.log.get_mut(index))
        {
            event.set(attribute, value);
        }
    }

    pub fn remove_attribute_from_last_move(&mut self, attribute: &str) {
        if let Some(EventLogEntryMut::Uncommitted(event)) =
            self.last_move_log.and_then(|index| self.log.get_mut(index))
        {
            event.remove(attribute);
        }
    }

    pub fn started(&self) -> bool {
        self.started
    }

    pub fn ending(&self) -> bool {
        self.ending
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
        context.battle_mut().in_pre_battle = true;

        let battle_type_event =
            log_event!("info", ("battletype", &context.battle().format.battle_type));
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
                .map(|rule_log| log_event!("info", ("rule", rule_log))),
        );

        let side_logs = context
            .battle()
            .sides()
            .map(|side| log_event!("side", ("id", side.index), ("name", &side.name)))
            .collect::<Vec<_>>();
        context.battle_mut().log_many(side_logs);

        if context.battle().format.battle_type.can_have_uneven_sides() {
            let event = log_event!(
                "maxsidelength",
                ("length", context.battle().max_side_length())
            );
            context.battle_mut().log(event);
        }

        // Before reporting player positions, shift players to try and center them as appropriate.
        for side in context.battle().side_indices() {
            let mut context = context.side_context(side)?;
            let players_on_side = context
                .battle()
                .players_on_side(context.side().index)
                .count();
            let players_on_foe_side = context
                .battle()
                .players_on_side(context.foe_side().index)
                .count();
            if players_on_foe_side > players_on_side {
                let shift_players_right_by = (players_on_foe_side - players_on_side) / 2;
                if shift_players_right_by > 0 {
                    for player in context
                        .battle()
                        .player_indices_on_side(context.side().index)
                        .collect::<Vec<_>>()
                    {
                        context
                            .as_battle_context_mut()
                            .player_context(player)?
                            .player_mut()
                            .position += shift_players_right_by;
                    }
                }
            }
        }

        let player_logs = context
            .battle()
            .players()
            .map(|player| {
                log_event!(
                    "player",
                    ("id", &player.id),
                    ("name", &player.name),
                    ("side", player.side),
                    ("position", player.position),
                )
            })
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

    fn time_now(&self) -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    }

    fn log_current_time(&mut self) {
        self.log(log_event!("time", ("value", self.time_now())));
    }

    fn log_team_sizes(&mut self) {
        let team_size_events = self
            .players()
            .filter(|player| !player.player_type.wild())
            .map(|player| {
                log_event!(
                    "teamsize",
                    ("player", &player.id),
                    ("size", player.mons.len()),
                )
            })
            .collect::<Vec<_>>();
        self.log_many(team_size_events);
    }

    fn has_team_preview(&self) -> bool {
        self.format.rules.has_rule(&Id::from_known("teampreview"))
    }

    fn start_team_preview(context: &mut Context) -> Result<(), Error> {
        context.battle_mut().log(log_event!("teampreviewstart"));
        let events = context
            .battle()
            .all_mon_handles()
            .collect::<Vec<_>>()
            .into_iter()
            .map(|mon_handle| {
                let context = context.mon_context(mon_handle)?;
                Ok(log_event!(
                    "mon",
                    ("player", &context.player().id),
                    Mon::public_details(&context)?,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        context.battle_mut().log_many(events);
        match context.battle().format.rules.numeric_rules.picked_team_size {
            Some(picked_team_size) => context
                .battle_mut()
                .log(log_event!("teampreview", ("pick", picked_team_size))),
            None => context.battle_mut().log(log_event!("teampreview")),
        }
        Self::make_request(context, RequestType::TeamPreview)?;
        Ok(())
    }

    fn get_request_for_player(
        context: &mut Context,
        player: usize,
        request_type: RequestType,
    ) -> Result<Option<Request>, Error> {
        match request_type {
            RequestType::TeamPreview => {
                let max_team_size = context
                    .battle()
                    .format
                    .rules
                    .numeric_rules
                    .picked_team_size
                    .map(|size| size as usize);
                Ok(Some(Request::TeamPreview(TeamPreviewRequest {
                    max_team_size,
                })))
            }
            RequestType::Turn => {
                let mut context = context.player_context(player)?;
                let active = context
                    .player()
                    .active_mon_handles()
                    .cloned()
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|mon| {
                        let mut context = context.mon_context(mon)?;
                        Mon::move_request(&mut context)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let ally_indices = context
                    .battle()
                    .player_indices_on_side(context.side().index)
                    .filter(|player| *player != context.player().index)
                    .collect::<Vec<_>>();
                let mut allies = Vec::with_capacity(ally_indices.len());
                for player in ally_indices {
                    let mut context = context.as_battle_context_mut().player_context(player)?;
                    allies.push(Player::request_data(&mut context)?);
                }
                Ok(Some(Request::Turn(TurnRequest { active, allies })))
            }
            RequestType::Switch => {
                // We only make a request if there are Mons that need to switch out.
                let context = context.player_context(player)?;
                if Player::mons_left(&context)? == 0 {
                    return Ok(None);
                }
                let mut needs_switch = Vec::new();
                for (slot, mon) in context.player().field_positions_with_active_or_exited_mon() {
                    if context.mon(*mon)?.needs_switch.is_some() {
                        needs_switch.push(slot);
                    }
                }
                if !needs_switch.is_empty() {
                    Ok(Some(Request::Switch(SwitchRequest { needs_switch })))
                } else {
                    Ok(None)
                }
            }
            RequestType::LearnMove => {
                let mut context = context.player_context(player)?;
                let mut learn_move_request = None;
                for mon in context.player().mon_handles().cloned().collect::<Vec<_>>() {
                    if let Some(request) = Mon::learn_move_request(&mut context.mon_context(mon)?)?
                    {
                        learn_move_request = Some(request);
                        break;
                    }
                }
                match learn_move_request {
                    Some(request) => Ok(Some(Request::LearnMove(LearnMoveRequest {
                        can_learn_move: request,
                    }))),
                    None => Ok(None),
                }
            }
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

        if context.battle().engine_options.auto_continue && Self::all_player_choices_done(context)?
        {
            Self::commit_choices(context)?;
        }

        Ok(())
    }

    fn make_request(context: &mut Context, request_type: RequestType) -> Result<(), Error> {
        context.battle_mut().log.commit();
        Self::clear_requests(context)?;
        context.battle_mut().request = Some(request_type);

        for player in 0..context.battle().players.len() {
            if let Some(request) = Self::get_request_for_player(context, player, request_type)? {
                let mut context = context.player_context(player)?;
                context.player_mut().make_request(request);
            }
        }
        Ok(())
    }

    fn all_player_choices_done(context: &mut Context) -> Result<bool, Error> {
        for player in 0..context.battle().players.len() {
            let mut context = context.player_context(player)?;
            if !Player::choice_done(&mut context)? {
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
        if !context.battle().engine_options.auto_continue {
            if !Self::all_player_choices_done(context)? {
                return Err(battler_error!(
                    "cannot continue: all players have not made their choice"
                ));
            }
            Self::commit_choices(context)?;
        }

        context.battle_mut().log_current_time();

        context.battle_mut().request = None;

        if !context.battle().mid_turn {
            BattleQueue::add_action(context, Action::BeforeTurn)?;
            BattleQueue::add_action(context, Action::Residual)?;
            context.battle_mut().mid_turn = true;
        }

        // Sort the new actions and continue the battle.
        BattleQueue::sort(context);

        // Run actions as long as possible.
        while let Some(action) = context.battle_mut().queue.pop_front() {
            Self::run_action(context, action)?;
            // This action initiated some request or ended the battle.
            if context.battle().request.is_some() || context.battle().ended {
                return Ok(());
            }
        }

        // We must drop all borrowed state before moving to the next turn.
        context.clear_context_cache();
        Self::next_turn(context)?;
        context.battle_mut().mid_turn = false;
        Ok(())
    }

    fn run_action(context: &mut Context, action: Action) -> Result<(), Error> {
        // Actions don't matter anymore if the battle ended.
        if context.battle().ended {
            return Ok(());
        }

        match &action {
            Action::Start => {
                context.battle_mut().log_team_sizes();
                context.battle_mut().in_pre_battle = false;

                context.battle_mut().log(log_event!("start"));

                let player_switch_in_orders = context
                    .battle()
                    .players()
                    .sorted_by(|a, b| match (a.player_type.wild(), b.player_type.wild()) {
                        (true, false) => Ordering::Less,
                        (false, true) => Ordering::Greater,
                        _ => Ordering::Equal,
                    })
                    .map(|player| player.index)
                    .collect::<Vec<_>>();
                for player in player_switch_in_orders {
                    let mut context = context.player_context(player)?;
                    let field_positions = context
                        .player()
                        .field_positions()
                        .map(|(pos, _)| pos)
                        .collect::<Vec<_>>();
                    for (mon, position) in Player::switchable_mon_handles(&context)
                        .cloned()
                        .zip(field_positions.into_iter())
                        .collect::<Vec<_>>()
                    {
                        let mut context = context.mon_context(mon)?;
                        core_battle_actions::switch_in(&mut context, position, None, false)?;
                    }
                }

                // TODO: Start event for species. Some forms changes happen at the very beginning of
                // the battle.

                // Clears the weather, which then sets the default weather.
                core_battle_actions::clear_weather(&mut context.field_effect_context(
                    EffectHandle::Condition(Id::from_known("start")),
                    None,
                    None,
                )?)?;

                // Clears the terrain, which then sets the default terrain.
                core_battle_actions::clear_terrain(&mut context.field_effect_context(
                    EffectHandle::Condition(Id::from_known("start")),
                    None,
                    None,
                )?)?;

                context.battle_mut().mid_turn = true;
            }
            Action::End(action) => {
                core_battle_effects::run_event_for_each_active_mon(
                    context,
                    fxlang::BattleEvent::EndBattle,
                )?;
                Self::win(context, action.winning_side)?;
            }
            Action::Team(action) => {
                let mut context = context.mon_context(action.mon_action.mon)?;
                if action.index == 0 {
                    context.player_mut().mons.clear();
                }
                context.mon_mut().team_position = action.index;
                context.player_mut().mons.push(action.mon_action.mon);
            }
            Action::Switch(action) => {
                let mut context = context.mon_context(action.mon_action.mon)?;
                core_battle_actions::switch_in(&mut context, action.position, None, false)?;
            }
            Action::SwitchEvents(action) => {
                let mut context = context.mon_context(action.mon_action.mon)?;
                core_battle_actions::run_switch_in_events(&mut context)?;
            }
            Action::Move(action) => {
                let mut context = context.mon_context(action.mon_action.mon)?;
                if !context.mon().active || !context.mon().active {
                    return Ok(());
                }
                core_battle_actions::do_move(
                    &mut context,
                    &action.id,
                    action.target,
                    action.original_target,
                )?;
            }
            Action::BeforeTurnMove(action) => {
                let mut context = context.mon_context(action.mon_action.mon)?;
                if !context.mon().active || !context.mon().active {
                    return Ok(());
                }
                core_battle_effects::run_applying_effect_event(
                    &mut context.applying_effect_context(
                        EffectHandle::InactiveMove(action.id.clone()),
                        None,
                        None,
                    )?,
                    fxlang::BattleEvent::BeforeTurn,
                    fxlang::VariableInput::default(),
                );
            }
            Action::PriorityChargeMove(action) => {
                let mut context = context.mon_context(action.mon_action.mon)?;
                if !context.mon().active || !context.mon().active {
                    return Ok(());
                }
                core_battle_effects::run_applying_effect_event(
                    &mut context.applying_effect_context(
                        EffectHandle::InactiveMove(action.id.clone()),
                        None,
                        None,
                    )?,
                    fxlang::BattleEvent::PriorityChargeMove,
                    fxlang::VariableInput::default(),
                );
            }
            Action::MegaEvo(_) => todo!("mega evolution is not implemented"),
            Action::Pass => (),
            Action::BeforeTurn => {
                for mon_handle in context
                    .battle()
                    .all_active_mon_handles()
                    .collect::<Vec<_>>()
                {
                    let foe_side = context.mon_context(mon_handle)?.foe_side().index;
                    for foe_handle in context
                        .battle()
                        .active_mon_handles_on_side(foe_side)
                        .collect::<Vec<_>>()
                    {
                        context
                            .mon_mut(foe_handle)?
                            .foes_fought_while_active
                            .insert(mon_handle);
                    }
                }
            }
            Action::Residual => {
                Self::clear_all_active_moves(context)?;
                Self::update_speed(context)?;
                core_battle_effects::run_event_for_residual(context, fxlang::BattleEvent::Residual);
                context.battle_mut().log(log_event!("residual"));
            }
            Action::Experience(action) => {
                core_battle_actions::gain_experience(
                    &mut context.mon_context(action.mon)?,
                    action.exp,
                )?;
            }
            Action::LevelUp(action) => {
                let mut context = context.mon_context(action.mon)?;
                let target_level = action.level.unwrap_or(context.mon().level + 1);
                core_battle_actions::level_up(&mut context, target_level)?;
            }
            Action::LearnMove(action) => {
                let mut context = context.mon_context(action.mon)?;
                let request = Mon::learn_move_request(&mut context)?.wrap_error_with_format(format_args!("mon {} has no move to learn, even though we allowed the player to choose to learn a move", context.mon().name))?;
                Mon::learn_move(&mut context, &request.id, action.forget_move_slot)?;
            }
            Action::Escape(action) => {
                core_battle_actions::try_escape(
                    &mut context.mon_context(action.mon_action.mon)?,
                    false,
                )?;
            }
            Action::Forfeit(action) => {
                core_battle_actions::forfeit(&mut context.player_context(action.player)?)?;
            }
            Action::Item(action) => {
                core_battle_actions::player_use_item(
                    &mut context.mon_context(action.mon_action.mon)?,
                    &action.item,
                    action.target,
                    core_battle_actions::PlayerUseItemInput {
                        move_slot: action.move_slot.clone(),
                    },
                )?;
            }
        }

        Self::after_action(context)?;
        Ok(())
    }

    fn after_action(context: &mut Context) -> Result<(), Error> {
        Self::faint_messages(context)?;
        Self::catch_messages(context)?;

        let mut some_move_can_be_learned = false;
        for mon in context.battle().all_mon_handles().collect::<Vec<_>>() {
            let mon = context.mon(mon)?;
            if !mon.learnable_moves.is_empty() {
                some_move_can_be_learned = true;
                break;
            }
        }
        if some_move_can_be_learned {
            Self::make_request(context, RequestType::LearnMove)?;
            return Ok(());
        }

        // Everything after this point does not matter if the battle is ending.
        if context.battle().ending {
            return Ok(());
        }

        // Drag out any Mons in the place of force switches.
        let mons = context
            .battle()
            .all_active_mon_handles()
            .collect::<Vec<_>>();
        for mon in mons {
            let mut context = context.mon_context(mon)?;
            if context.mon().force_switch.is_some() && context.mon().hp > 0 {
                if let Some(position) = context.mon().active_position {
                    core_battle_actions::drag_in(context.as_player_context_mut(), position)?;
                }
            }
            context.mon_mut().force_switch = None;
        }

        if context.battle().queue.is_empty() {
            // This sets that exited Mons must be switched out.
            //
            // We only do this at the end of the turn.
            Self::check_for_exited_mons(context)?;
        } else if let Some(Action::Switch(switch)) = context.battle().queue.peek() {
            // Instant switches should happen... instantly.
            if switch.instant {
                return Ok(());
            }
        }

        // TODO: Update speed dynamically, if we wish to support it like gen 8 does.

        Self::update(context)?;

        let mut some_switch_needed = false;
        for player in context.battle().player_indices() {
            let mut context = context.player_context(player)?;
            let needs_switch = Player::needs_switch(&context)?;
            let can_switch = Player::can_switch(&context);
            if needs_switch {
                if !can_switch {
                    // Switch can't happen, so unset the switch flag.
                    for mon in context
                        .player()
                        .active_or_exited_mon_handles()
                        .cloned()
                        .collect::<Vec<_>>()
                        .into_iter()
                    {
                        context.mon_mut(mon)?.needs_switch = None;
                    }
                } else {
                    // Switch out will occur mid turn.
                    //
                    // Run BeforeSwitchOut event now. This will make sure actions like Pursuit
                    // trigger on the same turn, rather than the next turn.
                    for mon in context
                        .player()
                        .active_or_exited_mon_handles()
                        .cloned()
                        .collect::<Vec<_>>()
                    {
                        let mut context = context.mon_context(mon)?;
                        if context.mon().needs_switch.is_some() {
                            if !context.mon().skip_before_switch_out {
                                core_battle_effects::run_event_for_mon(
                                    &mut context,
                                    fxlang::BattleEvent::BeforeSwitchOut,
                                    fxlang::VariableInput::default(),
                                );
                            }

                            context.mon_mut().skip_before_switch_out = true;

                            // Mon may have fainted here.
                            Self::faint_messages(context.as_battle_context_mut())?;
                            if context.battle().ending {
                                return Ok(());
                            }
                        }
                    }

                    // At this point, maybe the Mon that was going to switch fainted, so we should
                    // double check if the player still needs a switch.
                    some_switch_needed = some_switch_needed || Player::needs_switch(&context)?;
                }
            }
        }

        if some_switch_needed {
            Self::make_request(context, RequestType::Switch)?;
            return Ok(());
        }

        Ok(())
    }

    fn check_for_exited_mons(context: &mut Context) -> Result<(), Error> {
        for player in context.battle().player_indices() {
            let mut context = context.player_context(player)?;
            for mon in context
                .player()
                .active_or_exited_mon_handles()
                .cloned()
                .collect::<Vec<_>>()
            {
                let mut context = context.mon_context(mon)?;
                if context.mon().exited.is_some() {
                    context.mon_mut().needs_switch = Some(SwitchType::Normal);
                }
            }
        }
        Ok(())
    }

    fn next_turn(context: &mut Context) -> Result<(), Error> {
        context.battle_mut().turn += 1;

        if context.battle().turn >= 1000 {
            context.battle_mut().log(log_event!("turnlimit"));
            Self::schedule_tie(context)?;
            return Ok(());
        }

        for mon_handle in context
            .battle()
            .all_active_mon_handles()
            .collect::<Vec<_>>()
        {
            let mut context = context.mon_context(mon_handle)?;
            Mon::reset_state_for_next_turn(&mut context)?;

            if let Some(last_move) = context.mon().last_move {
                context
                    .battle()
                    .registry
                    .save_active_move_from_next_turn(last_move)?;
            }
            if let Some(last_move_used) = context.mon().last_move_used {
                context
                    .battle()
                    .registry
                    .save_active_move_from_next_turn(last_move_used)?;
            }
        }

        context.battle_mut().registry.next_turn()?;

        // TODO: Endless battle clause.

        let turn_event = log_event!("turn", ("turn", context.battle().turn));
        context.battle_mut().log(turn_event);

        Self::make_request(context, RequestType::Turn)?;
        Ok(())
    }

    fn schedule_tie(context: &mut Context) -> Result<(), Error> {
        Self::schedule_win(context, None)
    }

    fn schedule_win(context: &mut Context, mut side: Option<usize>) -> Result<(), Error> {
        if context.battle().ending {
            return Ok(());
        }

        if side.is_none() {
            // Resolve ties, if possible, using the last Mon that exited.
            if let Some(last_exited) = context.battle().last_exited {
                side = Some(context.mon(last_exited)?.side);
            }
        }
        BattleQueue::insert_action_into_sorted_position(
            context,
            Action::End(EndAction { winning_side: side }),
        )?;

        context.battle_mut().ending = true;
        Ok(())
    }

    fn win(context: &mut Context, side: Option<usize>) -> Result<(), Error> {
        match side {
            Some(side) => {
                context.battle_mut().log(log_event!("win", ("side", side)));
            }
            None => {
                context.battle_mut().log(log_event!("tie"));
            }
        }
        context.battle_mut().ended = true;
        context.battle_mut().log.commit();
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
            let move_id = mov.id().clone();
            action.priority = mov.data.priority as i32;
            action.sub_priority =
                core_battle_effects::run_event_for_mon_expecting_i32_quick_return(
                    &mut context.mon_context(action.mon_action.mon)?,
                    fxlang::BattleEvent::SubPriority,
                    fxlang::VariableInput::from_iter([fxlang::Value::Effect(
                        EffectHandle::InactiveMove(move_id),
                    )]),
                )
                .unwrap_or(0);
        }
        if let Action::Switch(action) = action {
            // The priority of switch actions are determined by the speed of the Mon switching out.
            let mut context = context.mon_context(action.switching_out)?;
            action.mon_action.speed = Mon::action_speed(&mut context)? as u32;
        } else if let Some(mon_action) = action.mon_action_mut() {
            let mut context = context.mon_context(mon_action.mon)?;
            mon_action.speed = Mon::action_speed(&mut context)? as u32;
        }
        Ok(())
    }

    /// Resolves the given action by calculating its priority in the context of the battle.
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

    /// Selects a random switchable Mon from the player.
    pub fn random_switchable(
        context: &mut Context,
        player: usize,
    ) -> Result<Option<MonHandle>, Error> {
        let prng = context.battle_mut().prng.as_mut();
        // SAFETY: PRNG is completely disjoint from the iterator created below.
        let prng = unsafe { mem::transmute(prng) };
        Ok(rand_util::sample_iter(
            prng,
            Player::switchable_mon_handles(&context.player_context(player)?),
        )
        .cloned())
    }

    /// Selects a random target for the move.
    pub fn random_target(
        context: &mut Context,
        mon: MonHandle,
        move_target: MoveTarget,
    ) -> Result<Option<MonHandle>, Error> {
        if move_target.can_target_user() {
            // Target the user if possible.
            return Ok(Some(mon));
        }

        let mut context = context.mon_context(mon)?;
        let mons = if !move_target.can_target_foes() {
            // Cannot target foes, so only consider allies.
            Mon::adjacent_allies(&mut context)?.collect::<Vec<_>>()
        } else if move_target.is_adjacent_only() {
            // Consider adjacent foes. Allies are excluded, so that a move will never randomly
            // target an ally if it doesn't need to.
            Mon::adjacent_foes(&mut context)?.collect::<Vec<_>>()
        } else {
            // Consider all foes.
            Mon::active_foes(&mut context).collect::<Vec<_>>()
        };

        let mons = mons
            .into_iter()
            .filter(|mon| {
                context
                    .as_battle_context()
                    .mon(*mon)
                    .is_ok_and(|mon| mon.hp > 0)
            })
            .collect::<Vec<_>>();

        Ok(
            rand_util::sample_slice(context.battle_mut().prng.as_mut(), &mons)
                .cloned()
                .map(|mon| Some(mon))
                .unwrap_or(None),
        )
    }

    /// Gets the selected target of the move.
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

        if let Some(target) = target {
            let mut context = context.mon_context(mon)?;
            if !move_target.is_random()
                && Self::valid_target(&mut context, move_target.clone(), target)?
            {
                if let Some(target_mon_handle) = Mon::get_target(&mut context, target)? {
                    let target_context = context
                        .as_battle_context_mut()
                        .mon_context(target_mon_handle)?;
                    if target_context.mon().active
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
            Self::random_target(context, mon, move_target)
        }
    }

    /// Gets the selected target of the move.
    pub fn get_item_target(
        context: &mut MonContext,
        item: &Id,
        target: Option<isize>,
    ) -> Result<Option<MonHandle>, Error> {
        let item = context.battle().dex.items.get_by_id(item).into_result()?;
        let item_target = item
            .data
            .target
            .wrap_error_with_message("item is not usable")?;

        match item_target {
            ItemTarget::Active => {
                return Ok(Some(context.mon_handle()));
            }
            ItemTarget::IsolatedFoe => {
                let foes = context
                    .battle()
                    .active_mon_handles_on_side(context.foe_side().index)
                    .collect::<Vec<_>>();
                if foes.len() != 1 {
                    return Ok(None);
                }
                return Ok(foes.first().cloned());
            }
            _ => (),
        }

        if let Some(target) = target {
            if Self::valid_item_target(context.as_player_context_mut(), item_target, target)? {
                if let Some(target_mon_handle) =
                    Player::get_item_target(context.as_player_context_mut(), target)?
                {
                    return Ok(Some(target_mon_handle));
                }
            }
        }

        Ok(None)
    }

    /// Checks if the selected target position is valid for the move.
    pub fn valid_target(
        context: &mut MonContext,
        move_target: MoveTarget,
        target_location: isize,
    ) -> Result<bool, Error> {
        if target_location == 0 {
            return Err(battler_error!("target position cannot be 0"));
        }
        let target_side = if target_location > 0 {
            context.foe_side().index
        } else {
            context.side().index
        };
        let target_location = target_location.abs() as usize;
        let target_location = target_location - 1;
        if !Mon::relative_location_of_target(&context, target_side, target_location).map_or(
            false,
            |relative_location| {
                move_target.valid_target(
                    relative_location,
                    context.battle().format.options.adjacency_reach,
                )
            },
        ) {
            return Ok(false);
        }
        Ok(true)
    }

    /// Checks if the selected target position is valid for the move.
    pub fn valid_item_target(
        context: &mut PlayerContext,
        item_target: ItemTarget,
        target_location: isize,
    ) -> Result<bool, Error> {
        match item_target {
            ItemTarget::Party => {
                if target_location >= 0 {
                    return Ok(false);
                }
                let team_slot = (-target_location) as usize;
                let team_slot = team_slot - 1;
                Ok(team_slot < context.player().mons.len())
            }
            ItemTarget::Active => Ok(false),
            ItemTarget::Foe => {
                if target_location <= 0 {
                    return Ok(false);
                }
                Ok(Side::mon_in_position(
                    &mut context.foe_side_context()?,
                    target_location as usize,
                )?
                .is_some())
            }
            ItemTarget::IsolatedFoe => {
                if target_location <= 0 {
                    return Ok(false);
                }
                if Side::active_mons_count(&mut context.foe_side_context()?) != 0 {
                    return Ok(false);
                }
                Ok(Side::mon_in_position(
                    &mut context.foe_side_context()?,
                    target_location as usize,
                )?
                .is_some())
            }
        }
    }

    /// Registers a new active move, returning its handle.
    pub fn register_move(&mut self, mov: Move) -> MoveHandle {
        self.registry.register_move(mov)
    }

    /// Clears all active moves for all Mons.
    pub fn clear_all_active_moves(context: &mut Context) -> Result<(), Error> {
        for mon in context
            .battle()
            .all_active_mon_handles()
            .collect::<Vec<_>>()
        {
            let mon = context.mon_mut(mon)?;
            mon.clear_active_move();
        }
        Ok(())
    }

    /// Updates the speed of all Mons.
    pub fn update_speed(context: &mut Context) -> Result<(), Error> {
        for mon_handle in context
            .battle()
            .all_active_mon_handles()
            .collect::<Vec<_>>()
        {
            Mon::update_speed(&mut context.mon_context(mon_handle)?)?;
        }
        Ok(())
    }

    /// Checks type immunity for several defensive types against an offensive type.
    pub fn check_type_immunity(&self, offense: Type, defense: &[Type]) -> bool {
        defense
            .iter()
            .map(|defense| {
                self.dex
                    .type_chart()
                    .types
                    .get(&offense)
                    .and_then(|row| row.get(&defense))
                    .unwrap_or(&TypeEffectiveness::Normal)
            })
            .any(|effectiveness| effectiveness == &TypeEffectiveness::None)
    }

    /// Checks the type effectiveness of an offensive type against a defensive type.
    pub fn check_type_effectiveness(&self, offense: Type, defense: Type) -> i8 {
        match self
            .dex
            .type_chart()
            .types
            .get(&offense)
            .and_then(|row| row.get(&defense))
            .unwrap_or(&TypeEffectiveness::Normal)
        {
            TypeEffectiveness::Strong => 1,
            TypeEffectiveness::Weak => -1,
            _ => 0,
        }
    }

    /// Randomizes damage, as part of the damage calculation formula.
    pub fn randomize_base_damage(&mut self, base_damage: u32) -> u32 {
        let random_factor = match self.engine_options.randomize_base_damage {
            CoreBattleEngineRandomizeBaseDamage::Randomize => {
                rand_util::range(self.prng.as_mut(), 0, 16) as u32
            }
            CoreBattleEngineRandomizeBaseDamage::Max => 0,
            CoreBattleEngineRandomizeBaseDamage::Min => 15,
        };
        base_damage * (100 - random_factor) / 100
    }

    /// Logs all faint messages.
    ///
    /// A Mon is considered truly fainted only after this method runs.
    pub fn faint_messages(context: &mut Context) -> Result<(), Error> {
        if context.battle().ending {
            return Ok(());
        }

        while let Some(entry) = context.battle_mut().faint_queue.pop_front() {
            let mut context = context.mon_context(entry.target)?;
            if !context.mon().active {
                continue;
            }

            // TODO: BeforeFaint event.
            core_battle_logs::faint(&mut context)?;

            let mon_handle = context.mon_handle();
            core_battle_actions::give_out_experience(context.as_battle_context_mut(), mon_handle)?;

            match entry.effect.clone() {
                Some(effect) => core_battle_effects::run_event_for_applying_effect(
                    &mut context.applying_effect_context(effect, entry.source, None)?,
                    fxlang::BattleEvent::Faint,
                    fxlang::VariableInput::default(),
                ),
                None => core_battle_effects::run_event_for_mon(
                    &mut context,
                    fxlang::BattleEvent::Faint,
                    fxlang::VariableInput::default(),
                ),
            };

            core_battle_effects::run_event_for_mon(
                &mut context,
                fxlang::BattleEvent::Exit,
                fxlang::VariableInput::default(),
            );

            Mon::clear_state_on_exit(&mut context, MonExitType::Fainted)?;
            context.battle_mut().last_exited = Some(context.mon_handle());
        }

        Self::check_win(context)?;

        Ok(())
    }

    /// Logs all catch messages.
    ///
    /// A Mon is considered truly caught only after this method runs.
    pub fn catch_messages(context: &mut Context) -> Result<(), Error> {
        while let Some(entry) = context.battle_mut().catch_queue.pop_front() {
            let mut context = context.mon_context(entry.target)?;

            core_battle_logs::catch(
                &mut context
                    .as_battle_context_mut()
                    .player_context(entry.player)?,
                entry.target,
                &entry.item,
                entry.shakes,
                entry.critical,
            )?;

            let mon_handle = context.mon_handle();
            core_battle_actions::give_out_experience(context.as_battle_context_mut(), mon_handle)?;

            core_battle_effects::run_event_for_mon(
                &mut context,
                fxlang::BattleEvent::Exit,
                fxlang::VariableInput::default(),
            );

            Mon::clear_state_on_exit(&mut context, MonExitType::Caught)?;
            context.battle_mut().last_exited = Some(context.mon_handle());

            context.mon_mut().ball = context
                .battle()
                .dex
                .items
                .get_by_id(&entry.item)?
                .data
                .name
                .clone();

            context
                .as_battle_context_mut()
                .player_context(entry.player)?
                .player_mut()
                .caught
                .push(entry.target);
        }

        Self::check_win(context)?;

        Ok(())
    }

    /// Checks if anyone has won the battle.
    pub fn check_win(context: &mut Context) -> Result<(), Error> {
        if context.battle().in_pre_battle {
            return Ok(());
        }

        let mut winner = None;
        for side in context.battle().side_indices() {
            let mut context = context.side_context(side)?;
            let mons_left = Side::mons_left(&mut context)?;
            if mons_left > 0 {
                if winner.is_some() {
                    return Ok(());
                }
                winner = Some(side);
            }
        }
        Self::schedule_win(context, winner)
    }

    /// Gets an [`EffectHandle`] by name.
    pub fn get_effect_handle(&mut self, name: &str) -> Result<&EffectHandle, Error> {
        self.get_effect_handle_by_id(&Id::from(name))
    }

    /// Gets an [`EffectHandle`] by ID.
    ///
    /// An [`Effect`] has many variants. An ID is not enough on its own to lookup a generic effect.
    /// For the duration of a battle, an ID will map to a single [`EffectHandle`]. This method
    /// handles the caching of this translation.
    pub fn get_effect_handle_by_id(&mut self, id: &Id) -> Result<&EffectHandle, Error> {
        if self.effect_handle_cache.contains_key(id) {
            return self.effect_handle_cache.get(id).wrap_error_with_message(
                "effect handle not found in cache after its key was found",
            );
        }

        let effect_handle = Self::lookup_effect_in_dex(self, id.clone());
        self.effect_handle_cache.insert(id.clone(), effect_handle);
        self.effect_handle_cache
            .get(id)
            .wrap_error_with_message("effect handle not found in cache after insertion")
    }

    fn lookup_effect_in_dex(&self, id: Id) -> EffectHandle {
        if self.dex.conditions.get_by_id(&id).into_option().is_some() {
            return EffectHandle::Condition(id);
        }
        if self.dex.moves.get_by_id(&id).into_option().is_some() {
            return EffectHandle::MoveCondition(id);
        }
        if self.dex.abilities.get_by_id(&id).into_option().is_some() {
            return EffectHandle::AbilityCondition(id);
        }
        if self.dex.items.get_by_id(&id).into_option().is_some() {
            return EffectHandle::ItemCondition(id);
        }
        EffectHandle::NonExistent(id)
    }

    /// Gets an [`Effect`] by handle.
    ///
    /// [`EffectHandle`] is considered a stable way to look up any effect in the dex.
    pub fn get_effect_by_handle<'context>(
        context: &'context Context,
        effect_handle: &EffectHandle,
    ) -> Result<Effect<'context>, Error> {
        match effect_handle {
            EffectHandle::ActiveMove(active_move_handle, hit_effect_type) => {
                Ok(Effect::for_active_move(
                    context.active_move_mut(*active_move_handle)?,
                    *hit_effect_type,
                ))
            }
            EffectHandle::MoveCondition(id) => Ok(Effect::for_move_condition(
                context.battle().dex.moves.get_by_id(id).into_result()?,
            )),
            EffectHandle::InactiveMove(id) => Ok(Effect::for_inactive_move(
                context.battle().dex.moves.get_by_id(id).into_result()?,
            )),
            EffectHandle::Ability(id) => Ok(Effect::for_ability(
                context.battle().dex.abilities.get_by_id(id).into_result()?,
            )),
            EffectHandle::AbilityCondition(id) => Ok(Effect::for_ability_condition(
                context.battle().dex.abilities.get_by_id(id).into_result()?,
            )),
            EffectHandle::Condition(id) => Ok(Effect::for_condition(
                context
                    .battle()
                    .dex
                    .conditions
                    .get_by_id(id)
                    .into_result()?,
            )),
            EffectHandle::Item(id) => Ok(Effect::for_item(
                context.battle().dex.items.get_by_id(id).into_result()?,
            )),
            EffectHandle::ItemCondition(id) => Ok(Effect::for_item_condition(
                context.battle().dex.items.get_by_id(id).into_result()?,
            )),
            EffectHandle::NonExistent(id) => Ok(Effect::for_non_existent(id.clone())),
        }
    }

    /// Gets an [`Effect`] by ID.
    pub fn get_effect_by_id<'context>(
        context: &'context mut Context,
        id: &Id,
    ) -> Result<Effect<'context>, Error> {
        let effect_handle = context.battle_mut().get_effect_handle_by_id(id)?.clone();
        Self::get_effect_by_handle(context, &effect_handle)
    }

    /// Sorts the given items by speed.
    pub fn speed_sort<T>(context: &mut Context, items: &mut [T])
    where
        for<'a> &'a T: SpeedOrderable,
    {
        let prng = context.battle_mut().prng.as_mut();
        // SAFETY: PRNG and sorting are completely disjoint.
        let prng = unsafe { mem::transmute(prng) };
        let tie_resolution = context.battle().engine_options.speed_sort_tie_resolution;
        speed_sort(items, prng, tie_resolution);
    }

    /// Updates the battle, triggering any miscellaneous effects on Mons that could activate.
    pub fn update(context: &mut Context) -> Result<(), Error> {
        core_battle_effects::run_event_for_each_active_mon_with_effect(
            &mut context.effect_context(EffectHandle::Condition(Id::from_known("update")), None)?,
            fxlang::BattleEvent::Update,
        )
    }
}
