use alloc::{
    borrow::ToOwned,
    format,
    string::{
        String,
        ToString,
    },
    vec::Vec,
};
use core::{
    fmt::{
        self,
        Display,
    },
    iter,
    ops::Mul,
    u8,
};

use anyhow::Result;
use battler_data::{
    Boost,
    BoostTable,
    Fraction,
    Gender,
    Id,
    Identifiable,
    MoveTarget,
    Nature,
    PartialStatTable,
    Stat,
    StatTable,
    SwitchType,
    Type,
};
use hashbrown::{
    HashMap,
    HashSet,
};
use serde::{
    Deserialize,
    Serialize,
};
use zone_alloc::ElementRef;

use crate::{
    battle::{
        CoreBattle,
        MonContext,
        MonHandle,
        MoveHandle,
        MoveOutcome,
        Player,
        Side,
        SpeedOrderable,
        calculate_hidden_power_type,
        calculate_mon_stats,
        core_battle::{
            CatchEntry,
            FaintEntry,
        },
        core_battle_actions,
        core_battle_effects,
        core_battle_logs,
        mon_states,
    },
    battle_log_entry,
    dex::Dex,
    effect::{
        AppliedEffectLocation,
        EffectHandle,
        LinkedEffectsManager,
        fxlang,
    },
    error::{
        WrapOptionError,
        WrapResultError,
        general_error,
    },
    log::{
        BattleLoggable,
        UncommittedBattleLogEntry,
    },
    mons::Species,
    moves::Move,
    teams::MonData,
};

fn default_ball() -> String {
    return "Poké Ball".to_owned();
}

/// The physical details of a [`Mon`].
///
/// Copied by "Illusion."
#[derive(Debug, Clone)]
pub struct PhysicalMonDetails {
    pub name: String,
    pub species: String,
    pub gender: Gender,
    pub shiny: bool,
}

/// Public [`Mon`] details, which are shared to both sides of a battle when the Mon
/// appears or during Team Preview.
#[derive(Debug, Clone)]
pub struct PublicMonDetails {
    pub physical_details: PhysicalMonDetails,
    pub level: u8,
}

impl BattleLoggable for PublicMonDetails {
    fn log(&self, entry: &mut UncommittedBattleLogEntry) {
        entry.set("species", self.physical_details.species.clone());
        entry.set("level", self.level);
        entry.set("gender", &self.physical_details.gender);
        if self.physical_details.shiny {
            entry.add_flag("shiny");
        }
    }
}

/// Public details for an active [`Mon`], which are shared to both sides of a battle when the Mon
/// appears in the battle.
#[derive(Debug, Clone)]
pub struct ActiveMonDetails {
    pub public_details: PublicMonDetails,
    pub player_id: String,
    pub side_position: usize,
    pub health: String,
    pub status: String,
    pub tera: Option<Type>,
}

impl BattleLoggable for ActiveMonDetails {
    fn log(&self, entry: &mut UncommittedBattleLogEntry) {
        entry.set("player", self.player_id.clone());
        entry.set("position", self.side_position);
        entry.set("name", self.public_details.physical_details.name.clone());
        entry.set("health", &self.health);
        if !self.status.is_empty() {
            entry.set("status", &self.status);
        }
        if let Some(tera) = self.tera {
            entry.set("tera", tera);
        }
        self.public_details.log(entry);
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
    pub target: MoveTarget,
    pub disabled: bool,
    pub used: bool,
    pub simulated: bool,
}

impl MoveSlot {
    /// Creates a new move slot.
    pub fn new(id: Id, name: String, pp: u8, max_pp: u8, target: MoveTarget) -> Self {
        Self {
            id,
            name,
            pp,
            max_pp,
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
            target,
            disabled: false,
            used: false,
            simulated: true,
        }
    }
}

/// A single ability slot for a Mon.
#[derive(Debug, Default, Clone)]
pub struct AbilitySlot {
    pub id: Id,
    pub effect_state: fxlang::EffectState,
}

/// Data for a single move on a [`Mon`].
///
/// Makes a copy of underlying data so that it can be stored on move requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonMoveSlotData {
    pub id: Id,
    pub name: String,
    pub pp: u8,
    pub max_pp: u8,
    pub target: MoveTarget,
    pub disabled: bool,
}

impl MonMoveSlotData {
    pub fn from(context: &mut MonContext, move_slot: &MoveSlot) -> Result<Self> {
        let mov = context.battle().dex.moves.get_by_id(&move_slot.id)?;
        let name = mov.data.name.clone();
        let id = mov.id().clone();
        // Some moves may have a special target, depending on the user's type (e.g., Curse).
        let target = core_battle_effects::run_mon_inactive_move_event_expecting_move_target(
            context,
            fxlang::BattleEvent::MoveTargetOverride,
            &move_slot.id,
        )
        .unwrap_or(move_slot.target);
        let mut disabled = move_slot.disabled;
        if move_slot.pp == 0 {
            disabled = true;
        }
        Ok(Self {
            name,
            id,
            pp: move_slot.pp,
            max_pp: move_slot.max_pp,
            target,
            disabled,
        })
    }
}

/// Persistent battle state for a single [`Move`] on a [`Mon`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonPersistentMoveData {
    pub name: String,
    pub pp: u8,
}

/// Data about a single [`Mon`]'s summary, which is its out-of-battle state.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonSummaryData {
    pub name: String,
    pub species: String,
    pub level: u8,
    pub gender: Gender,
    pub nature: Nature,
    pub shiny: bool,
    pub ball: Option<String>,
    pub hp: u16,
    pub friendship: u8,
    pub experience: u32,
    pub stats: StatTable,
    pub evs: StatTable,
    pub ivs: StatTable,
    pub moves: Vec<MonPersistentMoveData>,
    pub ability: String,
    pub item: Option<String>,
    pub status: Option<String>,
    pub hidden_power_type: Type,
}

/// Data about a single [`Mon`]'s battle state.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonBattleData {
    pub summary: MonSummaryData,
    pub species: String,
    pub hp: u16,
    pub max_hp: u16,
    pub health: String,
    pub types: Vec<Type>,
    pub active: bool,
    pub player_team_position: usize,
    pub player_effective_team_position: usize,
    pub player_active_position: Option<usize>,
    pub side_position: Option<usize>,
    pub stats: PartialStatTable,
    pub boosts: BoostTable,
    pub moves: Vec<MonMoveSlotData>,
    pub ability: String,
    pub item: Option<String>,
    pub status: Option<String>,
}

/// Request for a single [`Mon`] to move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonMoveRequest {
    pub team_position: usize,
    pub moves: Vec<MonMoveSlotData>,
    #[serde(default)]
    pub max_moves: Vec<MonMoveSlotData>,
    #[serde(default)]
    pub trapped: bool,
    #[serde(default)]
    pub can_mega_evolve: bool,
    #[serde(default)]
    pub can_dynamax: bool,
    #[serde(default)]
    pub can_terastallize: bool,
    #[serde(default)]
    pub locked_into_move: bool,
}

/// Request for a single [`Mon`] to learn a move.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// How a Mon exited the battle.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MonExitType {
    Fainted,
    Caught,
}

/// Policy for a Mon's HP should be updated when recalculating base stats.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum RecalculateBaseStatsHpPolicy {
    #[default]
    DoNotUpdate,
    KeepHealthRatio,
    KeepHealthRatioCeiling,
    KeepHealthRatioSilently,
    KeepDamageTaken,
}

impl RecalculateBaseStatsHpPolicy {
    fn keep_health_ratio(&self) -> bool {
        match self {
            Self::KeepHealthRatio
            | Self::KeepHealthRatioCeiling
            | Self::KeepHealthRatioSilently => true,
            _ => false,
        }
    }

