use crate::{
    battle::MonContext,
    common::{
        Error,
        Id,
        Identifiable,
        MaybeOwnedString,
    },
    dex::Dex,
    log::BattleLoggable,
    mons::Gender,
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
            items.push("true".into());
        }
    }
}

/// Public details for an active [`Mon`], which are shared to both sides of a battle when the Mon
/// appears in the battle.
pub struct ActiveMonDetals<'d> {
    pub public_details: PublicMonDetails<'d>,
    pub name: &'d str,
    pub player_id: &'d str,
    pub position: usize,
}

impl BattleLoggable for ActiveMonDetals<'_> {
    fn log<'s>(&'s self, items: &mut Vec<MaybeOwnedString<'s>>) {
        items.push(self.player_id.into());
        items.push(self.position.to_string().into());
        items.push(self.name.into());
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

    active: bool,
    active_turns: u32,
    active_move_actions: u32,
    position: usize,
    base_move_slots: Vec<MoveSlot>,
    move_slots: Vec<MoveSlot>,
    ability_priority: u32,
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
            active: false,
            active_turns: 0,
            active_move_actions: 0,
            position: 0,
            base_move_slots,
            move_slots,
            ability_priority: 0,
        })
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
    pub fn active_details<'b>(context: &'b MonContext) -> ActiveMonDetals<'b> {
        // TODO: This should contain HP information.
        ActiveMonDetals {
            public_details: context.mon().public_details(),
            name: &context.mon().data.name,
            player_id: context.player().id(),
            position: context.mon().position,
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
}
