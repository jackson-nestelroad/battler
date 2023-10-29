use std::ops::Div;

use crate::{
    mons::{
        Nature,
        Stat,
        StatTable,
    },
    teams::MonData,
};

/// Calculates a Mon's actual stats from a base stat table and [`MonData`].
pub fn calculate_mon_stats(base_stats: &StatTable, mon: &MonData) -> StatTable {
    let mut stats = StatTable::default();
    for (stat, value) in base_stats {
        let value = 2 * value + mon.ivs.get(stat) + mon.evs.get(stat) / 4;
        let value = value * (mon.level as u16) / 100;
        let value = if stat == Stat::HP {
            value + (mon.level as u16) + 10
        } else {
            value + 5
        };
        stats.set(stat, value);
    }
    apply_nature_to_stats(stats, mon.nature)
}

/// Applies the given nature to the stat table, returning the new stat table.
///
/// This calculation prevents overflow.
pub fn apply_nature_to_stats(mut stats: StatTable, nature: Nature) -> StatTable {
    let boosts = nature.boosts();
    let boosted_stat = stats.get(boosts);
    let boosted_stat = boosted_stat + (boosted_stat * 10).div(100);
    stats.set(boosts, boosted_stat);

    let drops = nature.drops();
    let dropped_stat = stats.get(drops);
    let dropped_stat = dropped_stat - num::Integer::div_ceil(&(dropped_stat * 10), &100);
    stats.set(drops, dropped_stat);

    stats
}

#[cfg(test)]
mod calclulations_tests {
    use serde::Deserialize;

    use crate::{
        battle::{
            apply_nature_to_stats,
            calculate_mon_stats,
        },
        common::read_test_cases,
        mons::{
            Nature,
            StatTable,
        },
        teams::MonData,
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

    impl StatCalculationTestCase {
        fn create_mon_data(&self) -> MonData {
            let mut mon_data: MonData = serde_json::from_str(
                r#"{
                    "name": "Bulba Fett",
                    "species": "Bulbasaur",
                    "ability": "Overgrow",
                    "moves": [],
                    "nature": "Adamant",
                    "gender": "M",
                    "ball": "Normal",
                    "level": 50
                }"#,
            )
            .unwrap();
            mon_data.level = self.level;
            mon_data.nature = self.nature;
            mon_data.ivs = self.ivs.clone();
            mon_data.evs = self.evs.clone();
            mon_data
        }
    }

    #[test]
    fn stat_calculation_test_cases() {
        let test_cases =
            read_test_cases::<StatCalculationTestCase>("stat_calculation_tests.json").unwrap();
        for (test_name, test_case) in test_cases {
            let got = calculate_mon_stats(&test_case.base_stats, &test_case.create_mon_data());
            pretty_assertions::assert_eq!(got, test_case.expected, "{test_name} failed");
        }
    }
}