    fn use_ceil(&self) -> bool {
        match self {
            Self::KeepHealthRatioCeiling => true,
            _ => false,
        }
    }

    fn silent(&self) -> bool {
        match self {
            Self::KeepHealthRatioSilently => true,
            _ => false,
        }
    }
}

/// The type of forme change the Mon has undergone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonSpecialFormeChangeType {
    MegaEvolution,
    PrimalReversion,
    Gigantamax,
}

/// State for the Mon going into the next turn.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct MonNextTurnState {
    pub locked_move: Option<String>,
    pub trapped: bool,
    pub cannot_receive_items: bool,
    pub can_mega_evolve: bool,
    pub can_dynamax: bool,
    pub can_terastallize: bool,
}

/// Switch data for a Mon.
#[derive(Debug, Default, Clone)]
pub struct MonSwitchState {
    /// The Mon needs to switch out at the end of the turn.
    pub needs_switch: Option<SwitchType>,
    /// The Mon is forced to switch out immediately.
    pub force_switch: Option<SwitchType>,
    /// The `BeforeSwitchOut` event already ran so it should be skipped.
    pub skip_before_switch_out: bool,
}

/// Volatile state for a Mon.
#[derive(Debug, Default, Clone)]
pub struct MonVolatileState {
    /// The current species.
    pub species: Id,
    /// The weight of the Mon.
    pub weight: u32,

    /// Current stats.
    pub stats: StatTable,
    /// Current stat boosts.
    pub boosts: BoostTable,
    /// Current speed.
    pub speed: u32,

    /// Current moves.
    pub move_slots: Vec<MoveSlot>,
    /// Current types.
    pub types: Vec<Type>,
    /// The current ability.
    pub ability: AbilitySlot,
    /// Effect state for the item.
    pub item_state: fxlang::EffectState,
    /// The last item used.
    pub last_item: Option<Id>,
    /// Volatile statuses.
    pub volatiles: HashMap<Id, fxlang::EffectState>,

    /// Is the Mon transformed into another?
    pub transformed: bool,
    /// The Mon's physical appearance.
    pub illusion: Option<PhysicalMonDetails>,

    /// The last move selected.
    pub last_move_selected: Option<Id>,
    /// The last move used for the Mon.
    pub last_move: Option<MoveHandle>,
    /// The last move used by the Mon, which can be different from `last_move` if that
    /// move executed a different move (like Metronome).
    pub last_move_used: Option<MoveHandle>,

    /// The outcome of a move used this turn.
    pub move_this_turn_outcome: Option<MoveOutcome>,
    /// The outcome of a move used last turn.
    pub move_last_turn_outcome: Option<MoveOutcome>,
    /// The last position targeted by a move.
    pub last_move_target_location: Option<isize>,

    /// Was the Mon damaged this turn?
    pub damaged_this_turn: bool,
    /// Were the Mon's stats raised this turn?
    pub stats_raised_this_turn: bool,
    /// Were the Mon's stats lowered this turn?
    pub stats_lowered_this_turn: bool,
    /// Did the Mon used an item this turn?
    pub item_used_this_turn: bool,

    /// Set of foes that appeared while this Mon was active.
    pub foes_fought_while_active: HashSet<MonHandle>,
    /// Attacks received by the Mon.
    pub received_attacks: Vec<ReceivedAttackEntry>,
}

/// A Mon in a battle, which battles against other Mons.
pub struct Mon {
    pub player: usize,
    pub side: usize,

    pub name: String,

    /// The original base species of the Mon when the battle started.
    pub original_base_species: Id,
    /// The base species of the Mon, which represents the Mon's permanent physical appearance. In
    /// other words, this species is preserved on switch out.
    ///
    /// NOTE: Not the same as [`battler_data::SpeciesData::base_species`].
    pub base_species: Id,

    /// `true` if the Mon is in an active position.
    ///
    /// The Mon may or may not be fainted.
    pub active: bool,
    pub active_turns: u32,
    pub active_move_actions: u32,
    pub active_position: Option<usize>,
    pub old_active_position: Option<usize>,

    /// The position on the player's team when the battle started.
    pub team_position: usize,

    /// The position on the player's team considering switches throughout the battle.
    pub effective_team_position: usize,

    pub base_stored_stats: StatTable,
    pub ivs: StatTable,
    pub evs: StatTable,

    pub level: u8,
    pub experience: u32,
    pub friendship: u8,
    pub nature: Nature,
    pub true_nature: Nature,
    pub gender: Gender,
    pub shiny: bool,
    pub ball: Option<Id>,
    pub hidden_power_type: Type,
    pub different_original_trainer: bool,
    pub dynamax_level: u8,
    pub gigantamax_factor: bool,
    pub tera_type: Type,

    pub base_move_slots: Vec<MoveSlot>,
    pub original_base_ability: Id,
    pub base_ability: Id,
    pub item: Option<Id>,
    pub status: Option<Id>,
    pub status_state: fxlang::EffectState,

    pub initial_hp: Option<u16>,
    pub hp: u16,
    pub base_max_hp: u16,
    pub max_hp: u16,
    pub exited: Option<MonExitType>,
    pub newly_switched: bool,
    pub being_called_back: bool,

    pub next_turn_state: MonNextTurnState,
    pub switch_state: MonSwitchState,
    pub volatile_state: MonVolatileState,

    /// The move the Mon is actively performing.
    pub active_move: Option<MoveHandle>,

    pub learnable_moves: Vec<Id>,

    pub special_forme_change_type: Option<MonSpecialFormeChangeType>,
    pub dynamaxed: bool,
    pub terastallized: Option<Type>,
    pub terastallization_state: fxlang::EffectState,
}

// Construction and initialization logic.
impl Mon {
    /// Creates a new Mon.
    pub fn new(data: MonData, team_position: usize, dex: &Dex) -> Result<Self> {
        let species = Id::from(data.species);
        let ability = Id::from(data.ability);

        let mut base_move_slots = Vec::with_capacity(data.moves.len());
        for (i, move_name) in data.moves.iter().enumerate() {
            let mov = dex.moves.get(move_name)?;
            let max_pp = if mov.data.no_pp_boosts {
                mov.data.pp
            } else {
                let pp_boosts = data.pp_boosts.get(i).cloned().unwrap_or(0).min(3);
                ((mov.data.pp as u32) * (pp_boosts as u32 + 5) / 5) as u8
            };
            let pp = data
                .persistent_battle_data
                .move_pp
                .get(i)
                .cloned()
                .unwrap_or(max_pp);
            let pp = pp.min(max_pp);
            base_move_slots.push(MoveSlot::new(
                mov.id().clone(),
                mov.data.name.clone(),
                pp,
                max_pp,
                mov.data.target.clone(),
            ));
        }

        let hidden_power_type = data
            .hidden_power_type
            .unwrap_or(calculate_hidden_power_type(&data.ivs));

        let ball = data
            .ball
            .map(|ball| {
                if ball.is_empty() {
                    default_ball()
                } else {
                    ball
                }
            })
            .map(|ball| Id::from(ball));

        Ok(Self {
            player: usize::MAX,
            side: usize::MAX,
            name: data.name,
            original_base_species: species.clone(),
            base_species: species,
            active: false,
            active_turns: 0,
            active_move_actions: 0,
            active_position: None,
            old_active_position: None,
            team_position,
            effective_team_position: team_position,
            base_stored_stats: StatTable::default(),
            ivs: data.ivs,
            evs: data.evs,
            level: data.level,
            experience: data.experience,
            friendship: data.friendship,
            nature: data.nature,
            true_nature: data.true_nature.unwrap_or(data.nature),
            gender: data.gender,
            shiny: data.shiny,
            ball,
            hidden_power_type,
            different_original_trainer: data.different_original_trainer,
            dynamax_level: data.dynamax_level,
            gigantamax_factor: data.gigantamax_factor,
            tera_type: data.tera_type.unwrap_or(Type::None),
            base_move_slots,
            original_base_ability: ability.clone(),
            base_ability: ability,
            item: data.item.map(|item| Id::from(item)),
            status: data
                .persistent_battle_data
                .status
                .map(|status| Id::from(status)),
            status_state: fxlang::EffectState::default(),
            initial_hp: data.persistent_battle_data.hp,
            hp: 0,
            base_max_hp: 0,
            max_hp: 0,
            exited: None,
            newly_switched: false,
            being_called_back: false,
            next_turn_state: MonNextTurnState::default(),
            switch_state: MonSwitchState::default(),
            volatile_state: MonVolatileState::default(),
            active_move: None,
            learnable_moves: Vec::default(),
            special_forme_change_type: None,
            dynamaxed: false,
            terastallized: None,
            terastallization_state: fxlang::EffectState::default(),
        })
    }

