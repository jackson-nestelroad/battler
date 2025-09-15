use std::str::FromStr;

use ahash::{
    HashMap,
    HashSet,
};
use anyhow::Result;
use battler_data::{
    Id,
    Identifiable,
    Rule,
    SerializedRuleSet,
};
use zone_alloc::ElementRef;

use crate::{
    battle::BattleType,
    config::Clause,
    dex::Dex,
    error::{
        WrapOptionError,
        WrapResultError,
        general_error,
    },
};

/// Common numeric rules that are easier to parse out once than parse multiple times in team
/// validation.
///
/// New rules should likely not live here.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct NumericRules {
    pub players_per_side: Option<u32>,
    pub min_team_size: u32,
    pub max_team_size: u32,
    pub picked_team_size: Option<u32>,
    pub max_move_count: u32,
    pub min_level: u32,
    pub max_level: u32,
    pub default_level: u32,
    pub force_level: Option<u32>,
    pub adjust_level_down: Option<u32>,
    pub ev_limit: u32,

    /// The number of steps away from a Mon that is counted as being adjacent to it for
    /// adjacent-targeting attacks.
    ///
    /// By default, moves that target adjacent Mons can reach any Mon two steps away from it.
    /// However, some battles (such as Horde Battles) require one Mon to be able to fight five Mons
    /// at once. This requires an `adjacency_reach` of 3, since the Mons on the edges will be three
    /// steps away from the center.
    ///
    /// For visualization, the following battle:
    ///
    /// ```ignore
    /// 5  4  3  2  1
    ///       1
    /// ```
    ///
    /// maps to the following adjacency counts, relative to the single Mon on the bottom side
    ///
    /// ```ignore
    /// 3  2  1  2  3
    ///       0
    /// ```
    pub adjacency_reach: u8,

    /// The maximum level that will obey its player if it originates from a different trainer.
    pub obedience_cap: u8,
}

impl NumericRules {
    fn validate(&self, battle_type: &BattleType) -> Result<()> {
        let battle_type_min_team_size = battle_type.min_team_size() as u32;

        if self.max_team_size > 6 {
            return Err(general_error(format!(
                "Max Team Size = {} is unsupported (maximum is 6)",
                self.max_team_size,
            )));
        }
        if self.max_level > 255 {
            return Err(general_error(format!(
                "Max Level = {} is unsupported (maximum is 255)",
                self.max_level,
            )));
        }

        if self.min_team_size > 0 && self.min_team_size < battle_type_min_team_size {
            return Err(general_error(format!(
                "Min Team Size = {} is too small for {battle_type}",
                self.min_team_size,
            )));
        }
        if self
            .picked_team_size
            .is_some_and(|val| val < battle_type_min_team_size)
        {
            return Err(general_error(format!(
                "Picked Team Size = {} is too small for {battle_type}",
                self.picked_team_size.unwrap(),
            )));
        }
        if self.min_team_size > 0
            && self
                .picked_team_size
                .is_some_and(|val| val > self.min_team_size)
        {
            return Err(general_error(
                "Min Team Size is smaller than Picked Team Size",
            ));
        }
        if self.max_team_size < self.min_team_size {
            return Err(general_error("Max Team Size is smaller than Min Team Size"));
        }
        if self.max_level < self.min_level {
            return Err(general_error("Max Level is smaller than Min Level"));
        }
        if self.default_level < self.min_level {
            return Err(general_error("Default Level is smaller than Min Level"));
        }
        if self.default_level > self.max_level {
            return Err(general_error("Default Level is greater than Max Level"));
        }
        if self.force_level.is_some_and(|val| val < self.min_level) {
            return Err(general_error("Force Level is smaller than Min Level"));
        }
        if self.force_level.is_some_and(|val| val > self.max_level) {
            return Err(general_error("Force Level is greater than Max Level"));
        }
        if self
            .adjust_level_down
            .is_some_and(|val| val < self.min_level)
        {
            return Err(general_error("Adjust Level Down is smaller than Min Level"));
        }
        if self
            .adjust_level_down
            .is_some_and(|val| val > self.max_level)
        {
            return Err(general_error("Adjust Level Down is greater than Max Level"));
        }
        if self.ev_limit >= 1512 {
            return Err(general_error(format!(
                "EV Limit = {} has no effect because it is not less than 1512 (252 x 6)",
                self.ev_limit,
            )));
        }
        Ok(())
    }
    fn parse_from_ruleset(ruleset: &RuleSet, battle_type: &BattleType) -> Result<Self> {
        let mut rules = NumericRules::default();

        rules.players_per_side = ruleset.numeric_value(&Id::from_known("playersperside"));
        rules.min_team_size = ruleset
            .numeric_value(&Id::from_known("minteamsize"))
            .unwrap_or(0);
        rules.max_team_size = ruleset
            .numeric_value(&Id::from_known("maxteamsize"))
            .unwrap_or(6);
        rules.picked_team_size = ruleset.numeric_value(&Id::from_known("pickedteamsize"));
        rules.max_move_count = ruleset
            .numeric_value(&Id::from_known("maxmovecount"))
            .unwrap_or(4);
        rules.min_level = ruleset
            .numeric_value(&Id::from_known("minlevel"))
            .unwrap_or(1);
        rules.max_level = ruleset
            .numeric_value(&Id::from_known("maxlevel"))
            .unwrap_or(100);
        rules.default_level = ruleset
            .numeric_value(&Id::from_known("defaultlevel"))
            .unwrap_or(rules.max_level);
        rules.force_level = ruleset.numeric_value(&Id::from_known("forcelevel"));
        rules.adjust_level_down = ruleset.numeric_value(&Id::from_known("adjustleveldown"));
        rules.ev_limit = ruleset
            .numeric_value(&Id::from_known("evlimit"))
            .unwrap_or(510);
        rules.adjacency_reach = ruleset
            .numeric_value(&Id::from_known("adjacencyreach"))
            .unwrap_or(2);
        rules.obedience_cap = ruleset
            .numeric_value(&Id::from_known("obediencecap"))
            .unwrap_or(u8::MAX);

        if let Some("Auto") = ruleset.value(&Id::from_known("pickedteamsize")) {
            rules.picked_team_size = Some(battle_type.default_picked_team_size() as u32);
        }

        rules.validate(battle_type).map(|_| rules)
    }
}

