use std::{
    fmt::{
        self,
        Display,
    },
    iter,
    mem,
    ops::Mul,
    u8,
};

use ahash::{
    HashMapExt,
    HashSetExt,
};
use lazy_static::lazy_static;
use serde::{
    Deserialize,
    Serialize,
};
use zone_alloc::ElementRef;

use crate::{
    battle::{
        calculate_hidden_power_type,
        calculate_mon_stats,
        core_battle::FaintEntry,
        core_battle_actions,
        core_battle_effects,
        core_battle_logs,
        modify_32,
        mon_states,
        Boost,
        BoostTable,
        CoreBattle,
        MonContext,
        MonHandle,
        MoveHandle,
        MoveOutcome,
        Player,
        Side,
        SpeedOrderable,
    },
    battler_error,
    common::{
        Error,
        FastHashMap,
        FastHashSet,
        Fraction,
        Id,
        Identifiable,
        WrapResultError,
    },
    dex::Dex,
    effect::{
        fxlang,
        EffectHandle,
    },
    log::{
        Event,
        EventLoggable,
    },
    log_event,
    mons::{
        Gender,
        Nature,
        PartialStatTable,
        Species,
        Stat,
        StatTable,
        Type,
    },
    moves::{
        Move,
        MoveTarget,
        SwitchType,
    },
    teams::MonData,
};

/// Public [`Mon`] details, which are shared to both sides of a battle when the Mon
/// appears or during Team Preview.
pub struct PublicMonDetails {
    pub species: String,
    pub level: u8,
    pub gender: Gender,
    pub shiny: bool,
}

impl EventLoggable for PublicMonDetails {
    fn log(&self, event: &mut Event) {
        event.set("species", self.species.clone());
        event.set("level", self.level);
        event.set("gender", &self.gender);
        if self.shiny {
            event.add_flag("shiny");
        }
    }
}

/// Public details for an active [`Mon`], which are shared to both sides of a battle when the Mon
/// appears in the battle.
pub struct ActiveMonDetails {
    pub public_details: PublicMonDetails,
    pub name: String,
    pub player_id: String,
    pub side_position: usize,
    pub health: String,
    pub status: String,
}

impl EventLoggable for ActiveMonDetails {
    fn log(&self, event: &mut Event) {
        event.set("player", self.player_id.clone());
        event.set("position", self.side_position);
        event.set("name", self.name.clone());
        event.set("health", &self.health);
        if !self.status.is_empty() {
            event.set("status", &self.status);
        }
        self.public_details.log(event);
    }
}

/// Public details for an active [`Mon`]'s position.
pub struct MonPositionDetails {
    pub name: String,
    pub player_id: String,
    pub side_position: Option<usize>,
}

impl Display for MonPositionDetails {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.side_position {
            Some(position) => write!(f, "{},{},{}", self.name, self.player_id, position),
            None => write!(f, "{},{}", self.name, self.player_id),
        }
    }
}

/// A single move slot for a Mon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveSlot {
    pub id: Id,
    pub name: String,
    pub pp: u8,
    pub max_pp: u8,
    pub pp_boosts: u8,
    pub target: MoveTarget,
    pub disabled: bool,
    pub used: bool,
    pub simulated: bool,
}

impl MoveSlot {
    /// Creates a new move slot.
    pub fn new(
        id: Id,
        name: String,
        pp: u8,
        max_pp: u8,
        pp_boosts: u8,
        target: MoveTarget,
    ) -> Self {
        Self {
            id,
            name,
            pp,
            max_pp,
            pp_boosts,
            target,
            disabled: false,
            used: false,
            simulated: false,
        }
    }

    /// Creates a new simulated move slot.
    pub fn new_simulated(id: Id, name: String, pp: u8, max_pp: u8, target: MoveTarget) -> Self {
        Self {
            id,
            name,
            pp,
            max_pp,
            pp_boosts: 0,
            target,
            disabled: false,
            used: false,
            simulated: true,
        }
    }
}

/// A single ability slot for a Mon.
#[derive(Clone)]
pub struct AbilitySlot {
    pub id: Id,
    pub order: u32,
    pub effect_state: fxlang::EffectState,
}

/// A single item slot for a Mon.
#[derive(Clone)]
pub struct ItemSlot {
    pub id: Id,
    pub effect_state: fxlang::EffectState,
}

/// Data for a single move on a [`Mon`].
///
/// Makes a copy of underlying data so that it can be stored on move requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonMoveSlotData {
    pub name: String,
    pub id: Id,
    pub pp: u8,
    pub max_pp: u8,
    pub target: Option<MoveTarget>,
    pub disabled: bool,
}

impl MonMoveSlotData {
    pub fn from(context: &mut MonContext, move_slot: &MoveSlot) -> Result<Self, Error> {
        let mov = context.battle().dex.moves.get_by_id(&move_slot.id)?;
        let name = mov.data.name.clone();
        let id = mov.id().clone();
        // Some moves may have a special target for non-Ghost types.
        let target = if let Some(non_ghost_target) = mov.data.non_ghost_target {
            if !Mon::has_type(context, Type::Ghost)? {
                non_ghost_target
            } else {
                move_slot.target
            }
        } else {
            move_slot.target
        };
        let mut disabled = move_slot.disabled;
        if move_slot.pp == 0 {
            disabled = true;
        }
        Ok(Self {
            name,
            id,
            pp: move_slot.pp,
            max_pp: move_slot.max_pp,
            target: Some(target),
            disabled,
        })
    }
}

/// Data about a single [`Mon`], shared across [`MonBattleRequestData`] and
/// [`MonSummaryRequestData`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonBaseRequestData {
    pub name: String,
    pub level: u8,
    pub gender: Gender,
    pub shiny: bool,
    pub ball: String,
}

/// Data about a single [`Mon`]'s battle state when a player is requested an action on their entire
/// team.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonBattleRequestData {
    #[serde(flatten)]
    pub base_data: MonBaseRequestData,
    pub species: String,
    pub health: String,
    pub types: Vec<Type>,
    pub status: String,
    pub active: bool,
    pub player_active_position: Option<usize>,
    pub side_position: Option<usize>,
    pub stats: PartialStatTable,
    pub moves: Vec<MonMoveSlotData>,
    pub ability: String,
    pub item: Option<String>,
}

/// Data about a single [`Mon`]'s base, unmodified state when a player is requested an action that
/// acts on base state (such as learning a move) or requests a summary of their team.
///
/// Very similar to [`MonBattleRequestData`], but some fields related to battle state are removed.
///
/// In most cases, clients should have their own external view of Mons in a battle. However, since
/// we make requests for things like learning moves that can alter that external state, we also
/// supply our own simple view of this type of data for convenience.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonSummaryRequestData {
    #[serde(flatten)]
    pub base_data: MonBaseRequestData,
    pub species: String,
    pub stats: StatTable,
    pub moves: Vec<MonMoveSlotData>,
    pub ability: String,
}

/// Request for a single [`Mon`] to move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonMoveRequest {
    pub team_position: usize,
    pub moves: Vec<MonMoveSlotData>,
    #[serde(default)]
    pub trapped: bool,
    #[serde(default)]
    pub can_mega_evo: bool,
}

/// Request for a single [`Mon`] to learn one or more moves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonLearnMoveRequest {
    pub team_position: usize,
    pub id: Id,
    pub name: String,
}

/// An interface that implements [`SpeedOrderable`][`crate::battle::SpeedOrderable`] for [`Mon`]s.
pub struct SpeedOrderableMon {
    pub mon_handle: MonHandle,
    pub speed: u32,
}

impl SpeedOrderable for SpeedOrderableMon {
    fn order(&self) -> u32 {
        0
    }

    fn priority(&self) -> i32 {
        0
    }

    fn sub_priority(&self) -> i32 {
        0
    }

    fn speed(&self) -> u32 {
        self.speed
    }

    fn sub_order(&self) -> u32 {
        0
    }
}

