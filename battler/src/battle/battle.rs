use crate::{
    battle::{
        BattleOptions,
        Request,
    },
    common::Error,
    dex::DataStore,
    log::Event,
    rng::{
        PseudoRandomNumberGenerator,
        RealPseudoRandomNumberGenerator,
    },
};

/// Battle engine option for how base damage should be randomized in the damage calculation.
#[derive(Debug)]
pub enum BattleEngineRandomizeBaseDamage {
    /// Randomize the base damage.
    ///
    /// This is the default behavior.
    Randomize,
    /// Only use the maximum base damage value.
    Max,
    /// Only use the minimum base damage value.
    Min,
}

/// Options that change how the battle engine itself behaves, which is not necessarily specific to
/// any individual battle.
///
/// Options defined here relate to how the battle engine is operated, so it is likely that these
/// options will be common across all battle instances.
#[derive(Debug)]
pub struct BattleEngineOptions {
    /// Should the [`Battle`] automatically continue when it is able to?
    ///
    /// If set to `true`, a [`Battle`] object will continue the battle as soon as it finds that it
    /// is able to. The best example of this is when a player makes a choice: if all players have
    /// made responded to their request, then the battle can automatically continue in the same
    /// method as the last player's choice.
    ///
    /// If set to `false`, [`Battle::continue_battle`] must be called to manually continue the
    /// battle (even at the start of the battle).
    pub auto_continue: bool,

    /// Should the [`Battle`] reveal the actual health of all Mons in the public battle logs?
    ///
    /// By default, the public logs will show the health of all Mons as a percentage (fraction out
    /// of 100). If this option is set to `true`, the battle will show the actual HP stat of each
    /// Mon.
    pub reveal_actual_health: bool,

    /// Function for creating the battle's random number generator.
    ///
    /// Primarily useful for tests where we wish to have fine-grained control over battle RNG.
    pub rng_factory: fn(seed: Option<u64>) -> Box<dyn PseudoRandomNumberGenerator>,

    /// Are players allowed to pass for unfainted Mons?
    ///
    /// By default, "pass" actions are forced when the player does not have enough Mons to fulfill
    /// all requirements. For example if a player has a team of 3 Mons for a doubles battle and 2
    /// faint at the same time, the player will is allowed to send one "switch" action and the
    /// other is forced to be a pass
    ///
    /// In all other cases, players cannot instruct their Mons to pass at the beginning of a turn.
    /// This prevents battles from getting into a stalemate position forever.
    ///
    /// If this property is set to `true`, players will be allowed to send "pass" actions. This is
    /// mostly useful for tests where we want to control one side while the other side sits
    /// passively.
    pub allow_pass_for_unfainted_mon: bool,

    /// Describes how base damage should be randomized in the damage calculation.
    ///
    /// By default, base damage is randomized early in the damage calculation. This property can
    /// control how the damage should be randomized. This is useful for tests against the damage
    /// calculator to discover the minimum and maximum damage values.
    pub randomize_base_damage: BattleEngineRandomizeBaseDamage,

    /// Should volatile statuses be logged?
    ///
    /// By default, volatile statuses are invisible to Mons, since they are used to implement
    /// complex interactions in the battle system. It may be helpful, especially for debugging
    /// purposes, to view all volatile statuses added to and removed from Mons through the course
    /// of a battle.
    pub log_volatile_statuses: bool,
}

impl Default for BattleEngineOptions {
    fn default() -> Self {
        Self {
            auto_continue: true,
            reveal_actual_health: false,
            rng_factory: |seed: Option<u64>| Box::new(RealPseudoRandomNumberGenerator::new(seed)),
            allow_pass_for_unfainted_mon: false,
            randomize_base_damage: BattleEngineRandomizeBaseDamage::Randomize,
            log_volatile_statuses: false,
        }
    }
}

/// An instance of a battle.
///
/// A battle has the following properties:
/// - Takes place on a single [`Field`][`crate::battle::Field`].
/// - Takes place between two [`Side`][`crate::battle::Side`]s.
/// - Receives input for a single [`Player`][`crate::battle::Player`].
/// - Features [`Mon`][`crate::battle::Mon`]s attacking one another in a turn-based manner.
/// - Adheres to a [`Format`][`crate::config::Format`].
pub trait Battle<'d, Options>: Sized
where
    Options: BattleOptions,
{
    /// Creates a new battle.
    fn new(
        options: Options,
        data: &'d dyn DataStore,
        engine_options: BattleEngineOptions,
    ) -> Result<Self, Error>;

    /// Has the battle started?
    fn started(&self) -> bool;
    /// Has the battle ended?
    fn ended(&self) -> bool;
    /// Does the battle have new battle logs since the last call to [`Self::new_logs`]?
    fn has_new_logs(&self) -> bool;
    /// Returns all battle logs.
    fn all_logs(&self) -> impl Iterator<Item = &str>;
    /// Returns new battle logs since the last call to [`Self::new_logs`].
    fn new_logs(&mut self) -> impl Iterator<Item = &str>;

    /// Logs a new battle event to the battle log.
    fn log(&mut self, event: Event);
    /// Logs many battle events to the battle log.
    fn log_many<I>(&mut self, events: I)
    where
        I: IntoIterator<Item = Event>;

    /// Starts the battle.
    fn start(&mut self) -> Result<(), Error>;
    /// Is the battle ready to continue?
    fn ready_to_continue(&mut self) -> Result<bool, Error>;
    /// Continues the battle.
    ///
    /// [`Self::ready_to_continue`] should return `Ok(true)` before this method
    /// is called.
    fn continue_battle(&mut self) -> Result<(), Error>;
    /// Returns all active requests for the battle, indexed by player ID.
    fn active_requests<'b>(&'b self) -> impl Iterator<Item = (String, Request)> + 'b;
    /// Returns the active request for the player ID.
    fn request_for_player(&self, player: &str) -> Option<Request>;
    /// Sets the player choice.
    fn set_player_choice(&mut self, player_id: &str, input: &str) -> Result<(), Error>;
}