/// The result of checking if a resource is allowed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResourceCheck {
    /// Resource is explicitly banned.
    Banned,
    /// Resource is explicitly allowed.
    Allowed,
    /// Resource is neither banned nor allowed.
    Unknown,
}

impl ResourceCheck {
    /// Performs the next resource check only if this resource check was inconclusive.
    pub fn and_then<F>(self, next: F) -> Self
    where
        F: Fn() -> Self,
    {
        match self {
            Self::Banned => Self::Banned,
            Self::Unknown => next(),
            Self::Allowed => Self::Allowed,
        }
    }
}

/// A set of rules for a battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleSet {
    original: SerializedRuleSet,
    bans: HashSet<Id>,
    unbans: HashSet<Id>,
    rules: HashMap<Id, String>,
    pub numeric_rules: NumericRules,
}

impl RuleSet {
    /// Constructs a new [`RuleSet`] from a [`SerializedRuleSet`].
    pub fn new(rules: SerializedRuleSet, battle_type: &BattleType, dex: &Dex) -> Result<Self> {
        let mut ruleset = Self {
            original: rules.clone(),
            bans: HashSet::default(),
            unbans: HashSet::default(),
            rules: HashMap::default(),
            numeric_rules: NumericRules::default(),
        };
        ruleset.store_flattened_ruleset(rules, dex);
        ruleset.resolve_numbers(battle_type)?;
        ruleset.validate_clauses(dex)?;

        Ok(ruleset)
    }

    /// Stores the given rules by flattening all compound rules.
    fn store_flattened_ruleset(&mut self, rules: SerializedRuleSet, dex: &Dex) {
        // First, record all repeals.
        let repeals = rules
            .iter()
            .filter_map(|rule| match rule {
                Rule::Repeal(id) => Some(id),
                _ => None,
            })
            .cloned()
            .collect::<HashSet<_>>();

        // Next, observe all rules and flatten out the ruleset.
        //
        // If a rule is found to be repealed, it is skipped over.
        let mut unstored_rules = rules;
        while !unstored_rules.is_empty() {
            let mut next_layer = SerializedRuleSet::default();
            for rule in unstored_rules {
                match rule {
                    Rule::Ban(id) => {
                        self.bans.insert(id);
                    }
                    Rule::Unban(id) => {
                        self.unbans.insert(id);
                    }
                    Rule::Value { name, mut value } => {
                        if repeals.contains(&name) {
                            continue;
                        }
                        if let Some(clause) = dex.clauses.get_by_id(&name).ok() {
                            if !clause.data.rules.is_empty() {
                                next_layer.extend(clause.data.rules.clone());
                                continue;
                            }
                            if value.is_empty() {
                                value = clause.data.default_value.clone();
                            }
                        }
                        self.rules.insert(name, value);
                    }
                    Rule::Repeal(_) => (),
                }
            }
            unstored_rules = next_layer;
        }
    }

