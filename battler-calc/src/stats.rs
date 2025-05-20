use anyhow::{
    Error,
    Result,
};
use battler_data::{
    DataStoreByName,
    Nature,
    Stat,
    StatTable,
};

use crate::common::Range;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Stats<T> {
    pub hp: T,
    pub atk: T,
    pub def: T,
    pub spa: T,
    pub spd: T,
    pub spe: T,
}

pub fn default_ev_ranges() -> Stats<Range<u64>> {
    Stats {
        hp: Range::new(0, 255),
        atk: Range::new(0, 255),
        def: Range::new(0, 255),
        spa: Range::new(0, 255),
        spd: Range::new(0, 255),
        spe: Range::new(0, 255),
    }
}

pub fn max_evs() -> Stats<Range<u64>> {
    Stats {
        hp: 255.into(),
        atk: 255.into(),
        def: 255.into(),
        spa: 255.into(),
        spd: 255.into(),
        spe: 255.into(),
    }
}

pub fn default_iv_ranges() -> Stats<Range<u64>> {
    Stats {
        hp: Range::new(0, 31),
        atk: Range::new(0, 31),
        def: Range::new(0, 31),
        spa: Range::new(0, 31),
        spd: Range::new(0, 31),
        spe: Range::new(0, 31),
    }
}

pub fn max_ivs() -> Stats<Range<u64>> {
    Stats {
        hp: 31.into(),
        atk: 31.into(),
        def: 31.into(),
        spa: 31.into(),
        spd: 31.into(),
        spe: 31.into(),
    }
}

impl<T> Stats<T>
where
    T: Copy,
{
    /// Returns the value for the given stat.
    pub fn get(&self, stat: Stat) -> T {
        match stat {
            Stat::HP => self.hp,
            Stat::Atk => self.atk,
            Stat::Def => self.def,
            Stat::SpAtk => self.spa,
            Stat::SpDef => self.spd,
            Stat::Spe => self.spe,
        }
    }

    /// Sets the given stat value.
    pub fn set(&mut self, stat: Stat, value: T) {
        let stat = match stat {
            Stat::HP => &mut self.hp,
            Stat::Atk => &mut self.atk,
            Stat::Def => &mut self.def,
            Stat::SpAtk => &mut self.spa,
            Stat::SpDef => &mut self.spd,
            Stat::Spe => &mut self.spe,
        };
        *stat = value;
    }
}

pub fn calculate_stats(
    data: &dyn DataStoreByName,
    species: &str,
    level: u64,
    nature: Option<Nature>,
    ivs: Option<&Stats<Range<u64>>>,
    evs: Option<&Stats<Range<u64>>>,
) -> Result<Stats<Range<u64>>> {
    let species = data
        .get_species_by_name(species)?
        .ok_or_else(|| Error::msg("species not found"))?;
    let mut stats = calculate_stats_by_base_stats(
        &species.base_stats,
        level,
        nature,
        ivs.unwrap_or(&default_iv_ranges()),
        evs.unwrap_or(&default_ev_ranges()),
    );

    if let Some(max_hp) = species.max_hp {
        stats.hp = Range::from(max_hp as u64);
    }

    Ok(stats)
}

fn calculate_stats_by_base_stats(
    base_stats: &StatTable,
    level: u64,
    nature: Option<Nature>,
    ivs: &Stats<Range<u64>>,
    evs: &Stats<Range<u64>>,
) -> Stats<Range<u64>> {
    let mut stats = Stats::default();
    for (stat, value) in base_stats {
        let value = Range::from(value as u64);
        let value = value * 2u64;
        let value = value + ivs.get(stat);
        let value = value + evs.get(stat) / 4u64;
        let value = value * level / 100u64;
        let value = if stat == Stat::HP {
            value + level + 10u64
        } else {
            value + 5u64
        };
        stats.set(stat, value);
    }

    if let Some(nature) = nature {
        let boosts = nature.boosts();
        let drops = nature.drops();
        if boosts != drops {
            let boosted = stats.get(boosts);
            let boosted = boosted + (boosted * 10u64) / 100u64;
            stats.set(boosts, boosted);

            let dropped = stats.get(drops);
            let dropped = dropped - (dropped * 10u64).div_ceil(100u64);
            stats.set(drops, dropped);
        }
    } else {
        for stat in [Stat::Atk, Stat::Def, Stat::SpAtk, Stat::SpDef, Stat::Spe] {
            let value = stats.get(stat);
            let boosted = value.b();
            let boosted = boosted + (boosted * 10) / 100;
            let dropped = value.a();
            let dropped = dropped - (dropped * 10).div_ceil(100);
            stats.set(stat, Range::new(dropped, boosted));
        }
    }

    stats
}

