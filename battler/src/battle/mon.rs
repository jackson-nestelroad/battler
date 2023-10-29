use crate::{
    battle::{
        calculate_mon_stats,
        MonContext,
    },
    common::{
        Error,
        Fraction,
        Id,
        Identifiable,
        MaybeOwnedString,
    },
    dex::Dex,
    log::BattleLoggable,
    mons::{
        Gender,
        StatTable,
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
    fn log<'s>(&'s self, items: &mut Vec<MaybeOwnedString<'s>>) {
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
    fn log<'s>(&'s self, items: &mut Vec<MaybeOwnedString<'s>>) {
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
    target: MoveTarget,
    disabled: bool,
    disabled_source: Option<String>,
    used: bool,
    simulated: bool,
}

/// A [`Mon`] in a battle, which battles against other Mons.
pub struct Mon {
    pub data: MonData,
    pub player: usize,

    hp: u16,
    max_hp: u16,
    speed: u16,
    active: bool,
    active_turns: u32,
    active_move_actions: u32,
    pub position: usize,
    base_stored_stats: StatTable,
    stats: StatTable,
    base_move_slots: Vec<MoveSlot>,
    move_slots: Vec<MoveSlot>,
    ability_priority: u32,

    pub fainted: bool,
    pub needs_switch: bool,
    pub trapped: bool,
}

// Block for getters.
impl Mon {
    pub fn active(&self) -> bool {
        self.active
    }
}

impl Mon {
    /// Creates a new [`Mon`] instance from [`MonData`].
    pub fn new(data: MonData, dex: &Dex) -> Result<Self, Error> {
        let mut base_move_slots = Vec::with_capacity(data.moves.len());
        for move_name in &data.moves {
            let mov = dex.moves.get(move_name).into_result()?;
            let pp = if mov.data.no_pp_boosts {
                mov.data.pp
            } else {
                ((mov.data.pp as u32) * 8 / 5) as u8
            };
            base_move_slots.push(MoveSlot {
                id: mov.id().clone(),
                name: mov.data.name.clone(),
                pp,
                target: mov.data.target.clone(),
                disabled: false,
                disabled_source: None,
                used: false,
                simulated: false,
            })
        }

        let move_slots = base_move_slots.clone();

        Ok(Self {
            data,
            player: usize::MAX,
            hp: 0,
            max_hp: 0,
            speed: 0,
            active: false,
            active_turns: 0,
            active_move_actions: 0,
            position: 0,
            base_stored_stats: StatTable::default(),
            stats: StatTable::default(),
            base_move_slots,
            move_slots,
            ability_priority: 0,
            fainted: false,
            needs_switch: false,
            trapped: false,
        })
    }

    /// Initializes a Mon for battle.
    ///
    /// This *must* be called at the very beginning of a battle, as it sets up important fields on
    /// the Mon, such as its stats.
    pub fn initialize(context: &mut MonContext) -> Result<(), Error> {
        Mon::clear_volatile(context)?;
        let mon = context.mon_mut();
        mon.hp = mon.max_hp;
        Ok(())
    }

    fn clear_volatile(context: &mut MonContext) -> Result<(), Error> {
        context.mon_mut().needs_switch = false;
        let species = context.mon().data.species.clone();
        Mon::set_species(context, species, false)?;
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
        let mut stats = calculate_mon_stats(&species.data.base_stats, &context.mon().data);
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

    fn health(&self) -> String {
        if self.hp == 0 || self.max_hp == 0 {
            return "0".to_owned();
        }
        let ratio = Fraction::new(self.hp, self.max_hp);
        // Always round up to avoid returning 0 when the Mon is not fainted.
        let percentage = (ratio * 100).ceil();
        format!("{percentage}/100")
    }

    /// Returns the public details for the Mon.
    pub fn public_details(&self) -> PublicMonDetails {
        PublicMonDetails {
            species_name: &self.data.species,
            level: self.data.level,
            gender: self.data.gender.clone(),
            shiny: self.data.shiny,
        }
    }

    /// Returns the public details for the active Mon.
    pub fn active_details<'b>(context: &'b MonContext) -> ActiveMonDetails<'b> {
        let mon = context.mon();
        ActiveMonDetails {
            public_details: mon.public_details(),
            name: &mon.data.name,
            player_id: context.player().id(),
            position: mon.position,
            health: mon.health(),
            status: "".to_owned(),
        }
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
        self.position = 0;
    }
}