    /// Initializes a Mon for battle.
    ///
    /// This *must* be called at the very beginning of a battle, as it sets up important fields on
    /// the Mon, such as its stats.
    pub fn initialize(context: &mut MonContext) -> Result<()> {
        let base_species = context.mon().original_base_species.clone();
        let base_ability = context.mon().original_base_ability.clone();
        Self::set_base_species(context, &base_species, &base_ability)?;

        Self::clear_volatile(context, true)?;
        Self::recalculate_base_stats(context, RecalculateBaseStatsHpPolicy::DoNotUpdate)?;

        // Generate level from experience points if needed.
        if context.mon().level == 0 {
            let species = context
                .battle()
                .dex
                .species
                .get_by_id(&context.mon().base_species)?;
            context.mon_mut().level = species
                .data
                .leveling_rate
                .level_from_exp(context.mon().experience);
        } else if context.mon().experience == 0 {
            let species = context
                .battle()
                .dex
                .species
                .get_by_id(&context.mon().base_species)?;
            context.mon_mut().experience =
                species.data.leveling_rate.exp_at_level(context.mon().level);
        }

        // Set the initial HP, which may signal that the Mon is already fainted.
        let mut hp = context.mon().initial_hp.unwrap_or(context.mon().max_hp);
        if hp > context.mon().max_hp {
            hp = context.mon().max_hp;
        }
        context.mon_mut().hp = hp;
        if context.mon().hp == 0 {
            context.mon_mut().exited = Some(MonExitType::Fainted);
        }

        if context.mon().status.is_some() {
            let mon_handle = context.mon_handle();
            context.mon_mut().status_state = fxlang::EffectState::initial_effect_state(
                context.as_battle_context_mut(),
                None,
                Some(mon_handle),
                None,
            )?;
        }

        if context.mon().tera_type == Type::None
            && let Some(first_type) = context.mon().volatile_state.types.first()
        {
            context.mon_mut().tera_type = *first_type;
        }

        Ok(())
    }
}

// Basic getters.
impl Mon {
    fn health(&self, actual_health: bool, public_base: u32) -> (u32, u32) {
        if actual_health {
            return self.actual_health();
        }
        if self.hp == 0 || self.max_hp == 0 {
            return (self.hp as u32, self.max_hp as u32);
        }
        let ratio = Fraction::new(self.hp as u32, self.max_hp as u32);
        // Always round up to avoid returning 0 when the Mon is not fainted.
        let mut percentage = (ratio * public_base).ceil();

        // Round down if the Mon is damaged.
        if percentage == public_base && ratio < Fraction::new(1, 1) {
            percentage = public_base - 1;
        }

        (percentage, public_base)
    }

    fn health_string(&self, actual_health: bool, public_base: u32) -> String {
        let (hp, max_hp) = self.health(actual_health, public_base);
        if hp == 0 || max_hp == 0 {
            return "0".to_owned();
        }
        format!("{hp}/{max_hp}")
    }

    fn actual_health(&self) -> (u32, u32) {
        (self.hp as u32, self.max_hp as u32)
    }

    fn actual_health_string(&self) -> String {
        if self.hp == 0 || self.max_hp == 0 {
            return "0".to_owned();
        }
        format!("{}/{}", self.hp, self.max_hp)
    }

    /// The physical details for the Mon.
    pub fn physical_details(context: &MonContext) -> Result<PhysicalMonDetails> {
        if let Some(illusion) = context.mon().volatile_state.illusion.clone() {
            return Ok(illusion);
        }

        let species = context
            .battle()
            .dex
            .species
            .get_by_id(&context.mon().volatile_state.species)?
            .data
            .name
            .clone();
        Ok(PhysicalMonDetails {
            name: context.mon().name.clone(),
            species,
            gender: context.mon().gender.clone(),
            shiny: context.mon().shiny,
        })
    }

    /// The public details for the Mon.
    pub fn public_details(context: &MonContext) -> Result<PublicMonDetails> {
        Ok(PublicMonDetails {
            physical_details: Self::physical_details(context)?,
            level: context.mon().level,
        })
    }

    fn active_details(context: &mut MonContext, secret: bool) -> Result<ActiveMonDetails> {
        let status = match context.mon().status.clone() {
            Some(status) => CoreBattle::get_effect_by_id(context.as_battle_context_mut(), &status)?
                .name()
                .to_owned(),
            None => String::new(),
        };
        Ok(ActiveMonDetails {
            public_details: Self::public_details(context)?,
            player_id: context.player().id.clone(),
            side_position: Self::position_on_side(context)
                .wrap_expectation("expected mon to be active")?
                + 1,
            health: if secret {
                context.mon().actual_health_string()
            } else {
                Self::public_health_string(context)
            },
            status,
            tera: context.mon().terastallized,
        })
    }

    /// The public details for the active Mon.
    pub fn public_active_details(context: &mut MonContext) -> Result<ActiveMonDetails> {
        Self::active_details(context, false)
    }

    /// The private details for the active Mon.
    pub fn private_active_details(context: &mut MonContext) -> Result<ActiveMonDetails> {
        Self::active_details(context, true)
    }

    fn public_name(&self) -> &str {
        match &self.volatile_state.illusion {
            Some(illusion) => &illusion.name,
            None => &self.name,
        }
    }

    /// The public details for the Mon when an action is made.
    pub fn position_details(context: &MonContext) -> Result<MonPositionDetails> {
        Ok(MonPositionDetails {
            name: context.mon().public_name().to_owned(),
            player_id: context.player().id.clone(),
            side_position: Self::position_on_side(context).map(|position| position + 1),
        })
    }

    /// Same as [`Self::position_details`], but it also considers the previous active position of
    /// the Mon.
    pub fn position_details_or_previous(context: &MonContext) -> Result<MonPositionDetails> {
        Ok(MonPositionDetails {
            name: context.mon().public_name().to_owned(),
            player_id: context.player().id.clone(),
            side_position: Self::position_on_side_or_previous(context).map(|position| position + 1),
        })
    }

    /// The public health of the Mon.
    pub fn public_health(context: &MonContext) -> (u32, u32) {
        context.mon().health(
            context.battle().engine_options.reveal_actual_health,
            context.battle().engine_options.public_health_base,
        )
    }

    /// The public health of the Mon, always based as a percentage.
    pub fn public_health_percentage(context: &MonContext) -> (u32, u32) {
        context
            .mon()
            .health(false, context.battle().engine_options.public_health_base)
    }