/// Information about a single attack received by a [`Mon`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceivedAttackEntry {
    pub source: MonHandle,
    pub source_side: usize,
    pub source_position: usize,
    pub damage: u16,
    pub turn: u64,
}

/// The context of an effect that is triggering a stat calculation.
#[derive(Clone)]
pub struct CalculateStatContext {
    pub effect: EffectHandle,
    pub source: MonHandle,
}

/// A Mon in a battle, which battles against other Mons.
pub struct Mon {
    pub player: usize,
    pub side: usize,

    pub name: String,
    pub base_species: Id,
    pub species: Id,

    /// `true` if the Mon is in an active position.
    ///
    /// The Mon may or may not be fainted.
    pub active: bool,
    pub active_turns: u32,
    pub active_move_actions: u32,
    pub active_position: Option<usize>,
    pub old_active_position: Option<usize>,
    pub team_position: usize,

    pub base_stored_stats: StatTable,
    pub stats: StatTable,
    pub boosts: BoostTable,
    pub ivs: StatTable,
    pub evs: StatTable,
    pub level: u8,
    pub experience: u32,
    pub friendship: u8,
    pub nature: Nature,
    pub gender: Gender,
    pub shiny: bool,
    pub ball: String,
    pub different_original_trainer: bool,

    pub base_move_slots: Vec<MoveSlot>,
    pub move_slots: Vec<MoveSlot>,

    pub base_ability: AbilitySlot,
    pub ability: AbilitySlot,

    pub types: Vec<Type>,
    pub hidden_power_type: Type,

    pub item: Option<ItemSlot>,
    pub last_item: Option<Id>,

    pub hp: u16,
    pub base_max_hp: u16,
    pub max_hp: u16,
    pub speed: u16,
    pub weight: u32,
    pub fainted: bool,
    pub newly_switched: bool,
    pub needs_switch: Option<SwitchType>,
    pub force_switch: Option<SwitchType>,
    pub skip_before_switch_out: bool,
    pub being_called_back: bool,
    pub trapped: bool,
    pub cannot_receive_items: bool,
    pub can_mega_evo: bool,
    pub transformed: bool,

    /// The move the Mon is actively performing.
    pub active_move: Option<MoveHandle>,
    /// The last move selected.
    pub last_move_selected: Option<Id>,
    /// The last move used for the Mon.
    pub last_move: Option<MoveHandle>,
    /// The last move used by the Mon, which can be different from `last_move` if that
    /// move executed a different move (like Metronome).
    pub last_move_used: Option<MoveHandle>,

    pub move_this_turn_outcome: Option<MoveOutcome>,
    pub last_move_target_location: Option<isize>,
    pub hurt_this_turn: u16,
    pub stats_raised_this_turn: bool,
    pub stats_lowered_this_turn: bool,
    pub item_used_this_turn: bool,
    pub foes_fought_while_active: FastHashSet<MonHandle>,
    pub received_attacks: Vec<ReceivedAttackEntry>,

    pub status: Option<Id>,
    pub status_state: fxlang::EffectState,
    pub volatiles: FastHashMap<Id, fxlang::EffectState>,

    pub learnable_moves: Vec<Id>,
}

// Construction and initialization logic.
impl Mon {
    /// Creates a new Mon.
    pub fn new(data: MonData, team_position: usize, dex: &Dex) -> Result<Self, Error> {
        let name = data.name;
        let species = Id::from(data.species);
        let ivs = data.ivs;
        let evs = data.evs;
        let level = data.level;
        let experience = data.experience;
        let friendship = data.friendship;
        let nature = data.nature;
        let gender = data.gender;
        let shiny = data.shiny;
        let ball = data.ball;
        let different_original_trainer = data.different_original_trainer;

        let mut base_move_slots = Vec::with_capacity(data.moves.len());
        for (i, move_name) in data.moves.iter().enumerate() {
            let mov = dex.moves.get(move_name)?;
            let (max_pp, pp_boosts) = if mov.data.no_pp_boosts {
                (mov.data.pp, 0)
            } else {
                let pp_boosts = data.pp_boosts.get(i).cloned().unwrap_or(0).min(3);
                (
                    ((mov.data.pp as u32) * (pp_boosts as u32 + 5) / 5) as u8,
                    pp_boosts,
                )
            };
            base_move_slots.push(MoveSlot::new(
                mov.id().clone(),
                mov.data.name.clone(),
                max_pp,
                max_pp,
                pp_boosts,
                mov.data.target.clone(),
            ));
        }

        let move_slots = base_move_slots.clone();

        let ability = AbilitySlot {
            id: Id::from(data.ability),
            order: 0,
            effect_state: fxlang::EffectState::new(),
        };

        let item = match data.item {
            Some(item) => Some(ItemSlot {
                id: Id::from(item),
                effect_state: fxlang::EffectState::new(),
            }),
            None => None,
        };

        let hidden_power_type = data
            .hidden_power_type
            .unwrap_or(calculate_hidden_power_type(&ivs));

        Ok(Self {
            player: usize::MAX,
            side: usize::MAX,

            name,
            base_species: species.clone(),
            species,

            active: false,
            active_turns: 0,
            active_move_actions: 0,
            active_position: None,
            old_active_position: None,
            team_position,

            base_stored_stats: StatTable::default(),
            stats: StatTable::default(),
            boosts: BoostTable::default(),
            ivs,
            evs,
            level,
            experience,
            friendship,
            nature,
            gender,
            shiny,
            ball,
            different_original_trainer,

            base_move_slots,
            move_slots,

            base_ability: ability.clone(),
            ability,

            types: Vec::new(),
            hidden_power_type,

            item,
            last_item: None,

            hp: 0,
            base_max_hp: 0,
            max_hp: 0,
            speed: 0,
            weight: 1,
            fainted: false,
            newly_switched: false,
            needs_switch: None,
            force_switch: None,
            skip_before_switch_out: false,
            being_called_back: false,
            trapped: false,
            cannot_receive_items: false,
            can_mega_evo: false,
            transformed: false,

            active_move: None,
            last_move_selected: None,
            last_move: None,
            last_move_used: None,

            move_this_turn_outcome: None,
            last_move_target_location: None,
            hurt_this_turn: 0,
            stats_raised_this_turn: false,
            stats_lowered_this_turn: false,
            item_used_this_turn: false,
            foes_fought_while_active: FastHashSet::new(),
            received_attacks: Vec::new(),

            status: None,
            status_state: fxlang::EffectState::new(),
            volatiles: FastHashMap::new(),

            learnable_moves: Vec::new(),
        })
    }

    /// Initializes a Mon for battle.
    ///
    /// This *must* be called at the very beginning of a battle, as it sets up important fields on
    /// the Mon, such as its stats.
    pub fn initialize(context: &mut MonContext) -> Result<(), Error> {
        let base_species = context.mon().base_species.clone();
        Self::set_base_species(context, &base_species, false)?;

        Self::clear_volatile(context, true)?;
        Self::recalculate_base_stats(context)?;

        // Generate level from experience points if needed.
        if context.mon().level == u8::default() {
            let species = context
                .battle()
                .dex
                .species
                .get_by_id(&context.mon().species)?;
            context.mon_mut().level = species
                .data
                .leveling_rate
                .level_from_exp(context.mon().experience);
        }

        context.mon_mut().hp = context.mon().max_hp;

        context.mon_mut().ability.effect_state = context.mon().base_ability.effect_state.clone();

        if let Some(item) = &context.mon().item {
            let mon_handle = context.mon_handle();
            let item = context.battle().dex.items.get_by_id(&item.id)?;
            context.mon_mut().item = Some(ItemSlot {
                id: item.id().clone(),
                effect_state: fxlang::EffectState::initial_effect_state(
                    &mut context
                        .as_battle_context_mut()
                        .effect_context(EffectHandle::Condition(Id::from_known("start")), None)?,
                    Some(mon_handle),
                    None,
                )?,
            })
        }

        Ok(())
    }
}