    /// Resolves numeric rules that are used for battle validation.
    fn resolve_numbers(&mut self, battle_type: &BattleType) -> Result<()> {
        self.numeric_rules = NumericRules::parse_from_ruleset(self, battle_type)?;
        Ok(())
    }

    /// Validates all clauses in the ruleset.
    fn validate_clauses(&mut self, dex: &Dex) -> Result<()> {
        for clause in self.clauses(dex) {
            let value = self
                .value(clause.id())
                .wrap_expectation_with_format(format_args!(
                    "expected {} to be present in ruleset",
                    clause.data.name
                ))?;
            clause
                .validate_value(value)
                .wrap_error_with_format(format_args!("rule {} is invalid", clause.data.name))?;
        }
        Ok(())
    }

    /// Returns an iterator over all [`Clause`] objects associated with a rule present in the
    /// ruleset.
    ///
    /// A [`Clause`] wraps one or more rules to impact different parts of the battle.
    pub fn clauses<'s, 'd>(
        &'s self,
        dex: &'d Dex<'d>,
    ) -> impl Iterator<Item = ElementRef<'d, Clause>> + 's
    where
        'd: 's,
    {
        self.rules()
            .filter_map(move |rule| dex.clauses.get_by_id(rule).ok())
    }

    /// Checks if the given resource is allowed.
    pub fn check_resource(&self, id: &Id) -> ResourceCheck {
        let banned = self.bans.contains(id);
        let allowed = self.unbans.contains(id);
        match (banned, allowed) {
            (_, true) => ResourceCheck::Allowed,
            (true, false) => ResourceCheck::Banned,
            (false, false) => ResourceCheck::Unknown,
        }
    }

    /// Checks if the ruleset contains the given rule.
    pub fn has_rule(&self, id: &Id) -> bool {
        return self.rules.contains_key(id);
    }

    /// Returns all general rules.
    ///
    /// Does not include bans, allows, or repeals.
    pub fn rules(&self) -> impl Iterator<Item = &Id> {
        self.rules.keys()
    }

    /// Returns the value associated with a rule, if it exists.
    pub fn value(&self, id: &Id) -> Option<&str> {
        self.rules.get(id).map(|value| value.as_ref())
    }

    /// Returns the numeric value associated with a rule, if it exists.
    pub fn numeric_value<T>(&self, id: &Id) -> Option<T>
    where
        T: FromStr,
    {
        self.value(id)?.parse().ok()
    }

    /// Returns a serialized form of the ruleset.
    ///
    /// This method makes a clone of all rules, so it can be a bit expensive.
    ///
    /// It is not guaranteed that the result of this operation is the same as the original rules
    /// passed in. This is because compound rules are flattened, and repealed rules are removed
    /// completely. However, the rulesets will be functionally equivalent.
    pub fn serialized(&self) -> SerializedRuleSet {
        self.rules
            .iter()
            .map(|(name, value)| Rule::Value {
                name: name.clone(),
                value: value.clone(),
            })
            .chain(self.bans.iter().map(|id| Rule::Ban(id.clone())))
            .chain(self.unbans.iter().map(|id| Rule::Unban(id.clone())))
            .collect()
    }
}

#[cfg(test)]
mod rule_set_test {
    use ahash::{
        HashMap,
        HashSet,
    };
    use anyhow::Result;
    use battler_data::{
        Id,
        Rule,
        SerializedRuleSet,
    };
    use battler_test_utils::static_local_data_store;

    use crate::{
        battle::BattleType,
        config::{
            ResourceCheck,
            RuleSet,
        },
        dex::Dex,
    };

