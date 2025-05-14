use std::{
    ops::Div,
    sync::LazyLock,
};

use battler_data::{
    Fraction,
    Nature,
    Stat,
    StatTable,
    Type,
};

/// Calculates a Mon's actual stats.
pub fn calculate_mon_stats(
    base_stats: &StatTable,
    ivs: &StatTable,
    evs: &StatTable,
    level: u8,
    nature: Nature,
) -> StatTable {
    let mut stats = StatTable::default();
    for (stat, value) in base_stats {
        let value = 2 * value + ivs.get(stat) + evs.get(stat) / 4;
        let value = value * (level as u16) / 100;
        let value = if stat == Stat::HP {
            value + (level as u16) + 10
        } else {
            value + 5
        };
        stats.set(stat, value);
    }
    apply_nature_to_stats(stats, nature)
}

/// Applies the given nature to the stat table, returning the new stat table.
///
/// This calculation prevents overflow.
pub fn apply_nature_to_stats(mut stats: StatTable, nature: Nature) -> StatTable {
    let boosts = nature.boosts();
    let drops = nature.drops();

    if boosts == drops {
        return stats;
    }

    let boosted_stat = stats.get(boosts);
    let boosted_stat = boosted_stat + (boosted_stat * 10).div(100);
    stats.set(boosts, boosted_stat);

    let dropped_stat = stats.get(drops);
    let dropped_stat = dropped_stat - num::Integer::div_ceil(&(dropped_stat * 10), &100);
    stats.set(drops, dropped_stat);

    stats
}

/// Calculates the Hidden Power type based on IVs.
pub fn calculate_hidden_power_type(ivs: &StatTable) -> Type {
    static HIDDEN_POWER_STAT_ORDER: LazyLock<[Stat; 6]> = LazyLock::new(|| {
        [
            Stat::HP,
            Stat::Atk,
            Stat::Def,
            Stat::Spe,
            Stat::SpAtk,
            Stat::SpDef,
        ]
    });

    let mut hp_type = 0;
    let mut i = 1;
    for stat in *HIDDEN_POWER_STAT_ORDER {
        hp_type += i * (ivs.get(stat) & 1);
        i *= 2;
    }
    let hp_type = hp_type * 15 / 63;
    match hp_type {
        0 => Type::Fighting,
        1 => Type::Flying,
        2 => Type::Poison,
        3 => Type::Ground,
        4 => Type::Rock,
        5 => Type::Bug,
        6 => Type::Ghost,
        7 => Type::Steel,
        8 => Type::Fire,
        9 => Type::Water,
        10 => Type::Grass,
        11 => Type::Electric,
        12 => Type::Psychic,
        13 => Type::Ice,
        14 => Type::Dragon,
        15 => Type::Dark,
        // This should never happen.
        _ => Type::Normal,
    }
}

/// Applies the given modifier to the value.
///
/// Mostly used for stat calculations. Split off into its own function to help guarantee
/// consistency.
pub fn modify_32(value: u32, modifier: Fraction<u32>) -> u32 {
    // Pokemon Showdown uses this calculation, even though it produces some wrong values. For
    // example, 37 * 0.75 = 27.75, but this formula produces 28.
    //
    // We use this formula for consistency with their damage calculator.
    let modifier = modifier.numerator() * 4096 / modifier.denominator();
    ((value * modifier) + 2048 - 1) / 4096
}

#[cfg(test)]
mod calclulations_test {
    use battler_data::{
        Nature,
        StatTable,
        Type,
    };
    use serde::Deserialize;

    use crate::{
        battle::{
            apply_nature_to_stats,
            calculate_hidden_power_type,
            calculate_mon_stats,
        },
        common::read_test_cases,
    };

    #[test]
    fn nature_boosts_and_drops_10_percent() {
        let stats = StatTable {
            hp: 100,
            atk: 100,
            def: 100,
            spa: 100,
            spd: 100,
            spe: 100,
        };
        let stats = apply_nature_to_stats(stats, Nature::Adamant);
        assert_eq!(stats.hp, 100);
        assert_eq!(stats.atk, 110);
        assert_eq!(stats.def, 100);
        assert_eq!(stats.spa, 90);
        assert_eq!(stats.spd, 100);
        assert_eq!(stats.spe, 100);
    }

    #[test]
    fn nature_boosts_10_percent_truncated() {
        let stats = StatTable {
            hp: 45,
            atk: 45,
            def: 45,
            spa: 45,
            spd: 45,
            spe: 45,
        };
        let stats = apply_nature_to_stats(stats, Nature::Sassy);
        assert_eq!(stats.hp, 45);
        assert_eq!(stats.atk, 45);
        assert_eq!(stats.def, 45);
        assert_eq!(stats.spa, 45);
        assert_eq!(stats.spd, 49);
        assert_eq!(stats.spe, 40);
    }

    #[test]
    fn neutral_nature_does_not_modify_stats() {
        let stats = StatTable {
            hp: 100,
            atk: 100,
            def: 100,
            spa: 100,
            spd: 100,
            spe: 100,
        };
        let stats = apply_nature_to_stats(stats, Nature::Hardy);
        assert_eq!(stats.hp, 100);
        assert_eq!(stats.atk, 100);
        assert_eq!(stats.def, 100);
        assert_eq!(stats.spa, 100);
        assert_eq!(stats.spd, 100);
        assert_eq!(stats.spe, 100);
    }

    #[test]
    fn nature_boosts_avoids_overflow() {
        let stats = StatTable {
            hp: 596,
            atk: 729,
            def: 596,
            spa: 596,
            spd: 596,
            spe: 596,
        };
        let stats = apply_nature_to_stats(stats, Nature::Bold);
        assert_eq!(stats.hp, 596);
        // Note: this should be 655 if the boosted stat gets capped at 595 to avoid overflow.
        assert_eq!(stats.atk, 656);
        // Note: this should be 654 if the dropped stat gets capped at 728 to avoid overflow.
        assert_eq!(stats.def, 655);
        assert_eq!(stats.spa, 596);
        assert_eq!(stats.spd, 596);
        assert_eq!(stats.spe, 596);
    }

    #[derive(Deserialize)]
    struct StatCalculationTestCase {
        level: u8,
        nature: Nature,
        base_stats: StatTable,
        ivs: StatTable,
        evs: StatTable,
        expected: StatTable,
    }

    #[test]
    fn stat_calculation_test_cases() {
        let test_cases =
            read_test_cases::<StatCalculationTestCase>("stat_calculation_tests.json").unwrap();
        for (test_name, test_case) in test_cases {
            let got = calculate_mon_stats(
                &test_case.base_stats,
                &test_case.ivs,
                &test_case.evs,
                test_case.level,
                test_case.nature,
            );
            pretty_assertions::assert_eq!(got, test_case.expected, "{test_name} failed");
        }
    }

    #[derive(Deserialize)]
    struct HiddenPowerTypeCalculationTestCase {
        ivs: StatTable,
        expected: Type,
    }

    #[test]
    fn hidden_power_type_calculation_test_cases() {
        let test_cases = read_test_cases::<HiddenPowerTypeCalculationTestCase>(
            "hidden_power_type_calculation_tests.json",
        )
        .unwrap();
        for (test_name, test_case) in test_cases {
            let got = calculate_hidden_power_type(&test_case.ivs);
            assert_eq!(got, test_case.expected, "{test_name} failed");
        }
    }
}