// Basic getters.
impl Mon {
    fn health(&self, actual_health: bool) -> String {
        if actual_health {
            return self.actual_health();
        }
        if self.hp == 0 || self.max_hp == 0 {
            return "0".to_owned();
        }
        let ratio = Fraction::new(self.hp, self.max_hp);
        // Always round up to avoid returning 0 when the Mon is not fainted.
        let mut percentage = (ratio * 100).ceil();
        if percentage == 100 && ratio < Fraction::new(1, 1) {
            percentage = 99;
        }
        format!("{percentage}/100")
    }

    fn actual_health(&self) -> String {
        if self.hp == 0 || self.max_hp == 0 {
            return "0".to_owned();
        }
        format!("{}/{}", self.hp, self.max_hp)
    }

    /// The public details for the Mon.
    pub fn public_details(context: &MonContext) -> Result<PublicMonDetails, Error> {
        let species = context
            .battle()
            .dex
            .species
            .get_by_id(&context.mon().species)?
            .data
            .name
            .clone();
        Ok(PublicMonDetails {
            species,
            level: context.mon().level,
            gender: context.mon().gender.clone(),
            shiny: context.mon().shiny,
        })
    }

    fn active_details(context: &mut MonContext, secret: bool) -> Result<ActiveMonDetails, Error> {
        let status = match context.mon().status.clone() {
            Some(status) => CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &status)?
                .name()
                .to_owned(),
            None => String::new(),
        };
        Ok(ActiveMonDetails {
            public_details: Self::public_details(context)?,
            name: context.mon().name.clone(),
            player_id: context.player().id.clone(),
            side_position: Self::position_on_side(context)
                .wrap_error_with_message("expected mon to be active")?
                + 1,
            health: if secret {
                context.mon().actual_health()
            } else {
                Self::public_health(context)
            },
            status,
        })
    }

    /// The public details for the active Mon.
    pub fn public_active_details(context: &mut MonContext) -> Result<ActiveMonDetails, Error> {
        Self::active_details(context, false)
    }

    /// The private details for the active Mon.
    pub fn private_active_details(context: &mut MonContext) -> Result<ActiveMonDetails, Error> {
        Self::active_details(context, true)
    }

    /// The public details for the Mon when an action is made.
    pub fn position_details(context: &MonContext) -> Result<MonPositionDetails, Error> {
        Ok(MonPositionDetails {
            name: context.mon().name.clone(),
            player_id: context.player().id.clone(),
            side_position: Self::position_on_side(context).map(|position| position + 1),
        })
    }

    /// Same as [`Self::position_details`], but it also considers the previous active position of
    /// the Mon.
    pub fn position_details_or_previous(context: &MonContext) -> Result<MonPositionDetails, Error> {
        Ok(MonPositionDetails {
            name: context.mon().name.clone(),
            player_id: context.player().id.clone(),
            side_position: Self::position_on_side_or_previous(context).map(|position| position + 1),
        })
    }

    /// The public health of the Mon.
    pub fn public_health(context: &MonContext) -> String {
        context
            .mon()
            .health(context.battle().engine_options.reveal_actual_health)
    }

    /// The secret health of the Mon.
    pub fn secret_health(context: &MonContext) -> String {
        context.mon().actual_health()
    }

    /// Looks up the Mon's types, which may be dynamic based on volatile effects.
    pub fn types(context: &mut MonContext) -> Result<Vec<Type>, Error> {
        let types = core_battle_effects::run_event_for_mon_expecting_types(
            context,
            fxlang::BattleEvent::Types,
            context.mon().types.clone(),
        );
        if !types.is_empty() {
            return Ok(types);
        }
        return Ok(Vec::from_iter([Type::Normal]));
    }

    /// Checks if the Mon has the given type.
    pub fn has_type(context: &mut MonContext, typ: Type) -> Result<bool, Error> {
        let types = Self::types(context)?;
        return Ok(types.contains(&typ));
    }

    /// Looks up the Mon's locked move, if any.
    pub fn locked_move(context: &mut MonContext) -> Result<Option<String>, Error> {
        let locked_move = core_battle_effects::run_event_for_mon_expecting_string(
            context,
            fxlang::BattleEvent::LockMove,
            fxlang::VariableInput::default(),
        );
        if locked_move.is_some() {
            // A Mon with a locked move is trapped.
            context.mon_mut().trapped = true;
            context.mon_mut().cannot_receive_items = true;
        }
        Ok(locked_move)
    }

    fn moves_and_locked_move(
        context: &mut MonContext,
    ) -> Result<(Vec<MonMoveSlotData>, Option<String>), Error> {
        let locked_move = Self::locked_move(context)?;
        let moves = Self::moves_with_locked_move(context, locked_move.as_deref())?;
        let has_usable_move = moves.iter().any(|mov| !mov.disabled);
        let moves = if has_usable_move { moves } else { Vec::new() };
        Ok((moves, locked_move))
    }

    /// Looks up all move slot data.
    ///
    /// If a Mon has a locked move, only that move will appear. Thus, this data is effectively
    /// viewed as the Mon's available move options.
    pub fn moves(context: &mut MonContext) -> Result<Vec<MonMoveSlotData>, Error> {
        Self::moves_and_locked_move(context).map(|(moves, _)| moves)
    }

    fn position_on_side_by_active_position(context: &MonContext, active_position: usize) -> usize {
        let player = context.player();
        let position = active_position
            + player.position * context.battle().format.battle_type.active_per_player();
        position
    }

    /// The Mon's current position on its side.
    ///
    /// A side is shared by multiple players, who each can have multiple Mons active at a time. This
    /// position is intended to be scoped to the side, so all active Mons on a side have a unique
    /// position.
    ///
    /// Side position is a zero-based integer index in the range `[0, active_mons)`, where
    /// `active_mons` is the number of active Mons on the side. This is calculated by
    /// `players_on_side * active_per_player`.
    pub fn position_on_side(context: &MonContext) -> Option<usize> {
        Some(Self::position_on_side_by_active_position(
            context,
            context.mon().active_position?,
        ))
    }

    /// Same as [`Self::position_on_side`], but also returns the previous position of the Mon.
    pub fn position_on_side_or_previous(context: &MonContext) -> Option<usize> {
        Some(Self::position_on_side_by_active_position(
            context,
            context
                .mon()
                .active_position
                .or(context.mon().old_active_position)?,
        ))
    }

    fn relative_location(
        mon_position: usize,
        target_position: usize,
        same_side: bool,
        mons_per_side: usize,
    ) -> isize {
        if same_side {
            let diff = (target_position).abs_diff(mon_position);
            let diff = diff as isize;
            -diff
        } else {
            let flipped_position = mons_per_side - target_position - 1;
            let diff = (flipped_position).abs_diff(mon_position);
            let diff = diff + 1;
            diff as isize
        }
    }

    /// Calculates the relative location of the given target position.
    ///
    /// Relative location is essentially the Manhatten distance between this Mon and the target Mon.
    /// It is negative if the Mon is on the same side and positive if the Mon is on the opposite
    /// side.
    pub fn relative_location_of_target(
        context: &MonContext,
        target_side: usize,
        target_position: usize,
    ) -> Result<isize, Error> {
        let mon = context.mon();
        let mon_side = mon.side;
        let mon_position = Self::position_on_side(context)
            .wrap_error_with_message("expected mon to have a position")?;

        // Note that this calculation assumes that both sides have the same amount of players,
        // but battles are not required to validate this. Nonetheless, this calculation is still
        // correct, since players are positioned from left to right.
        //
        // The "Players Per Side" clause can be used to validate that both sides have the same
        // amount of players.
        //
        // There can still be weird behavior in a multi-battle or triple battle where remaining
        // Mons are not adjacent to one another. Shifting logic should be implemented somewhere
        // higher up so that `Self::position_on_side` returns the correct value after shifting.
        let mons_per_side = context.battle().max_side_length();

        if target_position >= mons_per_side {
            return Err(battler_error!(
                "target position {target_position} is out of bounds"
            ));
        }

        Ok(Self::relative_location(
            mon_position,
            target_position,
            mon_side == target_side,
            mons_per_side,
        ))
    }

    /// Gets the target Mon based on this Mon's position.
    pub fn get_target(context: &mut MonContext, target: isize) -> Result<Option<MonHandle>, Error> {
        if target == 0 {
            return Err(battler_error!("target cannot be 0"));
        }
        let mut side_context = context.pick_side_context(target < 0)?;
        let position = (target.abs() - 1) as usize;
        Side::mon_in_position(&mut side_context, position)
    }

    /// Gets the target Mon's position based on this Mon's position.
    pub fn get_target_location(
        context: &mut MonContext,
        target: MonHandle,
    ) -> Result<isize, Error> {
        let target_context = context.as_battle_context_mut().mon_context(target)?;
        let target_side = target_context.mon().side;
        let target_position = Self::position_on_side(&target_context)
            .wrap_error_with_message("expected target to have a position")?
            + 1;
        if target_side == context.mon().side {
            Ok(-(target_position as isize))
        } else {
            Ok(target_position as isize)
        }
    }

    /// Checks if the given Mon is an ally.
    pub fn is_ally(&self, mon: &Mon) -> bool {
        self.side == mon.side
    }

    /// Checks if the given Mon is adjacent to this Mon.
    pub fn is_adjacent(context: &mut MonContext, other: MonHandle) -> Result<bool, Error> {
        let side = context.mon().side;
        let position = match Self::position_on_side(context) {
            Some(position) => position,
            None => return Ok(false),
        };
        let other_context = context.as_battle_context_mut().mon_context(other)?;
        let other_side = other_context.mon().side;
        let mut other_position = match Self::position_on_side(&other_context) {
            Some(position) => position,
            None => return Ok(false),
        };

        if side != other_side {
            let mons_per_side = context.battle().max_side_length();
            if position >= mons_per_side {
                return Err(battler_error!("mon position {position} is out of bounds"));
            }
            // Flip the position.
            other_position = mons_per_side - other_position - 1;
        }

        let reach = context.battle().format.options.adjacency_reach as usize - 1;
        let diff = if position > other_position {
            position - other_position
        } else {
            other_position - position
        };

        Ok(diff <= reach)
    }

    /// Creates an iterator over all active allies and this Mon.
    pub fn active_allies_and_self<'m>(
        context: &'m mut MonContext,
    ) -> impl Iterator<Item = MonHandle> + 'm {
        let side = context.side().index;
        context.battle().active_mon_handles_on_side(side)
    }

    /// Creates an iterator over all adjacent allies.
    pub fn adjacent_allies(
        context: &mut MonContext,
    ) -> Result<impl Iterator<Item = MonHandle>, Error> {
        let allies = context
            .battle()
            .active_mon_handles_on_side(context.mon().side)
            .collect::<Vec<_>>();
        let mut adjacent_allies = Vec::new();
        for ally in allies {
            if context.mon_handle() != ally && Self::is_adjacent(context, ally)? {
                adjacent_allies.push(ally);
            }
        }
        Ok(adjacent_allies.into_iter())
    }

    /// Creates an iterator over all adjacent allies and this Mon.
    pub fn adjacent_allies_and_self(
        context: &mut MonContext,
    ) -> Result<impl Iterator<Item = MonHandle>, Error> {
        Ok(Self::adjacent_allies(context)?.chain(iter::once(context.mon_handle())))
    }

    /// Checks if the given Mon is a foe.
    pub fn is_foe(&self, mon: &Mon) -> bool {
        self.side != mon.side
    }

    /// Creates an iterator over all active foes.
    pub fn active_foes<'m>(context: &'m mut MonContext) -> impl Iterator<Item = MonHandle> + 'm {
        let foe_side = context.foe_side().index;
        context.battle().active_mon_handles_on_side(foe_side)
    }

    /// Creates an iterator over all adjacent foes.
    pub fn adjacent_foes(
        context: &mut MonContext,
    ) -> Result<impl Iterator<Item = MonHandle>, Error> {
        let foes = Self::active_foes(context).collect::<Vec<_>>();
        let mut adjacent_foes = Vec::new();
        for foe in foes {
            if Self::is_adjacent(context, foe)? {
                adjacent_foes.push(foe);
            }
        }
        Ok(adjacent_foes.into_iter())
    }

    fn calculate_stat_internal(
        context: &mut MonContext,
        stat: Stat,
        unboosted: bool,
        boost: Option<i8>,
        unmodified: bool,
        modifier: Option<Fraction<u16>>,
        stat_user: Option<MonHandle>,
        calculate_stat_context: Option<CalculateStatContext>,
    ) -> Result<u16, Error> {
        let stat_user = stat_user.unwrap_or(context.mon_handle());

        if stat == Stat::HP {
            return Err(battler_error!(
                "HP should be read directly, not by calling get_stat"
            ));
        }

        let mut value = context.mon().stats.get(stat);
        if !unboosted {
            let boosts = context.mon().boosts.clone();
            let boosts = core_battle_effects::run_event_for_mon_expecting_boost_table(
                &mut context.as_battle_context_mut().mon_context(stat_user)?,
                fxlang::BattleEvent::ModifyBoosts,
                boosts,
            );
            let boost = match boost {
                Some(boost) => boost,
                None => boosts.get(stat.try_into()?),
            };
            lazy_static! {
                static ref BOOST_TABLE: [Fraction<u16>; 7] = [
                    Fraction::new(1, 1),
                    Fraction::new(3, 2),
                    Fraction::new(2, 1),
                    Fraction::new(5, 2),
                    Fraction::new(3, 1),
                    Fraction::new(7, 2),
                    Fraction::new(4, 1)
                ];
            }
            let boost = boost.max(-6).min(6);
            let boost_fraction = &BOOST_TABLE[boost.abs() as usize];
            if boost >= 0 {
                value = boost_fraction.mul(value).floor();
            } else {
                value = boost_fraction.inverse().mul(value).floor();
            }
        }
        if !unmodified {
            if let Some(modify_event) = stat.modify_event() {
                let mon_handle = context.mon_handle();
                value = match calculate_stat_context {
                    Some(calculate_stat_context) => {
                        core_battle_effects::run_event_for_applying_effect_expecting_u16(
                            &mut context.as_battle_context_mut().applying_effect_context(
                                calculate_stat_context.effect,
                                Some(calculate_stat_context.source),
                                mon_handle,
                                None,
                            )?,
                            modify_event,
                            value,
                        )
                    }
                    None => core_battle_effects::run_event_for_mon_expecting_u16(
                        context,
                        modify_event,
                        value,
                    ),
                }
            }
            let modifier = modifier.unwrap_or(Fraction::from(1u16));
            value = modify_32(value as u32, modifier.convert()) as u16;
        }

        Ok(value)
    }

    /// Calculates the current value for the given [`Stat`] on a [`Mon`].
    ///
    /// Similar to [`Self::get_stat`], but can take in custom boosts and modifiers.
    pub fn calculate_stat(
        context: &mut MonContext,
        stat: Stat,
        boost: i8,
        modifier: Fraction<u16>,
        stat_user: Option<MonHandle>,
        calculate_stat_context: Option<CalculateStatContext>,
    ) -> Result<u16, Error> {
        Self::calculate_stat_internal(
            context,
            stat,
            false,
            Some(boost),
            false,
            Some(modifier),
            stat_user,
            calculate_stat_context,
        )
    }

    /// Gets the current value for the given [`Stat`] on a [`Mon`] after all boosts/drops and
    /// modifications.
    pub fn get_stat(
        context: &mut MonContext,
        stat: Stat,
        unboosted: bool,
        unmodified: bool,
    ) -> Result<u16, Error> {
        Self::calculate_stat_internal(context, stat, unboosted, None, unmodified, None, None, None)
    }

    /// Calculates the speed value to use for battle action ordering.
    pub fn action_speed(context: &mut MonContext) -> Result<u16, Error> {
        let speed = Self::get_stat(context, Stat::Spe, false, false)?;
        // TODO: If Trick Room, return u16::MAX - speed. CalculateSpeed event?
        Ok(speed)
    }

    /// Updates the speed of the Mon, called at the end of each turn.
    pub fn update_speed(context: &mut MonContext) -> Result<(), Error> {
        context.mon_mut().speed = Self::action_speed(context)?;
        Ok(())
    }

    fn indexed_move_slot(&self, move_id: &Id) -> Option<(usize, &MoveSlot)> {
        self.move_slots
            .iter()
            .enumerate()
            .find(|(_, move_slot)| &move_slot.id == move_id)
    }

    fn move_slot(&self, move_id: &Id) -> Option<&MoveSlot> {
        self.indexed_move_slot(move_id)
            .map(|(_, move_slot)| move_slot)
    }

    /// Looks up the move slot index of the given move ID.
    pub fn move_slot_index(&self, move_id: &Id) -> Option<usize> {
        self.indexed_move_slot(move_id).map(|(i, _)| i)
    }

    fn indexed_move_slot_mut(&mut self, move_id: &Id) -> Option<(usize, &mut MoveSlot)> {
        self.move_slots
            .iter_mut()
            .enumerate()
            .find(|(_, move_slot)| &move_slot.id == move_id)
    }

    fn move_slot_mut(&mut self, move_id: &Id) -> Option<&mut MoveSlot> {
        self.indexed_move_slot_mut(move_id)
            .map(|(_, move_slot)| move_slot)
    }

    /// Calculates the Mon's weight.
    pub fn get_weight(context: &mut MonContext) -> u32 {
        // TODO: ModifyWeight event.
        context.mon().weight
    }

    /// Creates a speed-orderable object for the Mon.
    pub fn speed_orderable(context: &MonContext) -> SpeedOrderableMon {
        SpeedOrderableMon {
            mon_handle: context.mon_handle(),
            speed: context.mon().speed as u32,
        }
    }
}

