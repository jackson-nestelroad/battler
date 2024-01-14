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
}

impl Default for BattleEngineOptions {
    fn default() -> Self {
        Self {
            auto_continue: true,
            reveal_actual_health: false,
            rng_factory: |seed: Option<u64>| match seed {
                Some(seed) => Box::new(RealPseudoRandomNumberGenerator::new_with_seed(seed)),
                None => Box::new(RealPseudoRandomNumberGenerator::new()),
            },
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
    /// Sets the player choice.
    fn set_player_choice(&mut self, player_id: &str, input: &str) -> Result<(), Error>;
}