#[cfg(test)]
mod stats_test {
    use battler_data::{
        LocalDataStore,
        Nature,
    };

    use crate::{
        common::Range,
        stats::{
            Stats,
            calculate_stats,
        },
    };

    #[test]
    fn calculates_stat_ranges() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        assert_matches::assert_matches!(calculate_stats(&data, "Garchomp", 78, None, None, None), Ok(stats) => {
            assert_eq!(stats, Stats {
                hp: Range::new(256, 329),
                atk: Range::new(186, 309),
                def: Range::new(137, 248),
                spa: Range::new(116, 223),
                spd: Range::new(123, 231),
                spe: Range::new(147, 260),
            });
        });

        assert_matches::assert_matches!(calculate_stats(&data, "Blissey", 100, None, None, None), Ok(stats) => {
            assert_eq!(stats, Stats {
                hp: Range::new(620, 714),
                atk: Range::new(22, 130),
                def: Range::new(22, 130),
                spa: Range::new(139, 273),
                spd: Range::new(247, 405),
                spe: Range::new(103, 229),
            });
        });

        assert_matches::assert_matches!(calculate_stats(&data, "Blissey", 100, None, Some(&Stats::default()), None), Ok(stats) => {
            assert_eq!(stats, Stats {
                hp: Range::new(620, 683),
                atk: Range::new(22, 96),
                def: Range::new(22, 96),
                spa: Range::new(139, 239),
                spd: Range::new(247, 371),
                spe: Range::new(103, 195),
            });
        });
    }

    #[test]
    fn calculates_stats_with_all_information() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();

        assert_matches::assert_matches!(
            calculate_stats(
                &data,
                "Garchomp",
                78,
                Some(Nature::Adamant),
                Some(&Stats {
                    hp: Range::from(24),
                    atk: Range::from(12),
                    def: Range::from(30),
                    spa: Range::from(16),
                    spd: Range::from(23),
                    spe: Range::from(5),
                }),
                Some(&Stats {
                    hp: Range::from(74),
                    atk: Range::from(190),
                    def: Range::from(91),
                    spa: Range::from(48),
                    spd: Range::from(84),
                    spe: Range::from(23),
                }),
            ),
            Ok(stats) => {
                assert_eq!(stats, Stats {
                    hp: Range::from(289),
                    atk: Range::from(278),
                    def: Range::from(193),
                    spa: Range::from(135),
                    spd: Range::from(171),
                    spe: Range::from(171),
                });
            }
        );

        assert_matches::assert_matches!(
            calculate_stats(
                &data,
                "Blissey",
                100,
                Some(Nature::Bold),
                Some(&Stats {
                    hp: Range::from(31),
                    atk: Range::from(31),
                    def: Range::from(31),
                    spa: Range::from(31),
                    spd: Range::from(31),
                    spe: Range::from(31),
                }),
                Some(&Stats {
                    hp: Range::from(252),
                    atk: Range::from(0),
                    def: Range::from(252),
                    spa: Range::from(0),
                    spd: Range::from(4),
                    spe: Range::from(0),
                }),
            ),
            Ok(stats) => {
                assert_eq!(stats, Stats {
                    hp: Range::from(714),
                    atk: Range::from(50),
                    def: Range::from(130),
                    spa: Range::from(186),
                    spd: Range::from(307),
                    spe: Range::from(146),
                });
            }
        );
    }
}