    /// The public health of the Mon.
    pub fn public_health_string(context: &MonContext) -> String {
        context.mon().health_string(
            context.battle().engine_options.reveal_actual_health,
            context.battle().engine_options.public_health_base,
        )
    }

    /// The secret health of the Mon.
    pub fn secret_health_string(context: &MonContext) -> String {
        context.mon().actual_health_string()
    }

    fn moves_and_locked_move(
        context: &mut MonContext,
    ) -> Result<(Vec<MonMoveSlotData>, Option<String>)> {
        let locked_move = context.mon().next_turn_state.locked_move.clone();
        let moves = Self::moves_with_locked_move(context, locked_move.as_deref())?;
        let has_usable_move = moves.iter().any(|mov| !mov.disabled);
        let moves = if has_usable_move { moves } else { Vec::new() };
        Ok((moves, locked_move))
    }

    /// Looks up all move slot data.
    ///
    /// If a Mon has a locked move, only that move will appear. Thus, this data is effectively
    /// viewed as the Mon's available move options.
    pub fn moves(context: &mut MonContext) -> Result<Vec<MonMoveSlotData>> {
        Self::moves_and_locked_move(context).map(|(moves, _)| moves)
    }

    /// Looks up all Max Move slot data.
    ///
    /// Does not account for locked moves.
    fn max_moves(context: &mut MonContext) -> Result<Vec<MoveSlot>> {
        let mut max_moves = context.mon().volatile_state.move_slots.clone();
        for move_slot in &mut max_moves {
            if let Some(max_move) = core_battle_actions::max_move_by_id(context, &move_slot.id)? {
                let mov = context.battle().dex.moves.get_by_id(&max_move)?;
                move_slot.id = mov.id().clone();
                move_slot.name = mov.data.name.clone();
                move_slot.target = mov.data.target;
            }
        }
        Ok(max_moves)
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
    /// Relative location is essentially the Manhattan distance between this Mon and the target Mon.
    /// It is negative if the Mon is on the same side and positive if the Mon is on the opposite
    /// side.
    pub fn relative_location_of_target(
        context: &MonContext,
        target_side: usize,
        target_position: usize,
    ) -> Result<isize> {
        let mon = context.mon();
        let mon_side = mon.side;
        let mon_position =
            Self::position_on_side(context).wrap_expectation("expected mon to have a position")?;

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
            return Err(general_error(format!(
                "target position {target_position} is out of bounds",
            )));
        }

        Ok(Self::relative_location(
            mon_position,
            target_position,
            mon_side == target_side,
            mons_per_side,
        ))
    }

    /// Gets the target Mon based on this Mon's position.
    pub fn get_target(context: &mut MonContext, target: isize) -> Result<Option<MonHandle>> {
        if target == 0 {
            return Err(general_error("target cannot be 0"));
        }
        let mut side_context = context.pick_side_context(target < 0)?;
        let position = (target.abs() - 1) as usize;
        Side::mon_in_position(&mut side_context, position)
    }

    /// Gets the target Mon's position based on this Mon's position.
    pub fn get_target_location(context: &mut MonContext, target: MonHandle) -> Result<isize> {
        let target_context = context.as_battle_context_mut().mon_context(target)?;
        let target_side = target_context.mon().side;
        let target_position = Self::position_on_side(&target_context)
            .wrap_expectation("expected target to have a position")?
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
    pub fn is_adjacent(context: &mut MonContext, other: MonHandle) -> Result<bool> {
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
                return Err(general_error(format!(
                    "mon position {position} is out of bounds"
                )));
            }
            // Flip the position.
            other_position = mons_per_side - other_position - 1;
        }

