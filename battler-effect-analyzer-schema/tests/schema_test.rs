use battler_data::{
    BoostTable,
    Fraction,
    Id,
    MoveCategory,
    MoveTarget,
    Type,
};
use battler_effect_analyzer_schema::*;

#[test]
fn polarity_helpers_work() {
    assert!(EffectPolarity::Beneficial.is_beneficial());
    assert!(!EffectPolarity::Beneficial.is_harmful());
    assert!(!EffectPolarity::Beneficial.is_neutral());
    assert_eq!(
        EffectPolarity::Beneficial.invert(),
        EffectPolarity::Harmful
    );

    assert!(EffectPolarity::Harmful.is_harmful());
    assert!(!EffectPolarity::Harmful.is_beneficial());
    assert!(!EffectPolarity::Harmful.is_neutral());
    assert_eq!(
        EffectPolarity::Harmful.invert(),
        EffectPolarity::Beneficial
    );

    assert!(EffectPolarity::Neutral.is_neutral());
    assert!(!EffectPolarity::Neutral.is_beneficial());
    assert!(!EffectPolarity::Neutral.is_harmful());
    assert_eq!(EffectPolarity::Neutral.invert(), EffectPolarity::Neutral);
}

#[test]
fn target_role_helpers_work() {
    assert!(TargetRole::User.is_friendly());
    assert!(!TargetRole::User.is_opposing());
    assert!(TargetRole::User.is_mon());
    assert!(!TargetRole::User.is_side());

    assert!(TargetRole::Ally.is_friendly());
    assert!(!TargetRole::Ally.is_opposing());
    assert!(TargetRole::Ally.is_mon());

    assert!(TargetRole::Foe.is_opposing());
    assert!(!TargetRole::Foe.is_friendly());
    assert!(TargetRole::Foe.is_mon());

    assert!(TargetRole::AllySide.is_friendly());
    assert!(TargetRole::AllySide.is_side());
    assert!(!TargetRole::AllySide.is_mon());

    assert!(TargetRole::FoeSide.is_opposing());
    assert!(TargetRole::FoeSide.is_side());
    assert!(!TargetRole::FoeSide.is_mon());

    assert!(!TargetRole::Field.is_friendly());
    assert!(!TargetRole::Field.is_opposing());
    assert!(!TargetRole::Field.is_mon());
    assert!(!TargetRole::Field.is_side());
}

#[test]
fn serde_roundtrip_semantic_actions() {
    let actions = vec![
        SemanticAction::Damage {
            category: MoveCategory::Physical,
            base_power: Some(80),
            recoil_percent: Some(Fraction::new(1, 3)),
            drain_percent: None,
        },
        SemanticAction::Damage {
            category: MoveCategory::Special,
            base_power: Some(75),
            recoil_percent: None,
            drain_percent: Some(Fraction::new(1, 2)),
        },
        SemanticAction::StatChange {
            boosts: BoostTable {
                atk: 2,
                def: -1,
                ..Default::default()
            },
        },
        SemanticAction::StatusInfliction {
            status: "par".to_owned(),
            chance: Fraction::new(1, 1),
        },
        SemanticAction::StatusCure {
            statuses: vec!["slp".to_owned(), "frz".to_owned()],
        },
        SemanticAction::ApplyCondition {
            condition_id: "taunt".to_owned(),
            is_volatile: true,
            max_stacks: 1,
        },
        SemanticAction::ApplySideCondition {
            condition_id: "spikes".to_owned(),
            max_stacks: 3,
        },
        SemanticAction::SetFieldCondition {
            condition_id: "trickroom".to_owned(),
        },
        SemanticAction::Heal {
            fraction: Fraction::new(1, 2),
        },
        SemanticAction::Revive {
            fraction: Fraction::new(1, 1),
        },
        SemanticAction::ForceSwitch { target: true },
        SemanticAction::Protection,
    ];

    for action in actions {
        let serialized = serde_json::to_string(&action).unwrap();
        let deserialized: SemanticAction = serde_json::from_str(&serialized).unwrap();
        pretty_assertions::assert_eq!(action, deserialized);
    }
}