// Request getters.
impl Mon {
    fn base_request_data(&self) -> MonBaseRequestData {
        MonBaseRequestData {
            name: self.name.clone(),
            level: self.level,
            gender: self.gender.clone(),
            shiny: self.shiny,
            ball: self.ball.clone(),
        }
    }

    /// Generates battle request data.
    pub fn battle_request_data(context: &mut MonContext) -> Result<MonBattleRequestData, Error> {
        let side_position = Self::position_on_side(context);
        let species = context
            .battle()
            .dex
            .species
            .get_by_id(&context.mon().species)?
            .data
            .name
            .clone();
        let ability = context
            .battle()
            .dex
            .abilities
            .get_by_id(&context.mon().ability.id)?
            .data
            .name
            .clone();
        let item = if let Some(item) = context.mon().item.as_ref() {
            Some(
                context
                    .battle()
                    .dex
                    .items
                    .get_by_id(&item.id)?
                    .data
                    .name
                    .clone(),
            )
        } else {
            None
        };
        Ok(MonBattleRequestData {
            base_data: context.mon().base_request_data(),
            species,
            health: context.mon().actual_health(),
            types: context.mon().types.clone(),
            status: context
                .mon()
                .status
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or(String::default()),
            active: context.mon().active,
            player_active_position: context.mon().active_position,
            side_position,
            stats: context.mon().stats.without_hp(),
            moves: context
                .mon()
                .move_slots
                .clone()
                .into_iter()
                .map(|move_slot| MonMoveSlotData::from(context, &move_slot))
                .collect::<Result<Vec<_>, Error>>()?,
            ability,
            item,
        })
    }