        let reach = context.battle().format.rules.numeric_rules.adjacency_reach as usize - 1;
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
    ) -> Result<impl Iterator<Item = MonHandle> + use<>> {
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
    ) -> Result<impl Iterator<Item = MonHandle>> {
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
    pub fn adjacent_foes(context: &mut MonContext) -> Result<impl Iterator<Item = MonHandle>> {
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
        stat_user: Option<MonHandle>,
        calculate_stat_context: Option<CalculateStatContext>,
    ) -> Result<u16> {
        let stat_user = stat_user.unwrap_or(context.mon_handle());

        if stat == Stat::HP {
            return Err(general_error(
                "HP should be read directly, not by calling get_stat",
            ));
        }

        let mut value = context.mon().volatile_state.stats.get(stat);

        if !unmodified {
            value = core_battle_effects::run_event_for_mon_expecting_u16(
                context,
                fxlang::BattleEvent::CalculateStat,
                value,
                fxlang::VariableInput::from_iter([fxlang::Value::Stat(stat)]),
            );
        }

        if !unboosted {
            let mut boosts = context.mon().volatile_state.boosts.clone();

            if let Some(boost) = boost {
                boosts.set(stat.try_into()?, boost);
            }

            let boosts = match &calculate_stat_context {
                Some(calculate_stat_context) => {
                    core_battle_effects::run_event_for_applying_effect_expecting_boost_table(
                        &mut context.as_battle_context_mut().applying_effect_context(
                            calculate_stat_context.effect.clone(),
                            Some(calculate_stat_context.source),
                            stat_user,
                            None,
                        )?,
                        fxlang::BattleEvent::ModifyBoosts,
                        boosts,
                    )
                }
                None => core_battle_effects::run_event_for_mon_expecting_boost_table(
                    &mut context.as_battle_context_mut().mon_context(stat_user)?,
                    fxlang::BattleEvent::ModifyBoosts,
                    boosts,
                ),
            };

            let boost = boosts.get(stat.try_into()?);

            static BOOST_TABLE: [(u16, u16); 7] =
                [(1, 1), (3, 2), (2, 1), (5, 2), (3, 1), (7, 2), (4, 1)];
            let boost = boost.max(-6).min(6);
            let (num, den) = BOOST_TABLE[boost.abs() as usize];
            let boost_fraction = Fraction::new(num, den);
            if boost >= 0 {
                value = boost_fraction.mul(value).floor();
            } else {
                value = boost_fraction.inverse().mul(value).floor();
            }
        }
        if !unmodified {
            if let Some(modify_event) = match stat {
                Stat::HP => None,
                Stat::Atk => Some(fxlang::BattleEvent::ModifyAtk),
                Stat::Def => Some(fxlang::BattleEvent::ModifyDef),
                Stat::SpAtk => Some(fxlang::BattleEvent::ModifySpA),
                Stat::SpDef => Some(fxlang::BattleEvent::ModifySpD),
                Stat::Spe => Some(fxlang::BattleEvent::ModifySpe),
            } {
                value = match calculate_stat_context {
                    Some(calculate_stat_context) => {
                        let mon_handle = context.mon_handle();
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
                        fxlang::VariableInput::default(),
                    ),
                }
            }
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
        stat_user: Option<MonHandle>,
        calculate_stat_context: Option<CalculateStatContext>,
    ) -> Result<u16> {
        Self::calculate_stat_internal(
            context,
            stat,
            false,
            Some(boost),
            false,
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
    ) -> Result<u16> {
        Self::calculate_stat_internal(context, stat, unboosted, None, unmodified, None, None)
    }

    /// Calculates the speed value to use for battle action ordering.
    pub fn action_speed(context: &mut MonContext) -> Result<u16> {
        let speed = Self::get_stat(context, Stat::Spe, false, false)?;
        let speed = core_battle_effects::run_event_for_mon_expecting_u16(
            context,
            fxlang::BattleEvent::ModifyActionSpeed,
            speed,
            fxlang::VariableInput::default(),
        );
        Ok(speed)
    }

    /// Updates the speed of the Mon, called at the end of each turn.
    pub fn update_speed(context: &mut MonContext) -> Result<()> {
        context.mon_mut().volatile_state.speed = Self::action_speed(context)? as u32;
        Ok(())
    }

    fn indexed_move_slot(&self, move_id: &Id) -> Option<(usize, &MoveSlot)> {
        self.volatile_state
            .move_slots
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
        self.volatile_state
            .move_slots
            .iter_mut()
            .enumerate()
            .find(|(_, move_slot)| &move_slot.id == move_id)
    }

    fn indexed_base_move_slot_mut(&mut self, move_id: &Id) -> Option<(usize, &mut MoveSlot)> {
        self.base_move_slots
            .iter_mut()
            .enumerate()
            .find(|(_, move_slot)| &move_slot.id == move_id)
    }

    fn move_slot_mut(&mut self, move_id: &Id) -> Option<&mut MoveSlot> {
        self.indexed_move_slot_mut(move_id)
            .map(|(_, move_slot)| move_slot)
    }

    fn base_move_slot_mut(&mut self, move_id: &Id) -> Option<&mut MoveSlot> {
        self.indexed_base_move_slot_mut(move_id)
            .map(|(_, move_slot)| move_slot)
    }

    /// Calculates the Mon's weight.
    pub fn get_weight(context: &mut MonContext) -> u32 {
        // TODO: ModifyWeight event.
        context.mon().volatile_state.weight
    }

    /// Creates a speed-orderable object for the Mon.
    pub fn speed_orderable(context: &MonContext) -> SpeedOrderableMon {
        SpeedOrderableMon {
            mon_handle: context.mon_handle(),
            speed: context.mon().volatile_state.speed,
        }
    }
}

// Request getters.
impl Mon {
    /// Generates battle request data.
    pub fn battle_request_data(context: &mut MonContext) -> Result<MonBattleData> {
        let side_position = Self::position_on_side(context);
        let species = context
            .battle()
            .dex
            .species
            .get_by_id(&context.mon().volatile_state.species)?
            .data
            .name
            .clone();
        let ability = context
            .battle()
            .dex
            .abilities
            .get_by_id(&context.mon().volatile_state.ability.id)?
            .data
            .name
            .clone();
        let item = if let Some(item) = &context.mon().item {
            Some(
                context
                    .battle()
                    .dex
                    .items
                    .get_by_id(item)?
                    .data
                    .name
                    .clone(),
            )
        } else {
            None
        };
        Ok(MonBattleData {
            summary: Self::summary_request_data(context)?,
            species,
            hp: context.mon().hp,
            max_hp: context.mon().max_hp,
            health: context.mon().actual_health_string(),
            types: context.mon().volatile_state.types.clone(),
            active: context.mon().active,
            player_team_position: context.mon().team_position,
            player_effective_team_position: context.mon().effective_team_position,
            player_active_position: context.mon().active_position,
            side_position,
            stats: context.mon().volatile_state.stats.without_hp(),
            boosts: context.mon().volatile_state.boosts.clone(),
            moves: context
                .mon()
                .volatile_state
                .move_slots
                .clone()
                .into_iter()
                .map(|move_slot| MonMoveSlotData::from(context, &move_slot))
                .collect::<Result<Vec<_>>>()?,
            ability,
            item,
            status: context
                .mon()
                .status
                .as_ref()
                .map(|status| status.to_string()),
        })
    }

    /// Generates summary request data.
    pub fn summary_request_data(context: &mut MonContext) -> Result<MonSummaryData> {
        let species = context
            .battle()
            .dex
            .species
            .get_by_id(&context.mon().original_base_species)?
            .data
            .name
            .clone();
        let ability = context
            .battle()
            .dex
            .abilities
            .get_by_id(&context.mon().original_base_ability)?
            .data
            .name
            .clone();
        let item = match &context.mon().item {
            Some(item) => Some(
                context
                    .battle()
                    .dex
                    .items
                    .get_by_id(item)?
                    .data
                    .name
                    .clone(),
            ),
            None => None,
        };
        let ball = match &context.mon().ball {
            Some(ball) => Some(
                context
                    .battle()
                    .dex
                    .items
                    .get_by_id(ball)?
                    .data
                    .name
                    .clone(),
            ),
            None => None,
        };
        Ok(MonSummaryData {
            name: context.mon().name.clone(),
            species,
            level: context.mon().level,
            gender: context.mon().gender,
            nature: context.mon().nature,
            shiny: context.mon().shiny,
            ball,
            hp: context.mon().hp,
            friendship: context.mon().friendship,
            experience: context.mon().experience,
            stats: context.mon().base_stored_stats.clone(),
            evs: context.mon().evs.clone(),
            ivs: context.mon().ivs.clone(),
            moves: context
                .mon()
                .base_move_slots
                .clone()
                .into_iter()
                .map(|move_slot| MonPersistentMoveData {
                    name: move_slot.name.clone(),
                    pp: move_slot.pp,
                })
                .collect::<Vec<_>>(),
            ability,
            item,
            status: context
                .mon()
                .status
                .as_ref()
                .map(|status| status.to_string()),
            hidden_power_type: context.mon().hidden_power_type,
        })
    }

    /// Generates request data for a turn.
    pub fn move_request(context: &mut MonContext) -> Result<MonMoveRequest> {
        let (mut moves, mut locked_move) = Self::moves_and_locked_move(context)?;
        if moves.is_empty() {
            // No moves, the Mon must use Struggle.
            locked_move = Some("struggle".to_owned());
            moves = Vec::from_iter([MonMoveSlotData {
                name: "Struggle".to_owned(),
                id: Id::from_known("struggle"),
                target: MoveTarget::RandomNormal,
                pp: 1,
                max_pp: 1,
                disabled: false,
            }]);
        }

        let mut request = MonMoveRequest {
            team_position: context.mon().team_position,
            moves,
            max_moves: Vec::default(),
            trapped: false,
            can_mega_evolve: false,
            can_dynamax: false,
            can_terastallize: false,
            locked_into_move: false,
        };

        let can_switch = Player::can_switch(context.as_player_context_mut());
        if can_switch && context.mon().next_turn_state.trapped {
            request.trapped = true;
        }

        if locked_move.is_none() {
            request.can_mega_evolve = context.mon().next_turn_state.can_mega_evolve;
            request.can_dynamax = context.mon().next_turn_state.can_dynamax;
            request.can_terastallize = context.mon().next_turn_state.can_terastallize;

            // Communicate Max Moves, mostly for the player's convenience.
            //
            // The actual Max Move is decided immediately when the move is used.
            if request.can_dynamax || context.mon().dynamaxed {
                request.max_moves = Self::max_moves(context)?
                    .into_iter()
                    .map(|move_slot| MonMoveSlotData::from(context, &move_slot))
                    .collect::<Result<_>>()?;
            }
        } else {
            request.locked_into_move = true;
        }

        Ok(request)
    }

    /// Generates request data for learnable moves.
    pub fn learn_move_request(context: &mut MonContext) -> Result<Option<MonLearnMoveRequest>> {
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
    fn new_volatile_state(context: &mut MonContext) -> Result<MonVolatileState> {
        let mon_handle = context.mon_handle();
        Ok(MonVolatileState {
            species: context.mon().base_species.clone(),
            weight: 0, // Updated by set_species.
            stats: context.mon().base_stored_stats.clone(),
            boosts: BoostTable::default(),
            speed: 0, // Updated by set_species.
            move_slots: context.mon().base_move_slots.clone(),
            types: Vec::default(), // Updated by set_species.
            ability: AbilitySlot {
                id: context.mon().base_ability.clone(),
                effect_state: fxlang::EffectState::initial_effect_state(
                    context.as_battle_context_mut(),
                    None,
                    Some(mon_handle),
                    None,
                )?,
            },
            item_state: context
                .mon()
                .item
                .clone()
                .map(|_| {
                    Ok::<_, anyhow::Error>(fxlang::EffectState::initial_effect_state(
                        context.as_battle_context_mut(),
                        None,
                        Some(mon_handle),
                        None,
                    )?)
                })
                .transpose()?
                .unwrap_or_default(),
            last_item: None,
            volatiles: HashMap::default(),
            transformed: false,
            illusion: None,
            last_move_selected: None,
            last_move: None,
            last_move_used: None,
            move_this_turn_outcome: None,
            move_last_turn_outcome: None,
            last_move_target_location: None,
            damaged_this_turn: false,
            stats_raised_this_turn: false,
            stats_lowered_this_turn: false,
            item_used_this_turn: false,
            foes_fought_while_active: HashSet::default(),
            received_attacks: Vec::default(),
        })
    }
    /// Clears all volatile effects.
    pub fn clear_volatile(context: &mut MonContext, clear_switch_flags: bool) -> Result<()> {
        if clear_switch_flags {
            context.mon_mut().switch_state = MonSwitchState::default();
        }

        {
            let mon_handle = context.mon_handle();
            let volatiles = context
                .mon()
                .volatile_state
                .volatiles
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            let mut context = context
                .as_battle_context_mut()
                .effect_context(EffectHandle::Condition(Id::from_known("switchout")), None)?;
            for volatile in volatiles {
                LinkedEffectsManager::remove_by_id(
                    &mut context,
                    &volatile,
                    AppliedEffectLocation::MonVolatile(mon_handle),
                )?;
            }
        }

        context.mon_mut().volatile_state = Self::new_volatile_state(context)?;

        let species = context.mon().base_species.clone();
        Self::set_species(context, &species)?;
        Ok(())
    }

    /// Recalculates a Mon's base stats.
    ///
    /// Should only be used when a Mon levels up.
    pub fn recalculate_base_stats(
        context: &mut MonContext,
        hp_policy: RecalculateBaseStatsHpPolicy,
    ) -> Result<()> {
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

        let new_base_max_hp = stats.hp;
        context.mon_mut().base_max_hp = new_base_max_hp;

        Self::update_max_hp(context, hp_policy)?;

        context.mon_mut().base_stored_stats = stats.clone();

        Ok(())
    }

    /// Updates the Mon's maximum HP.
    pub fn update_max_hp(
        context: &mut MonContext,
        hp_policy: RecalculateBaseStatsHpPolicy,
    ) -> Result<()> {
        let new_max_hp = if context.mon().dynamaxed {
            let ratio =
                Fraction::new(3, 2) + Fraction::new(1, 20) * context.mon().dynamax_level as u16;
            (ratio * context.mon().base_max_hp).floor()
        } else {
            context.mon().base_max_hp
        };

        // Mon is being initialized.
        if context.mon().max_hp == 0 || hp_policy == RecalculateBaseStatsHpPolicy::DoNotUpdate {
            context.mon_mut().max_hp = new_max_hp;
            context.mon_mut().hp = context.mon().hp.min(context.mon().max_hp);
            return Ok(());
        }

        let new_hp = if hp_policy.keep_health_ratio() {
            let mut current_health = if context.mon().max_hp > 0 {
                Fraction::new(context.mon().hp as u32, context.mon().max_hp as u32)
            } else {
                Fraction::from(1u32)
            };
            if current_health > 1 {
                current_health = Fraction::from(1u32);
            }
            let hp = current_health * new_max_hp as u32;
            if hp_policy.use_ceil() {
                hp.ceil() as u16
            } else {
                hp.floor() as u16
            }
        } else {
            let damage_taken = if context.mon().hp > context.mon().max_hp {
                0
            } else {
                context.mon().max_hp - context.mon().hp
            };
            if context.mon().hp == 0 {
                0
            } else if new_max_hp < damage_taken {
                1
            } else {
                new_max_hp - damage_taken
            }
        };

        context.mon_mut().max_hp = new_max_hp;

        let previous_hp = context.mon().hp;
        context.mon_mut().hp = new_hp;

        // Battle log must reflect new HP.
        if previous_hp != context.mon().hp && !hp_policy.silent() {
            core_battle_logs::set_hp(context, None, None)?;
        }

        Ok(())
    }

    /// The current un-Dynamaxed HP of the Mon.
    pub fn undynamaxed_hp(&self) -> u16 {
        if self.dynamaxed {
            (Fraction::new(self.base_max_hp, self.max_hp) * self.hp).ceil()
        } else {
            self.hp
        }
    }

    /// Recalculates a Mon's stats.
    pub fn recalculate_stats(context: &mut MonContext) -> Result<()> {
        let species = context
            .battle()
            .dex
            .species
            .get_by_id(&context.mon().volatile_state.species)?;

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

        context.mon_mut().volatile_state.speed = stats.spe as u32;
        context.mon_mut().volatile_state.stats = stats;

        Ok(())
    }

    /// Sets the base species of the Mon.
    pub fn set_base_species(
        context: &mut MonContext,
        base_species: &Id,
        base_ability: &Id,
    ) -> Result<()> {
        let species = context.battle().dex.species.get_by_id(base_species)?;

        context.mon_mut().base_species = species.id().clone();

        let ability = context.battle().dex.abilities.get_by_id(&base_ability)?;
        context.mon_mut().base_ability = ability.id().clone();

        Ok(())
    }

    /// Sets the species of the Mon.
    pub fn set_species(context: &mut MonContext, species: &Id) -> Result<bool> {
        let species = context.battle().dex.species.get_by_id(species)?;

        // SAFETY: Nothing we do below will invalidate any data.
        let species = unsafe {
            core::mem::transmute::<ElementRef<'_, Species>, ElementRef<'_, Species>>(species)
        };

        let previous_species = context.mon().volatile_state.species.clone();

        context.mon_mut().volatile_state.species = species.id().clone();
        context.mon_mut().volatile_state.types = Vec::with_capacity(4);
        context
            .mon_mut()
            .volatile_state
            .types
            .push(species.data.primary_type);
        if let Some(secondary_type) = species.data.secondary_type {
            context.mon_mut().volatile_state.types.push(secondary_type);
        }

        Self::recalculate_stats(context)?;
        context.mon_mut().volatile_state.weight = species.data.weight;

        Ok(context.mon().volatile_state.species != previous_species)
    }

    /// Looks up the base species of this Mon's base species.
    pub fn base_species_of_species(context: &MonContext) -> Result<Id> {
        Ok(Id::from(
            context
                .battle()
                .dex
                .species
                .get_by_id(&context.mon().base_species)?
                .data
                .base_species
                .as_str(),
        ))
    }

    /// Overwrites the move slot at the given index.
    pub fn overwrite_move_slot(
        &mut self,
        index: usize,
        new_move_slot: MoveSlot,
        override_base_slot: bool,
    ) -> Result<()> {
        if override_base_slot {
            *self
                .base_move_slots
                .get_mut(index)
                .wrap_not_found_error_with_format(format_args!("move slot in index {index}"))? =
                new_move_slot.clone();
        }
        *self
            .volatile_state
            .move_slots
            .get_mut(index)
            .wrap_not_found_error_with_format(format_args!("move slot in index {index}"))? =
            new_move_slot;
        Ok(())
    }

    /// Clears all stat boosts.
    pub fn clear_boosts(&mut self) {
        self.volatile_state.boosts = BoostTable::new();
    }

    /// Decreases the weight of the Mon.
    pub fn decrease_weight(&mut self, delta: u32) {
        self.volatile_state.weight = self.volatile_state.weight.saturating_sub(delta).max(1);
    }

    fn moves_with_locked_move(
        context: &mut MonContext,
        locked_move: Option<&str>,
    ) -> Result<Vec<MonMoveSlotData>> {
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
                    target: MoveTarget::User,
                    disabled: false,
                }]));
            }

            // Look for the locked move in the Mon's moveset.
            if let Some(locked_move) = context
                .mon()
                .volatile_state
                .move_slots
                .iter()
                .find(|move_slot| move_slot.id == locked_move_id)
            {
                return Ok(Vec::from_iter([MonMoveSlotData {
                    name: locked_move.name.clone(),
                    id: locked_move.id.clone(),
                    pp: 0,
                    max_pp: 0,
                    target: MoveTarget::Scripted,
                    disabled: false,
                }]));
            }
            return Err(general_error(format!(
                "Mon's locked move {locked_move} does not exist in its moveset",
            )));
        }

        // Else, generate move details for each move.
        let move_slots = context.mon().volatile_state.move_slots.clone();
        move_slots
            .into_iter()
            .map(|move_slot| MonMoveSlotData::from(context, &move_slot))
            .collect()
    }

    /// Switches the Mon into the given position for the player.
    pub fn switch_in(context: &mut MonContext, position: usize) -> Result<()> {
        context.mon_mut().active = true;
        context.mon_mut().active_turns = 0;
        context.mon_mut().active_move_actions = 0;
        context.mon_mut().active_position = Some(position);
        context.mon_mut().newly_switched = true;
        context.mon_mut().effective_team_position = position;

        let mon_handle = context.mon_handle();
        context
            .player_mut()
            .set_active_position(position, Some(mon_handle))?;

        for move_slot in &mut context.mon_mut().volatile_state.move_slots {
            move_slot.used = false;
        }

        let ability_order = context.battle_mut().next_effect_order();
        context
            .mon_mut()
            .volatile_state
            .ability
            .effect_state
            .set_effect_order(ability_order);

        if context.mon().item.is_some() {
            let item_order = context.battle_mut().next_effect_order();
            context
                .mon_mut()
                .volatile_state
                .item_state
                .set_effect_order(item_order);
        }
        Ok(())
    }

    /// Switches the Mon out of its active position.
    pub fn switch_out(context: &mut MonContext) -> Result<()> {
        context.mon_mut().active = false;
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

    fn deduct_pp_from_move_slot(move_slot: &mut MoveSlot, amount: u8) -> u8 {
        let before = move_slot.pp;
        move_slot.used = true;
        if amount > move_slot.pp {
            move_slot.pp = 0;
        } else {
            move_slot.pp -= amount;
        }
        before - move_slot.pp
    }

    /// Deducts PP from the given move.
    pub fn deduct_pp(&mut self, move_id: &Id, amount: u8) -> u8 {
        let mut move_slot_index = None;
        let mut pp = 0;
        let mut delta = 0;
        if let Some((i, move_slot)) = self.indexed_move_slot_mut(move_id) {
            delta = Self::deduct_pp_from_move_slot(move_slot, amount);
            if !move_slot.simulated {
                move_slot_index = Some(i);
                pp = move_slot.pp;
            }
        }

        if let Some(index) = move_slot_index {
            if let Some(base_move_slot) = self.base_move_slots.get_mut(index) {
                base_move_slot.pp = pp;
            }
        } else if let Some(move_slot) = self.base_move_slot_mut(move_id) {
            // Required for cases where the Mon's moveset changes after it uses the move, such as
            // Transform.
            delta = Self::deduct_pp_from_move_slot(move_slot, amount);
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

    /// Sets friendship directly.
    pub fn set_friendship(context: &mut MonContext, value: u8) {
        context.mon_mut().friendship = value;
    }

    /// Checks if the Mon is immune to the given type.
    pub fn is_immune(context: &mut MonContext, typ: Type) -> Result<bool> {
        if !context.mon().active {
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

        let types = mon_states::effective_types(context);
        let immune = context.battle().check_type_immunity(typ, &types);

        Ok(immune)
    }

    /// Applies damage to the Mon.
    pub fn damage(
        context: &mut MonContext,
        damage: u16,
        source: Option<MonHandle>,
        effect: Option<&EffectHandle>,
    ) -> Result<u16> {
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

    /// Faints the Mon, placing it in the queue to be processed.
    pub fn faint(
        context: &mut MonContext,
        source: Option<MonHandle>,
        effect: Option<&EffectHandle>,
    ) -> Result<()> {
        if !context.mon().active {
            return Ok(());
        }
        context.mon_mut().hp = 0;
        context.mon_mut().switch_state.needs_switch = None;
        let mon_handle = context.mon_handle();
        context.battle_mut().faint_queue.push_back(FaintEntry {
            target: mon_handle,
            source,
            effect: effect.cloned(),
        });
        Ok(())
    }

    /// Catches the Mon, placing it in the queue to be processed.
    pub fn catch(
        context: &mut MonContext,
        player: usize,
        item: Id,
        shakes: u8,
        critical: bool,
    ) -> Result<()> {
        if !context.mon().active {
            return Ok(());
        }
        context.mon_mut().switch_state.needs_switch = None;
        let mon_handle = context.mon_handle();
        context.battle_mut().catch_queue.push_back(CatchEntry {
            target: mon_handle,
            player,
            item,
            shakes,
            critical,
        });
        Ok(())
    }

    /// Heals the Mon.
    pub fn heal(&mut self, mut damage: u16) -> Result<u16> {
        if self.hp == 0 || damage == 0 || self.hp > self.max_hp {
            return Ok(0);
        }
        self.hp += damage;
        if self.hp > self.max_hp {
            damage -= self.hp - self.max_hp;
            self.hp = self.max_hp;
        }
        Ok(damage)
    }

    /// Clears the Mon's state when it exits the battle.
    pub fn clear_state_on_exit(context: &mut MonContext, exit_type: MonExitType) -> Result<()> {
        let effect = match exit_type {
            MonExitType::Fainted => EffectHandle::Condition(Id::from_known("faint")),
            MonExitType::Caught => EffectHandle::Condition(Id::from_known("catch")),
        };
        let mut context = context.applying_effect_context(effect, None, None)?;
        core_battle_actions::end_ability_even_if_exiting(&mut context, true)?;

        core_battle_actions::revert_on_exit(&mut context)?;

        Self::clear_volatile(&mut context.target_context()?, false)?;

        context.target_mut().exited = Some(exit_type);
        match exit_type {
            MonExitType::Fainted => {
                context.target_mut().status = Some(Id::from_known("fnt"));
            }
            MonExitType::Caught => (),
        }

        let mut context = context.target_context()?;
        Self::switch_out(&mut context)?;
        Self::clear_volatile(&mut context, true)?;

        Ok(())
    }

    /// Revives the Mon so that it can be used again.
    pub fn revive(context: &mut MonContext, hp: u16) -> Result<u16> {
        if context.mon().exited != Some(MonExitType::Fainted) {
            return Ok(0);
        }

        context.mon_mut().exited = None;
        context.mon_mut().status = None;
        context.mon_mut().hp = 1;
        Self::set_hp(context, hp)?;
        Self::clear_volatile(context, true)?;
        Ok(context.mon().hp)
    }

    /// Caps the given boosts based on the Mon's existing boosts.
    pub fn cap_boosts(context: &MonContext, boosts: BoostTable) -> BoostTable {
        BoostTable::from_iter(boosts.non_zero_iter().map(|(boost, value)| {
            let current_value = context.mon().volatile_state.boosts.get(boost);
            (
                boost,
                (current_value + value).max(-6).min(6) - current_value,
            )
        }))
    }

    /// Applies the given stat boost.
    pub fn boost_stat(context: &mut MonContext, boost: Boost, value: i8) -> i8 {
        let current_value = context.mon().volatile_state.boosts.get(boost);
        let new_value = current_value + value;
        let new_value = new_value.max(-6).min(6);
        context
            .mon_mut()
            .volatile_state
            .boosts
            .set(boost, new_value);
        new_value - current_value
    }

    /// Sets the Mon's boosted stats directly.
    pub fn set_boosts(context: &mut MonContext, boosts: &BoostTable) {
        for (boost, val) in boosts.non_zero_iter() {
            context.mon_mut().volatile_state.boosts.set(boost, val);
        }
    }

    /// Counts the positive boosts applied to the Mon.
    pub fn positive_boosts(context: &MonContext) -> u8 {
        let mut boosts = 0;
        for (_, val) in context.mon().volatile_state.boosts.non_zero_iter() {
            if val > 0 {
                boosts += val as u8;
            }
        }
        boosts
    }

    /// Checks if the Mon has an ability.
    pub fn has_ability(context: &mut MonContext, id: &Id) -> bool {
        mon_states::effective_ability(context).is_some_and(|ability| ability == *id)
    }

    /// Checks if the Mon has an item.
    pub fn has_item(context: &mut MonContext, id: &Id) -> bool {
        mon_states::effective_item(context).is_some_and(|item| item == *id)
    }

    /// Checks if the Mon has a type.
    pub fn has_type(context: &mut MonContext, typ: Type) -> bool {
        mon_states::has_type(context, typ)
    }

    /// Checks if the Mon has the given type, before forced types (e.g., Terastallization).
    pub fn has_type_before_forced_types(context: &mut MonContext, typ: Type) -> bool {
        mon_states::has_type_before_forced_types(context, typ)
    }

    /// Checks if the Mon has a volatile effect.
    pub fn has_volatile(context: &mut MonContext, id: &Id) -> bool {
        context.mon().volatile_state.volatiles.contains_key(id)
    }

    /// Resets the Mon's state for the next turn.
    pub fn reset_state_for_next_turn(context: &mut MonContext) -> Result<()> {
        context.mon_mut().active_turns += 1;

        context.mon_mut().newly_switched = false;
        context.mon_mut().old_active_position = None;

        context.mon_mut().volatile_state.move_last_turn_outcome =
            context.mon().volatile_state.move_this_turn_outcome;
        context.mon_mut().volatile_state.move_this_turn_outcome = None;
        context.mon_mut().volatile_state.damaged_this_turn = false;
        context.mon_mut().volatile_state.stats_raised_this_turn = false;
        context.mon_mut().volatile_state.stats_lowered_this_turn = false;
        context.mon_mut().volatile_state.item_used_this_turn = false;

        for move_slot in &mut context.mon_mut().volatile_state.move_slots {
            move_slot.disabled = false;
        }

        core_battle_effects::run_event_for_mon(
            context,
            fxlang::BattleEvent::DisableMove,
            fxlang::VariableInput::default(),
        );

        context.mon_mut().next_turn_state = MonNextTurnState::default();

        if Self::trapped(context)? {
            core_battle_actions::trap_mon(context)?;
        }

        if core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::PreventUsedItems,
            false,
        ) {
            context.mon_mut().next_turn_state.cannot_receive_items = true;
        }

        if core_battle_actions::can_mega_evolve(context)?.is_some() {
            context.mon_mut().next_turn_state.can_mega_evolve = true;
        }

        if core_battle_actions::can_dynamax(context)?.is_some() {
            context.mon_mut().next_turn_state.can_dynamax = true;
        }

        if core_battle_actions::can_terastallize(context)?.is_some() {
            context.mon_mut().next_turn_state.can_terastallize = true;
        }

        if let Some(locked_move) = core_battle_effects::run_event_for_mon_expecting_string(
            context,
            fxlang::BattleEvent::LockMove,
            fxlang::VariableInput::default(),
        ) {
            context.mon_mut().next_turn_state.locked_move = Some(locked_move);

            // A Mon with a locked move is trapped and cannot do anything else.
            context.mon_mut().next_turn_state.trapped = true;
            context.mon_mut().next_turn_state.cannot_receive_items = true;
            context.mon_mut().next_turn_state.can_mega_evolve = false;
            context.mon_mut().next_turn_state.can_dynamax = false;
            context.mon_mut().next_turn_state.can_terastallize = false;
        }

        Ok(())
    }

    /// Disables the given move.
    pub fn disable_move(context: &mut MonContext, move_id: &Id) -> Result<()> {
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
    ) -> Result<()> {
        let mov = context.battle().dex.moves.get_by_id(move_id)?;
        // SAFETY: The move is borrowed with reference counting, so no mutable reference can be
        // taken without causing an error elsewhere.
        let mov =
            unsafe { core::mem::transmute::<ElementRef<'_, Move>, ElementRef<'_, Move>>(mov) };
        let (forget_move_slot, forget_move_slot_index) =
            match context.mon_mut().base_move_slots.get_mut(forget_move_slot) {
                None => {
                    if forget_move_slot
                        >= context.battle().format.rules.numeric_rules.max_move_count as usize
                    {
                        let event = battle_log_entry!(
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
            mov.data.target.clone(),
        );

        let old_name = forget_move_slot.name.clone();
        *forget_move_slot = new_move_slot.clone();

        // We also need to overwrite the Mon's move for the battle.
        if !context.mon().volatile_state.transformed {
            match context
                .mon_mut()
                .volatile_state
                .move_slots
                .get_mut(forget_move_slot_index)
            {
                Some(move_slot) => {
                    *move_slot = new_move_slot;
                }
                None => {
                    context
                        .mon_mut()
                        .volatile_state
                        .move_slots
                        .push(new_move_slot);
                }
            }
        }

        let mut event = battle_log_entry!(
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

    /// Checks if the Mon is trapped.
    pub fn trapped(context: &mut MonContext) -> Result<bool> {
        let trapped = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::TrapMon,
            false,
        );
        Ok(trapped)
    }

    /// Checks if the Mon can escape from battle.
    pub fn can_escape(context: &mut MonContext) -> Result<bool> {
        let can_escape = !Self::trapped(context)?;
        let can_escape = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::CanEscape,
            can_escape,
        );
        Ok(can_escape)
    }

    /// Checks if the Mon can Dynamax, based on its own effects.
    pub fn can_dynamax(context: &mut MonContext) -> Result<bool> {
        let can_dynamax = true;
        let can_dynamax = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::CanDynamax,
            can_dynamax,
        );
        Ok(can_dynamax)
    }

    /// Sets the HP on the Mon directly, returning the delta.
    pub fn set_hp(context: &mut MonContext, mut hp: u16) -> Result<i32> {
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

    /// Forces a Mon to be fully healed.
    ///
    /// WARNING: This method is completely silent, so the heal will not be made known and will run
    /// no events. It is intended to be used by the "Heal Ball" item after catching a Mon.
    pub fn force_fully_heal(context: &mut MonContext) -> Result<()> {
        let max_hp = context.mon().max_hp;
        context.mon_mut().heal(max_hp)?;

        context.mon_mut().status = None;

        for (move_id, max_pp) in context
            .mon()
            .base_move_slots
            .iter()
            .map(|move_slot| (move_slot.id.clone(), move_slot.max_pp))
            .collect::<Vec<_>>()
        {
            context.mon_mut().restore_pp(&move_id, max_pp);
        }

        Ok(())
    }
}

#[cfg(test)]
mod mon_test {
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
