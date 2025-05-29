/// Flags that control how the [`Trainer`][`crate::trainer::Trainer`] AI scores decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TrainerFlag {
    /// Discourage moves which would immediately benefit the opponent or waste a turn.
    ///
    /// Virtually all trainers have this flag for discouraging certain behavior.
    Basic,

    /// Prioritize raw damage output by performing damage calculations.
    EvaluateAttackDamage,

    /// Encourage and discourage certain move effects in particular circumstances.
    Expert,

    /// Prioritize setup moves on the first turn of the battle.
    SetUpFirstTurn,

    /// Prioritize setting up when at higher HP thresholds and passing stat boosts to party members.
    BatonPass,

    /// Encourage moves which would benefit a partner.
    BenefitPartner,

    /// Discourage certain move effects at particular HP thresholds.
    ConsiderHealth,

    /// Set up weather when applicable.
    SetUpWeather,

    /// Encourage moves which harass or disrupt the opponent's strategy.
    HarassTheOpponent,

    /// Consider switching the active Mon out of battle.
    ConsiderSwitching,

    /// Require using Mons in order.
    UseMonsInOrder,

    /// Reserve the last Mon in the party for the last Mon of the battle.
    ReserveLastMon,

    /// Use items in the bag when applicable.
    UseItems,
}