    /// Generates summary request data.
    pub fn summary_request_data(context: &mut MonContext) -> Result<MonSummaryRequestData, Error> {
        let mut stats = context.mon().base_stored_stats.clone();
        stats.hp = context.mon().base_max_hp;
        let species = context
            .battle()
            .dex
            .species
            .get_by_id(&context.mon().base_species)?
            .data
            .name
            .clone();
        let ability = context
            .battle()
            .dex
            .abilities
            .get_by_id(&context.mon().base_ability.id)?
            .data
            .name
            .clone();
        Ok(MonSummaryRequestData {
            base_data: context.mon().base_request_data(),
            species,
            stats,
            moves: context
                .mon()
                .base_move_slots
                .clone()
                .into_iter()
                .map(|move_slot| MonMoveSlotData::from(context, &move_slot))
                .collect::<Result<Vec<_>, Error>>()?,
            ability,
        })
    }

    /// Generates request data for a turn.
    pub fn move_request(context: &mut MonContext) -> Result<MonMoveRequest, Error> {
        let (mut moves, mut locked_move) = Self::moves_and_locked_move(context)?;
        if moves.is_empty() {
            // No moves, the Mon must use Struggle.
            locked_move = Some("struggle".to_owned());
            moves = Vec::from_iter([MonMoveSlotData {
                name: "Struggle".to_owned(),
                id: Id::from_known("struggle"),
                target: Some(MoveTarget::RandomNormal),
                pp: 1,
                max_pp: 1,
                disabled: false,
            }]);
        }

        let mut request = MonMoveRequest {
            team_position: context.mon().team_position,
            moves,
            trapped: false,
            can_mega_evo: false,
        };

        let can_switch = Player::can_switch(context.as_player_context_mut());
        if can_switch && context.mon().trapped {
            request.trapped = true;
        }

        if locked_move.is_none() {
            request.can_mega_evo = context.mon().can_mega_evo;
        }

        Ok(request)
    }

    /// Generates request data for learnable moves.
    pub fn learn_move_request(
        context: &mut MonContext,
    ) -> Result<Option<MonLearnMoveRequest>, Error> {
        // Stable sort the moves that can be learned for consistency.
        context.mon_mut().learnable_moves.sort_by(|a, b| a.cmp(&b));

        // Moves must be learned one at a time.
        match context.mon().learnable_moves.first() {
            Some(learnable_move) => {
                let name = context
                    .battle()
                    .dex
                    .moves
                    .get_by_id(learnable_move)
                    .into_result()
                    .wrap_error_with_format(format_args!(
                        "move id {} was not found in the move dex for learning a move",
                        learnable_move,
                    ))?
                    .data
                    .name
                    .clone();
                Ok(Some(MonLearnMoveRequest {
                    team_position: context.mon().team_position,
                    id: learnable_move.clone(),
                    name,
                }))
            }
            None => Ok(None),
        }
    }

    /// The affection level of the Mon, based on its friendship.
    pub fn affection_level(&self) -> u8 {
        match self.friendship {
            0 => 0,
            1..=49 => 1,
            50..=99 => 2,
            100..=149 => 3,
            150..=254 => 4,
            255 => 5,
        }
    }
}

impl Mon {
    /// Clears all volatile effects.
    pub fn clear_volatile(context: &mut MonContext, clear_switch_flags: bool) -> Result<(), Error> {
        if clear_switch_flags {
            context.mon_mut().needs_switch = None;
            context.mon_mut().force_switch = None;
        }

        context.mon_mut().last_move = None;
        context.mon_mut().last_move_target_location = None;
        context.mon_mut().last_move_used = None;
        context.mon_mut().foes_fought_while_active.clear();
        context.mon_mut().received_attacks.clear();

        context.mon_mut().clear_boosts();

        context.mon_mut().ability = context.mon_mut().base_ability.clone();

        context.mon_mut().move_slots = context.mon().base_move_slots.clone();
        context.mon_mut().volatiles.clear();

        let species = context.mon().base_species.clone();
        Self::set_species(context, &Id::from(species))?;
        Ok(())
    }

