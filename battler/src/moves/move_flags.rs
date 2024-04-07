use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// Move flags, which categorize moves for miscellaneous behavior (such as bans or side effects).
#[derive(
    Debug, Clone, PartialEq, Eq, Hash, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum MoveFlags {
    /// Lowers a target's accuracy.
    #[string = "AccuracyLowering"]
    AccuracyLowering,
    /// Bypasses a target's substitute.
    #[string = "BypassSubstitute"]
    #[alias = "BypassSub"]
    BypassSubstitute,
    /// A bite move.
    #[string = "Bite"]
    Bite,
    /// A bullet move.
    #[string = "Bullet"]
    Bullet,
    /// A charge move, which causes a Mon to be unable to move between turns.
    #[string = "Charge"]
    Charge,
    /// Makes contact.
    #[string = "Contact"]
    Contact,
    /// A dance move.
    #[string = "Dance"]
    Dance,
    /// Can target a Mon no matter its distance from the user.
    #[string = "Distance"]
    Distance,
    /// Raises the user's evasion.
    #[string = "EvasionRaising"]
    EvasionRaising,
    /// Cannot be selected by Copycat.
    #[string = "FailCopycat"]
    FailCopycat,
    /// Cannot be targeted by Encore.
    #[string = "FailEncore"]
    FailEncore,
    /// Cannot be repeated by Instruct.
    #[string = "FailInstruct"]
    FailInstruct,
    /// Cannot be selected by Me First.
    #[string = "FailMeFirst"]
    FailMeFirst,
    /// Cannot be copied by Mimic.
    #[string = "FailMimic"]
    FailMimic,
    /// Damages a target in the future.
    #[string = "Future"]
    Future,
    /// Cannot be used during Gravity's effect.
    #[string = "Gravity"]
    Gravity,
    /// Cannot be used during Heal Block's effect.
    #[string = "Heal"]
    Heal,
    /// Can be used by Metronome.
    #[string = "Metronome"]
    Metronome,
    /// Can be copied by Mirror Move.
    #[string = "Mirror"]
    Mirror,
    /// Additional PP is deducted due to Pressure when it ordinarily would not be.
    #[string = "MustPressure"]
    MustPressure,
    /// Cannot be selected by Assist.
    #[string = "NoAssist"]
    NoAssist,
    /// Cannot be made to hit twice via Parental Bond.
    #[string = "NoParentalBond"]
    NoParentalBond,
    /// Cannot be selected by sleep talk.
    #[string = "NoSleepTalk"]
    NoSleepTalk,
    /// A one-hit KO move.
    #[string = "OHKO"]
    OHKO,
    /// Gems will not active, and cannot be redirected by Storm Drain or Lightning Rod.
    #[string = "PledgeCombo"]
    PledgeCombo,
    /// A powder move.
    #[string = "Powder"]
    Powder,
    /// Blocked by protection moves.
    #[string = "Protect"]
    Protect,
    /// A pulse move.
    #[string = "Pulse"]
    Pulse,
    /// A punch move.
    #[string = "Punch"]
    Punch,
    /// A move requiring recharge if successful.
    #[string = "Recharge"]
    Recharge,
    /// A reflectable move.
    #[string = "Reflectable"]
    Reflectable,
    /// A sleep-inducing move.
    #[string = "SleepInducing"]
    SleepInducing,
    /// A slicing move.
    #[string = "Slicing"]
    Slicing,
    /// Can be stolen from the original user via Snatch.
    #[string = "Snatch"]
    Snatch,
    /// A sound move.
    #[string = "Sound"]
    Sound,
    /// A thawing move.
    #[string = "Thawing"]
    Thawing,
    /// A wind move.
    #[string = "Wind"]
    Wind,
}

#[cfg(test)]
mod move_flags_tests {
    use crate::{
        common::{
            test_string_deserialization,
            test_string_serialization,
        },
        moves::MoveFlags,
    };

    #[test]
    fn serializes_to_string() {
        test_string_serialization(MoveFlags::BypassSubstitute, "BypassSubstitute");
        test_string_serialization(MoveFlags::Bite, "Bite");
        test_string_serialization(MoveFlags::Thawing, "Thawing");
    }

    #[test]
    fn deserializes_lowercase() {
        test_string_deserialization("charge", MoveFlags::Charge);
        test_string_deserialization("noparentalbond", MoveFlags::NoParentalBond);
        test_string_deserialization("reflectable", MoveFlags::Reflectable);
    }
}
