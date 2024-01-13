use std::{
    fmt,
    fmt::Display,
    iter,
    ops::Mul,
};

use lazy_static::lazy_static;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        calculate_hidden_power_type,
        calculate_mon_stats,
        core_battle::FaintEntry,
        modify,
        BoostTable,
        MonContext,
        MonHandle,
        MoveHandle,
        Player,
        Side,
    },
    battler_error,
    common::{
        Error,
        Fraction,
        Id,
        Identifiable,
    },
    dex::Dex,
    effect::EffectHandle,
    log::{
        Event,
        EventLoggable,
    },
    mons::{
        Gender,
        Nature,
        PartialStatTable,
        Stat,
        StatTable,
        Type,
    },
    moves::MoveTarget,
    teams::MonData,
};

/// Public [`Mon`] details, which are shared to both sides of a battle when the Mon
/// appears or during Team Preview.
pub struct PublicMonDetails<'d> {
    pub species_name: &'d str,
    pub level: u8,
    pub gender: Gender,
    pub shiny: bool,
}

impl EventLoggable for PublicMonDetails<'_> {
    fn log<'s>(&'s self, event: &mut Event) {
        event.set("species", self.species_name);
        event.set("level", self.level);
        event.set("gender", &self.gender);
        if self.shiny {
            event.add_flag("shiny");
        }
    }
}

/// Public details for an active [`Mon`], which are shared to both sides of a battle when the Mon
/// appears in the battle.
pub struct ActiveMonDetails<'d> {
    pub public_details: PublicMonDetails<'d>,
    pub name: &'d str,
    pub player_id: &'d str,
    pub side_position: usize,
    pub health: String,
    pub status: String,
}

impl EventLoggable for ActiveMonDetails<'_> {
    fn log<'s>(&'s self, event: &mut Event) {
        event.set("player", self.player_id);
        event.set("position", self.side_position);
        event.set("name", self.name);
        event.set("health", &self.health);
        if !self.status.is_empty() {
            event.set("status", &self.status);
        }
        self.public_details.log(event);
    }
}

/// Public details for an active [`Mon`]'s position.
pub struct MonPositionDetails<'d> {
    pub name: &'d str,
    pub player_id: &'d str,
    pub side_position: usize,
}

impl Display for MonPositionDetails<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.name, self.player_id, self.side_position)
    }
}

/// A single move slot for a Mon.
#[derive(Clone)]
pub struct MoveSlot {
    pub id: Id,
    pub name: String,
    pub pp: u8,
    pub max_pp: u8,
    pub target: MoveTarget,
    pub disabled: bool,
    pub disabled_source: Option<String>,
    pub used: bool,
    pub simulated: bool,
}

/// Data for a single [`Mon`] when a player is requested an action on their entire team.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonTeamRequestData {
    pub name: String,
    pub species_name: String,
    pub level: u8,
    pub gender: Gender,
    pub shiny: bool,
    pub health: String,
    pub status: String,
    pub active: bool,
    pub side_position: Option<usize>,
    pub stats: PartialStatTable,
    pub moves: Vec<String>,
    pub ability: String,
    pub item: Option<String>,
    pub ball: String,
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
    pub target: MoveTarget,
    pub disabled: bool,
}

/// Request for a single [`Mon`] to move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonMoveRequest {
    /// Team position of the active Mon.
    pub team_position: usize,
    /// Available moves.
    pub moves: Vec<MonMoveSlotData>,
    /// Is the Mon trapped?
    pub trapped: bool,
    /// Can the Mon Mega Evolve?
    pub can_mega_evo: bool,
}

/// The outcome of a move used on a single turn of battle.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcome {
    Skipped,
    Failed,
    Success,
}

impl MoveOutcome {
    pub fn success(&self) -> bool {
        match self {
            Self::Success => true,
            _ => false,
        }
    }
}

impl From<MoveOutcome> for bool {
    fn from(value: MoveOutcome) -> Self {
        value.success()
    }
}

