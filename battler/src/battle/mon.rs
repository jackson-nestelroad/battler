use std::borrow::Cow;

use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        calculate_hidden_power_type,
        calculate_mon_stats,
        MonContext,
        Player,
    },
    battler_error,
    common::{
        Error,
        Fraction,
        Id,
        Identifiable,
    },
    dex::Dex,
    log::BattleLoggable,
    mons::{
        Gender,
        Nature,
        PartialStatTable,
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

impl BattleLoggable for PublicMonDetails<'_> {
    fn log<'s>(&'s self, items: &mut Vec<Cow<'s, str>>) {
        items.push(self.species_name.into());
        items.push(self.level.to_string().into());
        items.push(self.gender.to_string().into());
        if self.shiny {
            items.push("shiny".into());
        }
    }
}

/// Public details for an active [`Mon`], which are shared to both sides of a battle when the Mon
/// appears in the battle.
pub struct ActiveMonDetails<'d> {
    pub public_details: PublicMonDetails<'d>,
    pub name: &'d str,
    pub player_id: &'d str,
    pub position: usize,
    pub health: String,
    pub status: String,
}

impl BattleLoggable for ActiveMonDetails<'_> {
    fn log<'s>(&'s self, items: &mut Vec<Cow<'s, str>>) {
        items.push(self.player_id.into());
        items.push(self.position.to_string().into());
        items.push(self.name.into());
        items.push(self.health.as_str().into());
        items.push(self.status.as_str().into());
        self.public_details.log(items);
    }
}

/// A single move slot for a Mon.
#[derive(Clone)]
pub struct MoveSlot {
    id: Id,
    name: String,
    pp: u8,
    max_pp: u8,
    target: MoveTarget,
    disabled: bool,
    disabled_source: Option<String>,
    used: bool,
    simulated: bool,
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

/// A [`Mon`] in a battle, which battles against other Mons.
pub struct Mon {
    pub player: usize,

    pub name: String,
    pub species: String,

    pub active: bool,
    pub active_turns: u32,
    pub active_move_actions: u32,
    pub position: usize,

    pub base_stored_stats: StatTable,
    pub stats: StatTable,
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
    pub status: Option<String>,
    pub speed: u16,
    pub fainted: bool,
    pub needs_switch: bool,
    pub trapped: bool,
    pub can_mega_evo: bool,
}

// Construction and initialization logic.
impl Mon {
    /// Creates a new [`Mon`] instance from [`MonData`].
    pub fn new(data: MonData, dex: &Dex) -> Result<Self, Error> {
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

            name,
            species: species_name,

            active: false,
            active_turns: 0,
            active_move_actions: 0,
            position: usize::MAX,

            base_stored_stats: StatTable::default(),
            stats: StatTable::default(),
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
            trapped: false,
            can_mega_evo: false,
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
    pub fn active_details<'b>(context: &'b MonContext) -> ActiveMonDetails<'b> {
        let mon = context.mon();
        ActiveMonDetails {
            public_details: mon.public_details(),
            name: &mon.name,
            player_id: context.player().id(),
            position: mon.position,
            health: mon.health(),
            status: mon.status.clone().unwrap_or(String::default()),
        }
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
}

// Request getters.
impl Mon {
    pub fn team_request_data(&self) -> MonTeamRequestData {
        MonTeamRequestData {
            name: self.name.clone(),
            species_name: self.species.clone(),
            level: self.level,
            gender: self.gender.clone(),
            shiny: self.shiny,
            health: self.secret_health(),
            status: self.status.clone().unwrap_or(String::default()),
            active: self.active,
            stats: self.base_stored_stats.without_hp(),
            moves: self
                .move_slots
                .iter()
                .map(|move_slot| move_slot.name.clone())
                .collect(),
            ability: self.ability.clone(),
            item: self.item.clone(),
            ball: self.ball.clone(),
        }
    }

    pub fn move_request(context: &MonContext) -> Result<MonMoveRequest, Error> {
        let mut locked_move = Self::locked_move(context)?;
        let mut moves = Self::moves_with_locked_move(context, locked_move.as_deref())?;
        if moves.is_empty() {
            // No moves, the Mon must use Struggle.
            locked_move = Some("struggle".to_owned());
            moves = Vec::from_iter([MonMoveSlotData {
                name: "Struggle".to_owned(),
                target: MoveTarget::RandomNormal,
                pp: 0,
                max_pp: 0,
                disabled: false,
            }]);
        }

        let mut request = MonMoveRequest {
            team_position: context.mon().position,
            moves,
            trapped: false,
            can_mega_evo: false,
        };

        let can_switch = Player::switchable_mons(context.as_player_context()).count() > 0;
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

        let mon = context.mon_mut();
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
                Ok(MonMoveSlotData {
                    name: move_slot.name,
                    pp: move_slot.pp,
                    max_pp: move_slot.max_pp,
                    target,
                    disabled: false,
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
        mon.position = position;
        for move_slot in &mut mon.move_slots {
            move_slot.used = false;
        }
        let ability_priority = context.battle_mut().next_ability_priority();
        let mon = context.mon_mut();
        mon.ability_priority = ability_priority
    }

    /// Switches the Mon out of hte given position for the player.
    pub fn switch_out(&mut self) {
        self.active = false;
        self.position = usize::MAX;
    }
}