#[test]
fn serde_roundtrip_modifiers_and_flags() {
    let modifier = DamageModifier::new(
        DamageModifierEvent::BasePower,
        Fraction::new(3, 2),
        DamageModifierCondition::BasePowerMax(60),
        "Technician boost",
    );
    let serialized = serde_json::to_string(&modifier).unwrap();
    let deserialized: DamageModifier = serde_json::from_str(&serialized).unwrap();
    pretty_assertions::assert_eq!(modifier, deserialized);

    let fixed_damage = FixedDamage::FractionTargetCurrentHp(Fraction::new(1, 2));
    let serialized = serde_json::to_string(&fixed_damage).unwrap();
    let deserialized: FixedDamage = serde_json::from_str(&serialized).unwrap();
    pretty_assertions::assert_eq!(fixed_damage, deserialized);

    let flags: hashbrown::HashSet<EffectFlag> =
        [EffectFlag::BreaksScreens, EffectFlag::IgnoresSubstitute]
            .into_iter()
            .collect();
    let serialized = serde_json::to_string(&flags).unwrap();
    let deserialized: hashbrown::HashSet<EffectFlag> = serde_json::from_str(&serialized).unwrap();
    pretty_assertions::assert_eq!(flags, deserialized);
}

#[test]
fn serde_roundtrip_manifests() {
    let effect_manifest = EffectManifest {
        id: Id::from("closecombat"),
        target_scope: MoveTarget::Normal,
        default_polarity: EffectPolarity::Harmful,
        actions: vec![
            (
                EffectPolarity::Harmful,
                SemanticAction::Damage {
                    category: MoveCategory::Physical,
                    base_power: Some(120),
                    recoil_percent: None,
                    drain_percent: None,
                },
            ),
            (
                EffectPolarity::Harmful,
                SemanticAction::StatChange {
                    boosts: BoostTable {
                        def: -1,
                        spd: -1,
                        ..Default::default()
                    },
                },
            ),
        ],
        damage_modifiers: vec![],
        fixed_damage: None,
        flags: [EffectFlag::IgnoresProtect].into_iter().collect(),
    };
    let serialized = serde_json::to_string(&effect_manifest).unwrap();
    let deserialized: EffectManifest = serde_json::from_str(&serialized).unwrap();
    pretty_assertions::assert_eq!(effect_manifest, deserialized);

    let condition_manifest = ConditionManifest::new(
        Id::from("spikes"),
        ConditionScope::SideCondition,
        EffectPolarity::Harmful,
        3,
        None,
    );
    let serialized = serde_json::to_string(&condition_manifest).unwrap();
    let deserialized: ConditionManifest = serde_json::from_str(&serialized).unwrap();
    pretty_assertions::assert_eq!(condition_manifest, deserialized);

    let ability_manifest = AbilityManifest {
        id: Id::from("voltabsorb"),
        type_immunities: vec![Type::Electric],
        absorption: vec![(Type::Electric, HealOrBoost::Heal(Fraction::new(1, 4)))],
        status_immunities: vec![],
        move_flag_immunities: vec![],
        contact_punishment: None,
        survival: None,
        damage_modifiers: vec![],
        flags: [AbilityFlag::SuppressesWeather].into_iter().collect(),
    };
    let serialized = serde_json::to_string(&ability_manifest).unwrap();
    let deserialized: AbilityManifest = serde_json::from_str(&serialized).unwrap();
    pretty_assertions::assert_eq!(ability_manifest, deserialized);

    let item_manifest = ItemManifest {
        id: Id::from("heavydutyboots"),
        type_immunities: vec![],
        survival: None,
        contact_punishment: None,
        move_lock: None,
        damage_modifiers: vec![],
        bag_item: None,
        flags: [ItemFlag::IgnoresHazards].into_iter().collect(),
    };
    let serialized = serde_json::to_string(&item_manifest).unwrap();
    let deserialized: ItemManifest = serde_json::from_str(&serialized).unwrap();
    pretty_assertions::assert_eq!(item_manifest, deserialized);

    let air_balloon = ItemManifest {
        id: Id::from("airballoon"),
        type_immunities: vec![Type::Ground],
        survival: None,
        contact_punishment: None,
        move_lock: None,
        damage_modifiers: vec![],
        bag_item: None,
        flags: [].into_iter().collect(),
    };
    let serialized = serde_json::to_string(&air_balloon).unwrap();
    let deserialized: ItemManifest = serde_json::from_str(&serialized).unwrap();
    pretty_assertions::assert_eq!(air_balloon, deserialized);
}