    /// Recalculates a Mon's base stats.
    ///
    /// Should only be used when a Mon levels up.
    pub fn recalculate_base_stats(context: &mut MonContext) -> Result<(), Error> {
        let species = context
            .battle()
            .dex
            .species
            .get_by_id(&context.mon().base_species)?;

        let mut stats = calculate_mon_stats(
            &species.data.base_stats,
            &context.mon().ivs,
            &context.mon().evs,
            context.mon().level,
            context.mon().nature,
        );
        // Forced max HP always overrides stat calculations.
        if let Some(max_hp) = species.data.max_hp {
            stats.hp = max_hp;
        }

        let current_health = if context.mon().max_hp > 0 {
            Fraction::new(context.mon().hp, context.mon().max_hp)
        } else {
            Fraction::from(1u16)
        };
        context.mon_mut().max_hp = stats.hp;
        context.mon_mut().hp = (current_health * context.mon().max_hp).floor();

        context.mon_mut().base_max_hp = stats.hp;
        context.mon_mut().base_stored_stats = stats.clone();

        Ok(())
    }

    /// Recalculates a Mon's stats.
    pub fn recalculate_stats(context: &mut MonContext) -> Result<(), Error> {
        let species = context
            .battle()
            .dex
            .species
            .get_by_id(&context.mon().species)?;

        let mut stats = calculate_mon_stats(
            &species.data.base_stats,
            &context.mon().ivs,
            &context.mon().evs,
            context.mon().level,
            context.mon().nature,
        );
        // Forced max HP always overrides stat calculations.
        if let Some(max_hp) = species.data.max_hp {
            stats.hp = max_hp;
        }

        context.mon_mut().stats = context
            .mon()
            .stats
            .entries()
            .map(|(stat, _)| (stat, stats.get(stat)))
            .collect();
        context.mon_mut().speed = context.mon().stats.spe;

        Ok(())
    }

    /// Sets the base species of the Mon.
    pub fn set_base_species(
        context: &mut MonContext,
        base_species: &Id,
        change_ability: bool,
    ) -> Result<(), Error> {
        let species = context.battle().dex.species.get_by_id(base_species)?;

        // Base ability can change.
        //
        // We don't set this at the start of the battle since we trust the result of team validation
        // (or the lack thereof). Forme changes that happen within the battle, though, can force an
        // ability change (think about Mega Evolution).
        let mut base_ability = context.mon().base_ability.id.clone();
        if change_ability
            && !species
                .data
                .abilities
                .iter()
                .any(|ability| Id::from(ability.as_ref()) == base_ability)
        {
            base_ability = species
                .data
                .abilities
                .first()
                .map(|ability| Id::from(ability.as_ref()))
                .unwrap_or(Id::from_known("noability"));
        }

        context.mon_mut().base_species = Id::from(species.data.base_species.clone());

        let mon_handle = context.mon_handle();
        let ability = context.battle().dex.abilities.get_by_id(&base_ability)?;
        context.mon_mut().base_ability = AbilitySlot {
            id: ability.id().clone(),
            order: 0,
            effect_state: fxlang::EffectState::initial_effect_state(
                &mut context
                    .as_battle_context_mut()
                    .effect_context(EffectHandle::Condition(Id::from_known("start")), None)?,
                Some(mon_handle),
                None,
            )?,
        };

        Ok(())
    }

    /// Sets the species of the Mon.
    pub fn set_species(context: &mut MonContext, species: &Id) -> Result<bool, Error> {
        // TODO: ModifySpecies event.
        let species = context.battle().dex.species.get_by_id(species)?;

        // SAFETY: Nothing we do below will invalidate any data.
        let species: ElementRef<Species> = unsafe { mem::transmute(species) };

        context.mon_mut().species = species.id().clone();
        context.mon_mut().types = Vec::with_capacity(4);
        context.mon_mut().types.push(species.data.primary_type);
        if let Some(secondary_type) = species.data.secondary_type {
            context.mon_mut().types.push(secondary_type);
        }

        Self::recalculate_stats(context)?;
        context.mon_mut().weight = species.data.weight;

        Ok(true)
    }

    /// Overwrites the move slot at the given index.
    pub fn overwrite_move_slot(
        &mut self,
        index: usize,
        new_move_slot: MoveSlot,
        override_base_slot: bool,
    ) -> Result<(), Error> {
        if override_base_slot {
            *self
                .base_move_slots
                .get_mut(index)
                .wrap_error_with_format(format_args!("no move slot in index {index}"))? =
                new_move_slot.clone();
        }
        *self
            .move_slots
            .get_mut(index)
            .wrap_error_with_format(format_args!("no move slot in index {index}"))? = new_move_slot;
        Ok(())
    }

    /// Clears all stat boosts.
    pub fn clear_boosts(&mut self) {
        self.boosts = BoostTable::new();
    }

    fn moves_with_locked_move(
        context: &mut MonContext,
        locked_move: Option<&str>,
    ) -> Result<Vec<MonMoveSlotData>, Error> {
        // First, check if the Mon is locked into a certain move.
        if let Some(locked_move) = locked_move {
            let locked_move_id = Id::from(locked_move.as_ref());
            // Recharge is a special move for moves that require a turn to recharge.
            if locked_move_id.eq("recharge") {
                return Ok(Vec::from_iter([MonMoveSlotData {
                    name: "Recharge".to_owned(),
                    id: Id::from_known("recharge"),
                    pp: 0,
                    max_pp: 0,
                    target: Some(MoveTarget::User),
                    disabled: false,
                }]));
            }

            // Look for the locked move in the Mon's moveset.
            if let Some(locked_move) = context
                .mon()
                .move_slots
                .iter()
                .find(|move_slot| move_slot.id == locked_move_id)
            {
                return Ok(Vec::from_iter([MonMoveSlotData {
                    name: locked_move.name.clone(),
                    id: locked_move.id.clone(),
                    pp: 0,
                    max_pp: 0,
                    target: None,
                    disabled: false,
                }]));
            }
            return Err(battler_error!(
                "Mon's locked move {locked_move} does not exist in its moveset"
            ));
        }

        // Else, generate move details for each move.
        let move_slots = context.mon().move_slots.clone();
        move_slots
            .into_iter()
            .map(|move_slot| MonMoveSlotData::from(context, &move_slot))
            .collect()
    }

    /// Switches the Mon into the given position for the player.
    pub fn switch_in(context: &mut MonContext, position: usize) -> Result<(), Error> {
        context.mon_mut().active = true;
        context.mon_mut().active_turns = 0;
        context.mon_mut().active_move_actions = 0;
        context.mon_mut().active_position = Some(position);
        context.mon_mut().newly_switched = true;

        let mon_handle = context.mon_handle();
        context
            .player_mut()
            .set_active_position(position, Some(mon_handle))?;

        for move_slot in &mut context.mon_mut().move_slots {
            move_slot.used = false;
        }
        let ability_order = context.battle_mut().next_ability_order();
        context.mon_mut().ability.order = ability_order;
        Ok(())
    }

    /// Switches the Mon out of its active position.
    pub fn switch_out(context: &mut MonContext) -> Result<(), Error> {
        context.mon_mut().active = false;
        context.mon_mut().being_called_back = false;
        context.mon_mut().needs_switch = None;
        context.mon_mut().old_active_position = context.mon().active_position;
        if let Some(old_active_position) = context.mon().old_active_position {
            context
                .player_mut()
                .set_active_position(old_active_position, None)?;
        }
        context.mon_mut().active_position = None;
        Ok(())
    }

    /// Sets the active move.
    pub fn set_active_move(&mut self, active_move: MoveHandle) {
        self.active_move = Some(active_move);
    }

    /// Clears the active move.
    pub fn clear_active_move(&mut self) {
        self.active_move = None;
    }

    /// Checks the PP for the given move.
    ///
    /// Returns if the move can be used with the PP deduction.
    pub fn check_pp(&self, move_id: &Id, amount: u8) -> bool {
        if let Some(move_slot) = self.move_slot(move_id) {
            if amount > move_slot.pp {
                return false;
            } else {
                return true;
            }
        }
        return false;
    }