/// A [`Mon`] in a battle, which battles against other Mons.
pub struct Mon {
    pub player: usize,
    pub side: usize,

    pub name: String,
    pub species: String,

    pub active: bool,
    pub active_turns: u32,
    pub active_move_actions: u32,
    pub active_position: usize,
    pub team_position: usize,

    pub base_stored_stats: StatTable,
    pub stats: StatTable,
    pub boosts: BoostTable,
    pub ivs: StatTable,
    pub evs: StatTable,
    pub level: u8,
    pub nature: Nature,
    pub gender: Gender,
    pub shiny: bool,
    pub ball: String,

    pub base_move_slots: Vec<MoveSlot>,
    pub move_slots: Vec<MoveSlot>,

    pub ability: String,
    pub ability_priority: u32,

    pub types: Vec<Type>,
    pub hidden_power_type: Type,

    pub item: Option<String>,

    pub hp: u16,
    pub max_hp: u16,
    pub status: Option<Id>,
    pub speed: u16,
    pub fainted: bool,
    pub needs_switch: bool,
    pub force_switch: bool,
    pub skip_before_switch_out: bool,
    pub trapped: bool,
    pub can_mega_evo: bool,

    /// The move the Mon is actively performing.
    pub active_move: Option<MoveHandle>,
    /// The last move selected for the Mon.
    pub last_move_selected: Option<MoveHandle>,
    /// The last move used by the Mon, which can be different from `last_move` if that move
    /// executed a different move (like Metronome).
    pub last_move_used: Option<MoveHandle>,

    pub active_target: Option<MonHandle>,
    pub move_this_turn_outcome: Option<MoveOutcome>,
    pub last_move_target: Option<isize>,
    pub hurt_this_turn: u16,
}

// Construction and initialization logic.
impl Mon {
    /// Creates a new [`Mon`] instance from [`MonData`].
    pub fn new(data: MonData, team_position: usize, dex: &Dex) -> Result<Self, Error> {
        let name = data.name;
        let species_name = data.species;
        let ivs = data.ivs;
        let evs = data.evs;
        let level = data.level;
        let nature = data.nature;
        let gender = data.gender;
        let shiny = data.shiny;
        let ball = data.ball;
        let ability = data.ability;
        let item = data.item;

        let mut base_move_slots = Vec::with_capacity(data.moves.len());
        for (i, move_name) in data.moves.iter().enumerate() {
            let mov = dex.moves.get(move_name).into_result()?;
            let max_pp = if mov.data.no_pp_boosts {
                mov.data.pp
            } else {
                let boosts = *data.pp_boosts.get(i).unwrap_or(&0).max(&3) as u32;
                ((mov.data.pp as u32) * (boosts + 5) / 5) as u8
            };
            base_move_slots.push(MoveSlot {
                id: mov.id().clone(),
                name: mov.data.name.clone(),
                pp: max_pp,
                max_pp,
                target: mov.data.target.clone(),
                disabled: false,
                disabled_source: None,
                used: false,
                simulated: false,
            })
        }

        let move_slots = base_move_slots.clone();

        let species = dex.species.get(&species_name).into_result()?;
        let mut types = Vec::with_capacity(4);
        types.push(species.data.primary_type);
        if let Some(secondary_type) = species.data.secondary_type {
            types.push(secondary_type);
        }
        // TODO: Hidden Power type calculation.
        let hidden_power_type = data
            .hidden_power_type
            .unwrap_or(calculate_hidden_power_type(&ivs));

        Ok(Self {
            player: usize::MAX,
            side: usize::MAX,

            name,
            species: species_name,

            active: false,
            active_turns: 0,
            active_move_actions: 0,
            active_position: usize::MAX,
            team_position,

            base_stored_stats: StatTable::default(),
            stats: StatTable::default(),
            boosts: BoostTable::default(),
            ivs,
            evs,
            level,
            nature,
            gender,
            shiny,
            ball,

            base_move_slots,
            move_slots,

            ability,
            ability_priority: 0,

            types,
            hidden_power_type,

            item,

            hp: 0,
            max_hp: 0,
            status: None,
            speed: 0,
            fainted: false,
            needs_switch: false,
            force_switch: false,
            skip_before_switch_out: false,
            trapped: false,
            can_mega_evo: false,

            active_move: None,
            last_move_selected: None,
            last_move_used: None,

            active_target: None,
            move_this_turn_outcome: None,
            last_move_target: None,
            hurt_this_turn: 0,
        })
    }

