use battler_data::{
    Fraction,
    Stat,
    Type,
};
use serde::{
    Deserialize,
    Serialize,
};

/// The damage calculation phase in which a modifier applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DamageModifierEvent {
    /// Modifies the move's base power (e.g., Technician, Charcoal, Terrains).
    BasePower,
    /// Modifies a specific stat calculation (e.g., Choice Band for Atk, Huge Power).
    Stat(Stat),
    /// Modifies pre-random damage (e.g., Weather, Critical hits, Spread modifier).
    PreRandom,
    /// Modifies final damage (e.g., Life Orb, Multiscale, Resist Berries).
    Damage,
}

/// The condition under which a damage modifier is active.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DamageModifierCondition {
    /// Applies unconditionally.
    Always,
    /// Applies when the bearer's health is at or below the given fraction of max HP (e.g., Blaze/Overgrow/Torrent at <= 1/3 HP).
    HealthBelow(Fraction<u32>),
    /// Applies when the bearer is at 100% full HP (e.g., Multiscale, Shadow Shield).
    HealthFull,
    /// Applies when the target is at 100% full HP.
    TargetHealthFull,
    /// Applies when the move has the specified primary type (e.g., Charcoal for Fire, Magnet for Electric).
    MoveType(Type),
    /// Applies when the move makes contact (e.g., Tough Claws).
    ContactMove,
    /// Applies when the move is a bite move (e.g., Strong Jaw).
    BiteMove,
    /// Applies when the move is a punch move (e.g., Iron Fist).
    PunchMove,
    /// Applies when the move is a sound move.
    SoundMove,
    /// Applies when the move is a pulse/aura move (e.g., Mega Launcher).
    PulseMove,
    /// Applies when the move inflicts recoil (e.g., Reckless).
    RecoilMove,
    /// Applies when the move is super-effective against the target (e.g., Resist berries, Neuroforce).
    SuperEffective,
    /// Applies when the move is not very effective / resisted (e.g., Tinted Lens).
    Resisted,
    /// Applies when the move's base power is at or below the threshold (e.g., Technician <= 60 BP).
    BasePowerMax(u32),
    /// Applies when the attacker is grounded on the field.
    GroundedAttacker,
    /// Applies when the specified weather is active (e.g., Solar Beam reduction in rain).
    WeatherActive(String),
    /// Applies when the specified terrain is active (e.g., Terrain Pulse, Electric Terrain boost).
    TerrainActive(String),
}

/// A declarative modifier to damage calculations, extracted from fxlang or item/ability data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageModifier {
    /// Calculation phase.
    pub event: DamageModifierEvent,
    /// Multiplier fraction applied to the value.
    pub multiplier: Fraction<u32>,
    /// Activation condition.
    #[serde(default = "default_condition")]
    pub condition: DamageModifierCondition,
    /// Human-readable explanation for calculation traces and debugging.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

fn default_condition() -> DamageModifierCondition {
    DamageModifierCondition::Always
}

impl DamageModifier {
    /// Creates a new damage modifier.
    pub fn new(
        event: DamageModifierEvent,
        multiplier: Fraction<u32>,
        condition: DamageModifierCondition,
        description: impl Into<String>,
    ) -> Self {
        Self {
            event,
            multiplier,
            condition,
            description: description.into(),
        }
    }
}

/// Fixed damage formulas that bypass standard damage equations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixedDamage {
    /// Flat constant damage (e.g., Dragon Rage = 40, Sonic Boom = 20).
    Constant(u32),
    /// Deals damage equal to attacker's level (e.g., Seismic Toss, Night Shade).
    Level,
    /// Deals damage equal to a fraction of the target's current HP (e.g., Super Fang = 1/2).
    FractionTargetCurrentHp(Fraction<u32>),
    /// Deals damage equal to a fraction of the target's max HP.
    FractionTargetMaxHp(Fraction<u32>),
    /// Deals damage equal to the difference between defender HP and attacker HP (e.g., Endeavor).
    HpDifference,
    /// Deals damage in a scaled level range (e.g., Psywave 50% to 150% of level).
    LevelRange(u32, u32),
}

/// Engine simulation flags for moves and effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EffectFlag {
    /// Breaks Light Screen, Reflect, and Aurora Veil before dealing damage (e.g., Brick Break).
    BreaksScreens,
    /// Bypasses a target's Substitute.
    IgnoresSubstitute,
    /// Bypasses Protect / Detect.
    IgnoresProtect,
    /// Move is an OHKO attack (e.g., Fissure, Sheer Cold, Horn Drill, Guillotine).
    Ohko,
}