    /// Deducts PP from the given move.
    pub fn deduct_pp(&mut self, move_id: &Id, amount: u8) -> u8 {
        let mut move_slot_index = None;
        let mut pp = 0;
        let mut delta = 0;
        if let Some((i, move_slot)) = self.indexed_move_slot_mut(move_id) {
            let before = move_slot.pp;
            move_slot.used = true;
            if amount > move_slot.pp {
                move_slot.pp = 0;
            } else {
                move_slot.pp -= amount;
            }
            if !move_slot.simulated {
                move_slot_index = Some(i);
                pp = move_slot.pp;
            }
            delta = before - move_slot.pp;
        }

        if let Some(index) = move_slot_index {
            if let Some(base_move_slot) = self.base_move_slots.get_mut(index) {
                base_move_slot.pp = pp;
            }
        }

        delta
    }

    /// Restores PP for the given move.
    pub fn restore_pp(&mut self, move_id: &Id, amount: u8) -> u8 {
        let mut move_slot_index = None;
        let mut pp = 0;
        let mut delta = 0;
        if let Some((i, move_slot)) = self.indexed_move_slot_mut(move_id) {
            let before = move_slot.pp;
            let max_diff = move_slot.max_pp - move_slot.pp;
            if amount > max_diff {
                move_slot.pp = move_slot.max_pp;
            } else {
                move_slot.pp += amount;
            }
            if !move_slot.simulated {
                move_slot_index = Some(i);
                pp = move_slot.pp;
            }
            delta = move_slot.pp - before;
        }

        if let Some(index) = move_slot_index {
            if let Some(base_move_slot) = self.base_move_slots.get_mut(index) {
                base_move_slot.pp = pp;
            }
        }

        delta
    }

    /// Sets the PP for a given move.
    pub fn set_pp(&mut self, move_id: &Id, amount: u8) -> u8 {
        let mut move_slot_index = None;
        let mut pp = 0;
        if let Some((i, move_slot)) = self.indexed_move_slot_mut(move_id) {
            move_slot.pp = amount;
            if move_slot.pp > move_slot.max_pp {
                move_slot.pp = move_slot.max_pp;
            }

            if !move_slot.simulated {
                move_slot_index = Some(i);
                pp = move_slot.pp;
            }
        }

        if let Some(index) = move_slot_index {
            if let Some(base_move_slot) = self.base_move_slots.get_mut(index) {
                base_move_slot.pp = pp;
            }
        }

        pp
    }

    /// Increases friendship based on the current friendship level.
    pub fn increase_friendship(context: &mut MonContext, delta: [u8; 3]) {
        let delta = delta[(context.mon().friendship / 100) as usize];
        let delta = core_battle_effects::run_event_for_mon_expecting_u8(
            context,
            fxlang::BattleEvent::ModifyFriendshipIncrease,
            delta,
        );
        let max_delta = u8::MAX - context.mon().friendship;
        context.mon_mut().friendship += delta.min(max_delta);
    }

    /// Increases friendship based on the current friendship level.
    pub fn decrease_friendship(context: &mut MonContext, delta: [u8; 3]) {
        let delta = delta[(context.mon().friendship / 100) as usize];
        let max_delta = context.mon().friendship;
        context.mon_mut().friendship -= delta.min(max_delta);
    }

    /// Checks if the Mon is immune to the given type.
    pub fn is_immune(context: &mut MonContext, typ: Type) -> Result<bool, Error> {
        if context.mon().fainted {
            return Ok(false);
        }

        if !core_battle_effects::run_event_for_mon(
            context,
            fxlang::BattleEvent::NegateImmunity,
            fxlang::VariableInput::from_iter([fxlang::Value::Type(typ)]),
        ) {
            return Ok(false);
        }

        if !core_battle_effects::run_event_for_mon(
            context,
            fxlang::BattleEvent::TypeImmunity,
            fxlang::VariableInput::from_iter([fxlang::Value::Type(typ)]),
        ) {
            return Ok(true);
        }

        let types = Self::types(context)?;
        let immune = context.battle().check_type_immunity(typ, &types);

        Ok(immune)
    }

    /// Applies damage to the Mon.
    pub fn damage(
        context: &mut MonContext,
        damage: u16,
        source: Option<MonHandle>,
        effect: Option<&EffectHandle>,
    ) -> Result<u16, Error> {
        if context.mon().hp == 0 || damage == 0 {
            return Ok(0);
        }
        let damage = context.mon().hp.min(damage);
        context.mon_mut().hp -= damage;
        if context.mon().hp == 0 {
            Self::faint(context, source, effect)?;
        }
        Ok(damage)
    }

    /// Faints the Mon.
    pub fn faint(
        context: &mut MonContext,
        source: Option<MonHandle>,
        effect: Option<&EffectHandle>,
    ) -> Result<(), Error> {
        if context.mon().fainted {
            return Ok(());
        }
        context.mon_mut().hp = 0;
        context.mon_mut().needs_switch = None;
        let mon_handle = context.mon_handle();
        context.battle_mut().faint_queue.push_back(FaintEntry {
            target: mon_handle,
            source,
            effect: effect.cloned(),
        });
        Ok(())
    }

    /// Heals the Mon.
    pub fn heal(context: &mut MonContext, mut damage: u16) -> Result<u16, Error> {
        if context.mon().hp == 0 || damage == 0 || context.mon().hp > context.mon().max_hp {
            return Ok(0);
        }
        context.mon_mut().hp += damage;
        if context.mon().hp > context.mon().max_hp {
            damage -= context.mon().hp - context.mon().max_hp;
            context.mon_mut().hp = context.mon().max_hp;
        }
        Ok(damage)
    }

    /// Clears the Mon's state when it faints.
    pub fn clear_state_on_faint(context: &mut MonContext) -> Result<(), Error> {
        core_battle_effects::run_mon_ability_event(
            &mut context.applying_effect_context(
                EffectHandle::Condition(Id::from_known("faint")),
                None,
                None,
            )?,
            fxlang::BattleEvent::End,
        );
        Self::clear_volatile(context, false)?;
        context.mon_mut().fainted = true;
        Self::switch_out(context)?;
        Ok(())
    }

    /// Revives the Mon so that it can be used again.
    pub fn revive(context: &mut MonContext, hp: u16) -> Result<u16, Error> {
        context.mon_mut().fainted = false;
        context.mon_mut().status = None;
        context.mon_mut().hp = 1;
        Self::set_hp(context, hp)?;
        Ok(context.mon().hp)
    }

    /// Caps the given boosts based on the Mon's existing boosts.
    pub fn cap_boosts(context: &MonContext, boosts: BoostTable) -> BoostTable {
        BoostTable::from_iter(boosts.non_zero_iter().map(|(boost, value)| {
            let current_value = context.mon().boosts.get(boost);
            (
                boost,
                (current_value + value).max(-6).min(6) - current_value,
            )
        }))
    }

    /// Applies the given stat boost.
    pub fn boost_stat(context: &mut MonContext, boost: Boost, value: i8) -> i8 {
        let current_value = context.mon().boosts.get(boost);
        let new_value = current_value + value;
        let new_value = new_value.max(-6).min(6);
        context.mon_mut().boosts.set(boost, new_value);
        new_value - current_value
    }

    /// Checks if the Mon has an ability.
    pub fn has_ability(context: &mut MonContext, id: &Id) -> bool {
        mon_states::effective_ability(context).is_some_and(|ability| ability == *id)
    }

    /// Checks if the Mon has an item.
    pub fn has_item(context: &mut MonContext, id: &Id) -> bool {
        mon_states::effective_item(context).is_some_and(|item| item == *id)
    }

    /// Checks if the Mon has a volatile effect.
    pub fn has_volatile(context: &mut MonContext, id: &Id) -> bool {
        context.mon().volatiles.contains_key(id)
    }