    /// Initializes a Mon for battle.
    ///
    /// This *must* be called at the very beginning of a battle, as it sets up important fields on
    /// the Mon, such as its stats.
    pub fn initialize(context: &mut MonContext) -> Result<(), Error> {
        Self::clear_volatile(context)?;
        let mon = context.mon_mut();
        mon.hp = mon.max_hp;
        Ok(())
    }
}

// Basic getters.
impl Mon {
    fn health(&self) -> String {
        if self.hp == 0 || self.max_hp == 0 {
            return "0".to_owned();
        }
        let ratio = Fraction::new(self.hp, self.max_hp);
        // Always round up to avoid returning 0 when the Mon is not fainted.
        let percentage = (ratio * 100).ceil();
        format!("{percentage}/100")
    }

    fn secret_health(&self) -> String {
        if self.hp == 0 || self.max_hp == 0 {
            return "0".to_owned();
        }
        format!("{}/{}", self.hp, self.max_hp)
    }

    /// Returns the public details for the Mon.
    pub fn public_details(&self) -> PublicMonDetails {
        PublicMonDetails {
            species_name: self.species.as_ref(),
            level: self.level,
            gender: self.gender.clone(),
            shiny: self.shiny,
        }
    }

    /// Returns the public details for the active Mon.
    pub fn active_details<'b>(context: &'b MonContext) -> Result<ActiveMonDetails<'b>, Error> {
        let mon = context.mon();
        Ok(ActiveMonDetails {
            public_details: mon.public_details(),
            name: &mon.name,
            player_id: context.player().id.as_ref(),
            side_position: Self::position_on_side(context)? + 1,
            health: mon.health(),
            status: mon
                .status
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or(String::default()),
        })
    }

    /// Returns the public details for the Mon when an action is made.
    pub fn position_details<'b>(context: &'b MonContext) -> Result<MonPositionDetails<'b>, Error> {
        Ok(MonPositionDetails {
            name: &context.mon().name,
            player_id: context.player().id.as_ref(),
            side_position: Self::position_on_side(context)? + 1,
        })
    }

    pub fn types(context: &MonContext) -> Result<Vec<Type>, Error> {
        // TODO: Run type event for the Mon, since there could be volatile effects here.
        if !context.mon().types.is_empty() {
            return Ok(context.mon().types.clone());
        }
        return Ok(Vec::from_iter([Type::Normal]));
    }

    pub fn has_type(context: &MonContext, typ: Type) -> Result<bool, Error> {
        let types = Self::types(context)?;
        return Ok(types.contains(&typ));
    }

    pub fn locked_move(context: &MonContext) -> Result<Option<String>, Error> {
        // TODO: Run locked move event for the Mon.
        return Ok(None);
    }

    pub fn moves(context: &MonContext) -> Result<Vec<MonMoveSlotData>, Error> {
        let locked_move = Self::locked_move(context)?;
        Self::moves_with_locked_move(context, locked_move.as_deref())
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
    pub fn position_on_side(context: &MonContext) -> Result<usize, Error> {
        let mon = context.mon();
        if !mon.active {
            return Err(battler_error!("Mon is not active"));
        }
        let player = context.player();
        let position = mon.active_position
            + player.position * context.battle().format.battle_type.active_per_player();
        Ok(position)
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
        let mon_position = Self::position_on_side(context)?;

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

        Ok(Mon::relative_location(
            mon_position,
            target_position,
            mon_side == target_side,
            mons_per_side,
        ))
    }

    pub fn get_target(context: &mut MonContext, target: isize) -> Result<Option<MonHandle>, Error> {
        if target == 0 {
            return Err(battler_error!("target cannot be 0"));
        }
        let mut side_context = context.pick_side_context(target > 0)?;
        let position = (target.abs() - 1) as usize;
        Side::mon_in_position(&mut side_context, position)
    }

    pub fn is_ally(&self, mon: &Mon) -> bool {
        self.side == mon.side
    }

    pub fn active_allies<'m>(context: &'m mut MonContext) -> impl Iterator<Item = MonHandle> + 'm {
        let side = context.side().index;
        context.battle().active_mon_handles_on_side(side)
    }

    pub fn adjacent_allies(
        context: &mut MonContext,
    ) -> Result<impl Iterator<Item = Option<MonHandle>>, Error> {
        let position = Mon::position_on_side(context)?;
        let context = context.as_side_context_mut();
        let left = if position > 0 {
            Side::mon_in_position(context, position - 1)?
        } else {
            None
        };
        let right = Side::mon_in_position(context, position + 1)?;
        Ok(iter::once(left).chain(iter::once(right)))
    }

    pub fn adjacent_allies_and_self(
        context: &mut MonContext,
    ) -> Result<impl Iterator<Item = Option<MonHandle>>, Error> {
        Ok(Self::adjacent_allies(context)?.chain(iter::once(Some(context.mon_handle()))))
    }

    pub fn is_foe(&self, mon: &Mon) -> bool {
        self.side != mon.side
    }

    pub fn active_foes<'m>(context: &'m mut MonContext) -> impl Iterator<Item = MonHandle> + 'm {
        let foe_side = context.foe_side().index;
        context.battle().active_mon_handles_on_side(foe_side)
    }

    pub fn adjacent_foes(
        context: &mut MonContext,
    ) -> Result<impl Iterator<Item = Option<MonHandle>>, Error> {
        let position = Mon::position_on_side(context)?;
        let mons_per_side = context.battle().max_side_length();
        if position >= mons_per_side {
            return Err(battler_error!("Mon position {position} is out of bounds"));
        }
        let flipped_position = mons_per_side - position - 1;
        let mut context = context.foe_side_context()?;
        let left = if flipped_position > 0 {
            Side::mon_in_position(&mut context, flipped_position - 1)?
        } else {
            None
        };
        let center = Side::mon_in_position(&mut context, flipped_position)?;
        let right = Side::mon_in_position(&mut context, flipped_position + 1)?;
        Ok(iter::once(left)
            .chain(iter::once(center))
            .chain(iter::once(right)))
    }

    fn calculate_stat_internal(
        context: &mut MonContext,
        stat: Stat,
        unboosted: bool,
        boost: Option<i8>,
        unmodified: bool,
        modifier: Option<Fraction<u16>>,
        stat_user: MonHandle,
        stat_target: Option<MonHandle>,
    ) -> Result<u16, Error> {
        if stat == Stat::HP {
            return Err(battler_error!(
                "HP should be read directly, not by calling get_stat"
            ));
        }

        let mut value = context.mon().stats.get(stat);
        if !unboosted {
            let boost = match boost {
                Some(boost) => boost,
                None => context.mon().boosts.get(stat.try_into()?),
            };
            // TODO: ModifyBoost event. Should apply to stat_user.
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
            let boost = boost.max(6).min(-6);
            let boost_fraction = &BOOST_TABLE[boost.abs() as usize];
            if boost >= 0 {
                value = boost_fraction.mul(value).floor();
            } else {
                value = boost_fraction.inverse().mul(value).floor();
            }
        }
        if !unmodified {
            // TODO: ModifyStat event (individual per stat).
            let modifier = modifier.unwrap_or(Fraction::from(1));
            value = modify(value, modifier);
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
        stat_user: MonHandle,
        stat_target: MonHandle,
    ) -> Result<u16, Error> {
        Self::calculate_stat_internal(
            context,
            stat,
            false,
            Some(boost),
            false,
            Some(modifier),
            stat_user,
            Some(stat_target),
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
        Self::calculate_stat_internal(
            context,
            stat,
            unboosted,
            None,
            unmodified,
            None,
            context.mon_handle(),
            None,
        )
    }

    /// Calculates the speed value to use for battle action ordering.
    pub fn action_speed(context: &mut MonContext) -> Result<u16, Error> {
        let speed = Self::get_stat(context, Stat::Spe, false, false)?;
        // TODO: If Trick Room, return u16::MAX - speed.
        Ok(speed)
    }

    fn move_slot(&self, move_id: &Id) -> Option<&MoveSlot> {
        self.move_slots
            .iter()
            .find(|move_slot| &move_slot.id == move_id)
    }

    fn move_slot_mut(&mut self, move_id: &Id) -> Option<&mut MoveSlot> {
        self.move_slots
            .iter_mut()
            .find(|move_slot| &move_slot.id == move_id)
    }
}

// Request getters.
impl Mon {
    pub fn team_request_data(context: &MonContext) -> Result<MonTeamRequestData, Error> {
        let side_position = if context.mon().active {
            Some(Self::position_on_side(context)?)
        } else {
            None
        };
        Ok(MonTeamRequestData {
            name: context.mon().name.clone(),
            species_name: context.mon().species.clone(),
            level: context.mon().level,
            gender: context.mon().gender.clone(),
            shiny: context.mon().shiny,
            health: context.mon().secret_health(),
            status: context
                .mon()
                .status
                .as_ref()
                .map(|id| id.to_string())
                .unwrap_or(String::default()),
            active: context.mon().active,
            side_position,
            stats: context.mon().base_stored_stats.without_hp(),
            moves: context
                .mon()
                .move_slots
                .iter()
                .map(|move_slot| move_slot.name.clone())
                .collect(),
            ability: context.mon().ability.clone(),
            item: context.mon().item.clone(),
            ball: context.mon().ball.clone(),
        })
    }

    pub fn move_request(context: &mut MonContext) -> Result<MonMoveRequest, Error> {
        let mut locked_move = Self::locked_move(context)?;
        let mut moves = Self::moves_with_locked_move(context, locked_move.as_deref())?;
        let has_usable_move = moves.iter().any(|mov| !mov.disabled);
        if moves.is_empty() || !has_usable_move {
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
            trapped: false,
            can_mega_evo: false,
        };

        let can_switch = Player::can_switch(context.as_player_context_mut())?;
        if can_switch && context.mon().trapped {
            request.trapped = true;
        }

        if locked_move.is_none() {
            request.can_mega_evo = context.mon().can_mega_evo;
        }

        Ok(request)
    }
}

impl Mon {
    fn clear_volatile(context: &mut MonContext) -> Result<(), Error> {
        context.mon_mut().needs_switch = false;
        let species = context.mon().species.clone();
        Self::set_species(context, species, false)?;
        Ok(())
    }

    fn set_species(
        context: &mut MonContext,
        species: String,
        transform: bool,
    ) -> Result<(), Error> {
        let species = context
            .battle()
            .dex
            .species
            .get(species.as_str())
            .into_result()?;

        let mon = context.mon();
        let mut stats = calculate_mon_stats(
            &species.data.base_stats,
            &mon.ivs,
            &mon.evs,
            mon.level,
            mon.nature,
        );
        // Forced max HP always overrides stat calculations.
        if let Some(max_hp) = species.data.max_hp {
            stats.hp = max_hp;
        }

        let mon = context.mon_mut();

        // Max HP has not yet been set (beginning of the battle).
        if mon.max_hp == 0 {
            mon.max_hp = stats.hp;
        }
        // Transformations should keep the original "base" stats for the Mon.
        if !transform {
            mon.base_stored_stats = stats.clone();
        }
        mon.stats = mon
            .stats
            .entries()
            .map(|(stat, _)| (stat, stats.get(stat)))
            .collect();
        mon.speed = mon.stats.spe;
        Ok(())
    }

    fn moves_with_locked_move(
        context: &MonContext,
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
                    target: MoveTarget::User,
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
                    pp: locked_move.pp,
                    max_pp: locked_move.max_pp,
                    target: locked_move.target.clone(),
                    disabled: locked_move.disabled,
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
            .map(|move_slot| {
                let mov = context
                    .battle()
                    .dex
                    .moves
                    .get_by_id(&move_slot.id)
                    .into_result()?;
                // Some moves may have a special target for non-Ghost types.
                let target = if let Some(non_ghost_target) = &mov.data.non_ghost_target {
                    if !Self::has_type(context, Type::Ghost)? {
                        non_ghost_target.clone()
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
                Ok(MonMoveSlotData {
                    name: move_slot.name,
                    id: move_slot.id,
                    pp: move_slot.pp,
                    max_pp: move_slot.max_pp,
                    target,
                    disabled,
                })
            })
            .collect()
    }

    /// Switches the Mon into the given position for the player.
    pub fn switch_in(context: &mut MonContext, position: usize) {
        let mon = context.mon_mut();
        mon.active = true;
        mon.active_turns = 0;
        mon.active_move_actions = 0;
        mon.active_position = position;
        for move_slot in &mut mon.move_slots {
            move_slot.used = false;
        }
        let ability_priority = context.battle_mut().next_ability_priority();
        let mon = context.mon_mut();
        mon.ability_priority = ability_priority
    }

    /// Switches the Mon out of the given position for the player.
    pub fn switch_out(&mut self) {
        self.active = false;
        self.active_position = usize::MAX;
        self.needs_switch = false;
    }

    /// Sets the active move and target.
    pub fn set_active_move(&mut self, active_move: MoveHandle, target: Option<MonHandle>) {
        self.active_move = Some(active_move);
        self.active_target = target;
    }

    /// Clears the active move.
    pub fn clear_active_move(&mut self) {
        self.active_move = None;
        self.active_target = None;
    }

    /// Deducts PP from the given move.
    ///
    /// Returns if the move can be used with the PP deduction.
    pub fn deduct_pp(&mut self, move_id: &Id, amount: u8) -> bool {
        if let Some(move_slot) = self.move_slot_mut(move_id) {
            move_slot.used = true;
            if amount > move_slot.pp {
                move_slot.pp = 0;
                return false;
            } else {
                move_slot.pp -= amount;
                return true;
            }
        }
        return false;
    }

    /// Checks if the Mon is immune to the given type.
    pub fn is_immune(context: &mut MonContext, typ: Type) -> Result<bool, Error> {
        if context.mon().fainted {
            return Ok(false);
        }

        let types = Self::types(context)?;
        // TODO: NegateImmunity event.
        // TODO: Handle immunity from being grounded and potentially other volatile conditions.
        let immune = context.battle().check_type_immunity(typ, &types);

        Ok(immune)
    }

    pub fn type_effectiveness(context: &mut MonContext, typ: Type) -> Result<i8, Error> {
        let mut total = 0;
        for defense in Mon::types(context)? {
            let modifier = context.battle().check_type_effectiveness(typ, defense);
            // TODO: Effectiveness event.
            total += modifier;
        }
        Ok(total)
    }

    pub fn damage(
        context: &mut MonContext,
        damage: u16,
        source: Option<MonHandle>,
        effect: Option<EffectHandle>,
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

    pub fn faint(
        context: &mut MonContext,
        source: Option<MonHandle>,
        effect: Option<EffectHandle>,
    ) -> Result<(), Error> {
        if context.mon().fainted {
            return Ok(());
        }
        context.mon_mut().hp = 0;
        context.mon_mut().needs_switch = false;
        let mon_handle = context.mon_handle();
        context.battle_mut().faint_queue.push(FaintEntry {
            target: mon_handle,
            source,
            effect,
        });
        Ok(())
    }

    pub fn clear_state_on_faint(context: &mut MonContext) -> Result<(), Error> {
        // TODO: End event for ability.
        Mon::clear_volatile(context)?;
        context.mon_mut().fainted = true;
        context.mon_mut().active = false;
        Ok(())
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
