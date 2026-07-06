#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod discovery;
mod log;
mod state;
mod state_selectors;
pub mod ui;

pub use discovery::*;
pub use log::*;
pub use state::*;
pub use state_selectors::*;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

#[cfg(test)]
#[cfg(feature = "typescript")]
mod export_ts_bindings {
    use ts_rs::TS;

    use crate::{
        BattlePhase,
        BattleState,
        ConditionData,
        Field,
        Mon,
        MonBattleAppearance,
        MonBattleAppearanceReference,
        MonBattleAppearanceWithRecovery,
        MonPhysicalAppearance,
        MonVolatileData,
        Player,
        Side,
        ui,
    };

    #[test]
    fn export_types() {
        BattleState::export().unwrap();
        BattlePhase::export().unwrap();
        ConditionData::export().unwrap();
        MonVolatileData::export().unwrap();
        MonPhysicalAppearance::export().unwrap();
        MonBattleAppearance::export().unwrap();
        MonBattleAppearanceWithRecovery::export().unwrap();
        Mon::export().unwrap();
        MonBattleAppearanceReference::export().unwrap();
        Player::export().unwrap();
        Side::export().unwrap();
        Field::export().unwrap();
        ui::FieldPosition::export().unwrap();
        ui::MonReference::export().unwrap();
        ui::Mon::export().unwrap();
        ui::MoveTarget::export().unwrap();
        ui::Effect::export().unwrap();
        ui::EffectData::export().unwrap();
        ui::UiLogEntry::export().unwrap();
    }
}