    /// Resets the Mon's state for the next turn.
    pub fn reset_state_for_next_turn(context: &mut MonContext) -> Result<(), Error> {
        context.mon_mut().active_turns += 1;

        context.mon_mut().old_active_position = None;
        context.mon_mut().move_this_turn_outcome = None;
        context.mon_mut().hurt_this_turn = 0;
        context.mon_mut().stats_raised_this_turn = false;
        context.mon_mut().stats_lowered_this_turn = false;
        context.mon_mut().item_used_this_turn = false;
        context.mon_mut().newly_switched = false;

        for move_slot in &mut context.mon_mut().move_slots {
            move_slot.disabled = false;
        }

        core_battle_effects::run_event_for_mon(
            context,
            fxlang::BattleEvent::DisableMove,
            fxlang::VariableInput::default(),
        );

        context.mon_mut().trapped = false;
        context.mon_mut().cannot_receive_items = false;

        if core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::TrapMon,
            false,
        ) {
            core_battle_actions::trap_mon(context)?;
        }

        if core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::PreventUsedItems,
            false,
        ) {
            context.mon_mut().cannot_receive_items = true;
        }

        Ok(())
    }

    /// Disables the given move.
    pub fn disable_move(context: &mut MonContext, move_id: &Id) -> Result<(), Error> {
        match context.mon_mut().move_slot_mut(move_id) {
            Some(move_slot) => {
                move_slot.disabled = true;
            }
            None => (),
        }
        Ok(())
    }

    fn remove_move_from_learnable_moves(&mut self, move_id: &Id) {
        self.learnable_moves
            .retain(|learn_move| learn_move != move_id);
    }

    /// Checks if the Mon can learn the move.
    pub fn can_learn_move(context: &mut MonContext, move_id: &Id) -> bool {
        context
            .mon()
            .base_move_slots
            .iter()
            .all(|move_slot| &move_slot.id != move_id)
    }

    /// Learns a move, overwriting the given move slot.
    ///
    /// If the move slot is invalid for the base move slots on the Mon, but the battle format allows
    /// for a move in this slot, the move will be added to the Mon's base move slots. If not, the
    /// move is not learned.
    pub fn learn_move(
        context: &mut MonContext,
        move_id: &Id,
        forget_move_slot: usize,
    ) -> Result<(), Error> {
        let mov = context.battle().dex.moves.get_by_id(move_id)?;
        // SAFETY: The move is borrowed with reference counting, so no mutable reference can be
        // taken without causing an error elsewhere.
        let mov: ElementRef<Move> = unsafe { mem::transmute(mov) };
        let (forget_move_slot, forget_move_slot_index) =
            match context.mon_mut().base_move_slots.get_mut(forget_move_slot) {
                None => {
                    if forget_move_slot
                        >= context.battle().format.rules.numeric_rules.max_move_count as usize
                    {
                        let event = log_event!(
                            "didnotlearnmove",
                            ("mon", Self::position_details(context)?),
                            ("move", mov.data.name.clone())
                        );
                        context.battle_mut().log(event);
                        context.mon_mut().remove_move_from_learnable_moves(move_id);
                        return Ok(());
                    }
                    context.mon_mut().base_move_slots.push(MoveSlot::new(
                        Id::from_known("placeholder"),
                        String::new(),
                        0,
                        0,
                        0,
                        MoveTarget::Normal,
                    ));
                    let index = context.mon().base_move_slots.len() - 1;
                    (
                        // SAFETY: We insert this element directly above.
                        context.mon_mut().base_move_slots.last_mut().unwrap(),
                        index,
                    )
                }
                Some(move_slot) => (move_slot, forget_move_slot),
            };

        let new_move_slot = MoveSlot::new(
            mov.id().clone(),
            mov.data.name.clone(),
            mov.data.pp,
            mov.data.pp,
            0,
            mov.data.target.clone(),
        );

        let old_name = forget_move_slot.name.clone();
        *forget_move_slot = new_move_slot.clone();

        // We also need to overwrite the Mon's move for the battle.
        if !context.mon().transformed {
            match context.mon_mut().move_slots.get_mut(forget_move_slot_index) {
                Some(move_slot) => {
                    *move_slot = new_move_slot;
                }
                None => {
                    context.mon_mut().move_slots.push(new_move_slot);
                }
            }
        }

        let mut event = log_event!(
            "learnedmove",
            ("mon", Self::position_details(context)?),
            ("move", mov.data.name.clone()),
        );
        if !old_name.is_empty() {
            event.set("forgot", old_name);
        }
        context.battle_mut().log(event);
        context.mon_mut().remove_move_from_learnable_moves(move_id);

        Ok(())
    }

    /// Checks if the Mon can escape from battle.
    pub fn can_escape(context: &mut MonContext) -> Result<bool, Error> {
        let can_escape = !context.mon().trapped;
        let can_escape = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::CanEscape,
            can_escape,
        );
        Ok(can_escape)
    }

    /// Sets the HP on the Mon directly, returning the delta.
    pub fn set_hp(context: &mut MonContext, mut hp: u16) -> Result<i32, Error> {
        if context.mon().hp == 0 {
            return Ok(0);
        }
        if hp < 1 {
            hp = 1;
        }
        let mut delta = context.mon().hp as i32 - hp as i32;
        context.mon_mut().hp = hp;
        if context.mon().hp > context.mon().max_hp {
            let hp_delta = context.mon().hp - context.mon().max_hp;
            delta -= hp_delta as i32;
            context.mon_mut().hp = context.mon().max_hp;
        }

        core_battle_logs::set_hp(context, None, None)?;
        Ok(delta)
    }
}

#[cfg(test)]
mod mon_tests {
    use crate::battle::Mon;

    #[test]
    fn relative_location_for_triples() {
        assert_eq!(Mon::relative_location(0, 0, true, 3), 0);
        assert_eq!(Mon::relative_location(0, 1, true, 3), -1);
        assert_eq!(Mon::relative_location(0, 2, true, 3), -2);
        assert_eq!(Mon::relative_location(1, 0, true, 3), -1);
        assert_eq!(Mon::relative_location(1, 1, true, 3), 0);
        assert_eq!(Mon::relative_location(1, 2, true, 3), -1);
        assert_eq!(Mon::relative_location(2, 0, true, 3), -2);
        assert_eq!(Mon::relative_location(2, 1, true, 3), -1);
        assert_eq!(Mon::relative_location(2, 2, true, 3), 0);

        assert_eq!(Mon::relative_location(0, 0, false, 3), 3);
        assert_eq!(Mon::relative_location(0, 1, false, 3), 2);
        assert_eq!(Mon::relative_location(0, 2, false, 3), 1);
        assert_eq!(Mon::relative_location(1, 0, false, 3), 2);
        assert_eq!(Mon::relative_location(1, 1, false, 3), 1);
        assert_eq!(Mon::relative_location(1, 2, false, 3), 2);
        assert_eq!(Mon::relative_location(2, 0, false, 3), 1);
        assert_eq!(Mon::relative_location(2, 1, false, 3), 2);
        assert_eq!(Mon::relative_location(2, 2, false, 3), 3);
    }

    #[test]
    fn relative_location_for_doubles_and_multi() {
        assert_eq!(Mon::relative_location(0, 0, true, 2), 0);
        assert_eq!(Mon::relative_location(0, 1, true, 2), -1);
        assert_eq!(Mon::relative_location(1, 0, true, 2), -1);
        assert_eq!(Mon::relative_location(1, 1, true, 2), 0);

        assert_eq!(Mon::relative_location(0, 0, false, 2), 2);
        assert_eq!(Mon::relative_location(0, 1, false, 2), 1);
        assert_eq!(Mon::relative_location(1, 0, false, 2), 1);
        assert_eq!(Mon::relative_location(1, 1, false, 2), 2);
    }

    #[test]
    fn relative_location_for_singles() {
        assert_eq!(Mon::relative_location(0, 0, true, 1), 0);
        assert_eq!(Mon::relative_location(0, 0, false, 1), 1);
    }
}