    fn construct_ruleset(serialized: &str, battle_type: &BattleType) -> Result<RuleSet> {
        let dex = Dex::new(static_local_data_store())?;
        let ruleset = serde_json::from_str::<SerializedRuleSet>(serialized).unwrap();
        RuleSet::new(ruleset, battle_type, &dex)
    }

    #[test]
    fn constructs_from_string() {
        let ruleset = construct_ruleset(
            r#"[
                "- Legendary",
                "+ Giratina (Origin)",
                "Max Level = 50",
                "OU"
            ]"#,
            &BattleType::Singles,
        )
        .unwrap();
        assert_eq!(ruleset.bans, HashSet::from_iter([Id::from("legendary")]),);
        assert_eq!(
            ruleset.unbans,
            HashSet::from_iter([Id::from("giratinaorigin")]),
        );
        assert_eq!(
            ruleset.rules,
            HashMap::from_iter([
                (Id::from("maxlevel"), "50".to_owned()),
                (Id::from("ou"), "".to_owned())
            ]),
        );
    }

    #[test]
    fn checks_if_resource_is_banned() {
        let ruleset = construct_ruleset(
            r#"[
                "- Bulbasaur",
                "- Charmander",
                "+ Squirtle",
                "+ Bulbasaur"
            ]"#,
            &BattleType::Singles,
        )
        .unwrap();
        assert_eq!(
            ruleset.check_resource(&Id::from("bulbasaur")),
            ResourceCheck::Allowed,
        );
        assert_eq!(
            ruleset.check_resource(&Id::from("charmander")),
            ResourceCheck::Banned
        );
        assert_eq!(
            ruleset.check_resource(&Id::from("squirtle")),
            ResourceCheck::Allowed
        );
        assert_eq!(
            ruleset.check_resource(&Id::from("pikachu")),
            ResourceCheck::Unknown
        );
    }

    #[test]
    fn checks_for_value_rule() {
        let ruleset = construct_ruleset(
            r#"[
                "- abcd",
                "+ defg",
                "hijk",
                "lmno = pqrs"
            ]"#,
            &BattleType::Singles,
        )
        .unwrap();
        assert!(!ruleset.has_rule(&Id::from("abcd")));
        assert!(!ruleset.has_rule(&Id::from("defg")));
        assert!(ruleset.has_rule(&Id::from("hijk")));
        assert!(ruleset.has_rule(&Id::from("lmno")));
    }

    #[test]
    fn gets_value_for_rule() {
        let ruleset = construct_ruleset(
            r#"[
                "- abcd",
                "+ defg",
                "hijk",
                "lmno = pqrs"
            ]"#,
            &BattleType::Singles,
        )
        .unwrap();
        assert_eq!(ruleset.value(&Id::from("abcd")), None);
        assert_eq!(ruleset.value(&Id::from("defg")), None);
        assert_eq!(ruleset.value(&Id::from("hijk")), Some(""));
        assert_eq!(ruleset.value(&Id::from("lmno")), Some("pqrs"));
    }

    #[test]
    fn gets_numeric_value_for_rule() {
        let ruleset = construct_ruleset(
            r#"[
                "Max Level = 50",
                "Min Level = 1",
                "Random Rule"
            ]"#,
            &BattleType::Singles,
        )
        .unwrap();
        assert_eq!(ruleset.numeric_value(&Id::from("maxlevel")), Some(50));
        assert_eq!(ruleset.numeric_value(&Id::from("minlevel")), Some(1));
        assert_eq!(ruleset.numeric_value::<u32>(&Id::from("randomrule")), None);
    }

    #[test]
    fn resolves_numbers() {
        let ruleset = construct_ruleset(
            r#"[
                "Players Per Side = 2",
                "Min Team Size = 3",
                "Max Team Size = 6",
                "Picked Team Size = 2",
                "Max Move Count = 10",
                "Min Level = 100",
                "Max Level = 255",
                "Default Level = 100",
                "Force Level = 100",
                "Adjust Level Down = 150",
                "EV Limit = 1500",
                "Adjacency Reach = 3",
                "Obedience Cap = 50"
            ]"#,
            &BattleType::Singles,
        )
        .unwrap();
        assert_eq!(ruleset.numeric_rules.players_per_side, Some(2));
        assert_eq!(ruleset.numeric_rules.min_team_size, 3);
        assert_eq!(ruleset.numeric_rules.max_team_size, 6);
        assert_eq!(ruleset.numeric_rules.picked_team_size, Some(2));
        assert_eq!(ruleset.numeric_rules.max_move_count, 10);
        assert_eq!(ruleset.numeric_rules.min_level, 100);
        assert_eq!(ruleset.numeric_rules.max_level, 255);
        assert_eq!(ruleset.numeric_rules.default_level, 100);
        assert_eq!(ruleset.numeric_rules.force_level, Some(100));
        assert_eq!(ruleset.numeric_rules.adjust_level_down, Some(150));
        assert_eq!(ruleset.numeric_rules.ev_limit, 1500);
        assert_eq!(ruleset.numeric_rules.adjacency_reach, 3);
        assert_eq!(ruleset.numeric_rules.obedience_cap, 50);
    }

    #[test]
    fn resolves_numbers_to_default_values() {
        let ruleset = construct_ruleset("[]", &BattleType::Doubles).unwrap();
        assert_eq!(ruleset.numeric_rules.players_per_side, None);
        assert_eq!(ruleset.numeric_rules.min_team_size, 0);
        assert_eq!(ruleset.numeric_rules.max_team_size, 6);
        assert_eq!(ruleset.numeric_rules.picked_team_size, None);
        assert_eq!(ruleset.numeric_rules.max_move_count, 4);
        assert_eq!(ruleset.numeric_rules.min_level, 1);
        assert_eq!(ruleset.numeric_rules.max_level, 100);
        assert_eq!(ruleset.numeric_rules.default_level, 100);
        assert_eq!(ruleset.numeric_rules.force_level, None);
        assert_eq!(ruleset.numeric_rules.adjust_level_down, None);
        assert_eq!(ruleset.numeric_rules.ev_limit, 510);
        assert_eq!(ruleset.numeric_rules.adjacency_reach, 2);
        assert_eq!(ruleset.numeric_rules.obedience_cap, u8::MAX);
    }

    #[test]
    fn auto_picked_team_size_singles() {
        let ruleset = construct_ruleset(
            r#"[
                "Picked Team Size = Auto"
            ]"#,
            &BattleType::Singles,
        )
        .unwrap();
        assert_eq!(ruleset.numeric_rules.picked_team_size, Some(3));
    }

    #[test]
    fn auto_picked_team_size_doubles() {
        let ruleset = construct_ruleset(
            r#"[
                "Picked Team Size = Auto"
            ]"#,
            &BattleType::Doubles,
        )
        .unwrap();
        assert_eq!(ruleset.numeric_rules.picked_team_size, Some(4));
    }

    #[test]
    fn auto_picked_team_size_multi() {
        let ruleset = construct_ruleset(
            r#"[
                "Picked Team Size = Auto"
            ]"#,
            &BattleType::Multi,
        )
        .unwrap();
        assert_eq!(ruleset.numeric_rules.picked_team_size, Some(3));
    }

    fn resolves_numbers_fails_with_error(input: &str, battle_type: BattleType, error: &str) {
        assert!(
            format!(
                "{:#}",
                construct_ruleset(input, &battle_type).err().unwrap()
            )
            .contains(error)
        );
    }

    #[test]
    fn validates_numbers() {
        resolves_numbers_fails_with_error(
            r#"["Max Team Size = 200"]"#,
            BattleType::Singles,
            "Max Team Size = 200 is unsupported",
        );
        resolves_numbers_fails_with_error(
            r#"["Max Level = 100000"]"#,
            BattleType::Singles,
            "Max Level = 100000 is unsupported",
        );
        resolves_numbers_fails_with_error(
            r#"["Min Team Size = 1"]"#,
            BattleType::Doubles,
            "Min Team Size = 1 is too small for Doubles",
        );
        resolves_numbers_fails_with_error(
            r#"["Picked Team Size = 1"]"#,
            BattleType::Doubles,
            "Picked Team Size = 1 is too small for Doubles",
        );
        resolves_numbers_fails_with_error(
            r#"["Min Team Size = 3", "Picked Team Size = 6"]"#,
            BattleType::Doubles,
            "Min Team Size is smaller than Picked Team Size",
        );
        resolves_numbers_fails_with_error(
            r#"["Max Team Size = 3", "Min Team Size = 6"]"#,
            BattleType::Doubles,
            "Max Team Size is smaller than Min Team Size",
        );
        resolves_numbers_fails_with_error(
            r#"["Max Level = 50", "Min Level = 60"]"#,
            BattleType::Doubles,
            "Max Level is smaller than Min Level",
        );
        resolves_numbers_fails_with_error(
            r#"["Default Level = 50", "Min Level = 60"]"#,
            BattleType::Doubles,
            "Default Level is smaller than Min Level",
        );
        resolves_numbers_fails_with_error(
            r#"["Default Level = 50", "Max Level = 40"]"#,
            BattleType::Doubles,
            "Default Level is greater than Max Level",
        );
        resolves_numbers_fails_with_error(
            r#"["Force Level = 50", "Min Level = 60"]"#,
            BattleType::Doubles,
            "Force Level is smaller than Min Level",
        );
        resolves_numbers_fails_with_error(
            r#"["Force Level = 50", "Max Level = 40"]"#,
            BattleType::Doubles,
            "Force Level is greater than Max Level",
        );
        resolves_numbers_fails_with_error(
            r#"["EV Limit = 10000"]"#,
            BattleType::Doubles,
            "EV Limit = 10000 has no effect because it is not less than 1512 (252 x 6)",
        );
    }

    #[test]
    fn serializes_rules() {
        let ruleset = construct_ruleset(
            r#"[
                "- abcd",
                "+ defg",
                "hijk",
                "lmno = pqrs"
            ]"#,
            &BattleType::Singles,
        )
        .unwrap();
        let serialized = ruleset.serialized();
        assert_eq!(
            serialized,
            SerializedRuleSet::from_iter([
                Rule::Ban(Id::from("abcd")),
                Rule::Unban(Id::from("defg")),
                Rule::Value {
                    name: Id::from("hijk"),
                    value: "".to_string(),
                },
                Rule::Value {
                    name: Id::from("lmno"),
                    value: "pqrs".to_string()
                }
            ])
        );
        let serialized = serde_json::to_string(&serialized).unwrap();
        let ruleset_clone = construct_ruleset(&serialized, &BattleType::Singles).unwrap();
        assert_eq!(ruleset, ruleset_clone);
    }

    #[test]
    fn enforces_required_rule_value() {
        assert!(
            format!(
                "{:#}",
                construct_ruleset(
                    r#"[
                "Adjust Level Down"
            ]"#,
                    &BattleType::Singles,
                )
                .err()
                .unwrap()
            )
            .contains("Adjust Level Down is invalid: missing value")
        );
    }

    #[test]
    fn enforces_rule_value_type() {
        assert!(
            format!(
                "{:#}",
                construct_ruleset(
                    r#"[
                "Adjust Level Down = abc"
            ]"#,
                    &BattleType::Singles,
                )
                .err()
                .unwrap()
            )
            .contains("rule Adjust Level Down is invalid")
        );
    }

    #[test]
    fn flattens_clauses() {
        let ruleset = construct_ruleset(
            r#"[
                "Standard",
                "Evasion Clause"
            ]"#,
            &BattleType::Singles,
        )
        .unwrap();
        let want = serde_json::from_str(
            r#"[
                "Team Preview",
                "Sleep Clause",
                "Species Clause",
                "Nickname Clause",
                "Endless Battle Clause",
                "- Ability Tag: Evasion Raising",
                "- Item Tag: Evasion Raising",
                "- Move Tag: Evasion Raising",
                "- Move Tag: OHKO"
            ]"#,
        )
        .unwrap();
        pretty_assertions::assert_eq!(ruleset.serialized(), want);
    }

    #[test]
    fn repeals_entire_clauses() {
        let ruleset = construct_ruleset(
            r#"[
                "Standard",
                "Evasion Clause",
                "! Standard"
            ]"#,
            &BattleType::Singles,
        )
        .unwrap();
        let want = serde_json::from_str(
            r#"[
                "- Ability Tag: Evasion Raising",
                "- Item Tag: Evasion Raising",
                "- Move Tag: Evasion Raising"
            ]"#,
        )
        .unwrap();
        pretty_assertions::assert_eq!(ruleset.serialized(), want);
    }
}
