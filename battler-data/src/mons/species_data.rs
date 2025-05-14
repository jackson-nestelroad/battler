use ahash::{
    HashMap,
    HashSet,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    Color,
    EventData,
    EvolutionData,
    LearnSet,
    LevelingRate,
    SpeciesFlag,
    StatTable,
    Type,
};

fn default_as_true() -> bool {
    true
}

/// Data about a particular species.
///
/// Species data is common to all Mons of a given species. Data about a specific Mon (such as its
/// nature, stats, or battle-specific conditions) does not belong here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeciesData {
    /// The name of the species plus any forme name.
    ///
    /// Should be equal to [base_species][`SpeciesData::base_species`] +
    /// [forme][`SpeciesData::forme`].
    ///
    /// The species name should be unique across all species and formes.
    pub name: String,
    /// The base species name, which excludes any forme name.
    pub base_species: String,
    /// The forme name, if it exists.
    pub forme: Option<String>,
    /// The species class, which displays on the Dex page.
    pub class: String,
    /// The primary color of the species.
    pub color: Color,
    /// The primary type of the species.
    pub primary_type: Type,
    /// The secondary type of the species, if it exists.
    pub secondary_type: Option<Type>,
    /// Abilities.
    pub abilities: Vec<String>,
    /// Hidden ability, if it exists.
    pub hidden_ability: Option<String>,
    /// Gender ratio.
    ///
    /// Gender ratio is represented as one byte (a value between 0 and 255). There are three
    /// special values:
    /// - 0 = male only
    /// - 254 = female only
    /// - 255 = gender unknown
    ///
    /// Otherwise, the gender ratio is compared to a random number between 1 and 252 (inclusive).
    /// If the random number is less than `gender_ratio`, the Mon will be female.
    pub gender_ratio: u8,
    /// Catch rate.
    ///
    /// Catch rate is represented as one byte (a value between 0 and 255).
    pub catch_rate: u8,
    /// Can the species be hatched from an egg?
    #[serde(default = "default_as_true")]
    pub can_hatch: bool,
    /// Egg groups the species belongs to, which indicates which species can be bred together.
    pub egg_groups: HashSet<String>,
    /// The number of egg cycles reqired to hatch an egg of this species.
    ///
    /// One egg cycle is equal to 255 steps.
    pub hatch_time: u8,
    /// Height in meters (m).
    pub height: u32,
    /// Weight in kilograms (kg).
    pub weight: u32,
    /// Base experience yield when defeating this species.
    pub base_exp_yield: u16,
    /// Leveling rate of this species, which determines how much experience is required for
    /// leveling up.
    pub leveling_rate: LevelingRate,
    /// EV (effort value) yield when defating this species in battle.
    pub ev_yield: StatTable,
    /// Base friendship.
    ///
    /// Base friendship is represented as one byte (a value between 0 and 255).
    pub base_friendship: u8,
    /// Maximum HP override.
    ///
    /// This is used for Shedinja, which always has a maximum HP of 1.
    pub max_hp: Option<u16>,
    /// Base stats.
    pub base_stats: StatTable,
    /// Learnset, which contains all legal moves for this species.
    ///
    /// The learnset also defines how moves are learned by the species
    /// ([`crate::mons::MoveSource`]).
    #[serde(default)]
    pub learnset: LearnSet,

    /// Pre-evolution, if it exists.
    pub prevo: Option<String>,
    /// Evolutions.
    #[serde(default)]
    pub evos: Vec<String>,
    /// Evolution data, which gives information on how the species' pre-evolution evolves into this
    /// species.
    pub evolution_data: Option<EvolutionData>,

    /// The name of the base forme of this species, if it exists.
    ///
    /// For example, Giratina's base forme is "Altered".
    pub base_forme: Option<String>,
    /// Cosmetic formes, which have no impact on species data.
    #[serde(default)]
    pub cosmetic_formes: HashSet<String>,
    /// Is this forme available only in battles?
    #[serde(default)]
    pub battle_only_forme: bool,
    /// Items required for transforming into this forme.
    #[serde(default)]
    pub required_items: HashSet<String>,
    /// The species and forme name this forme transforms from.
    pub changes_from: Option<String>,

    /// Tags.
    #[serde(default)]
    pub tags: HashSet<SpeciesFlag>,
    /// Event giveaways for this species.
    #[serde(default)]
    pub events: HashMap<String, EventData>,
}

impl SpeciesData {
    /// The display name of the species with the forme name.
    pub fn display_name(&self) -> String {
        match self.forme.clone().or(self.base_forme.clone()) {
            None => self.name.to_owned(),
            Some(forme) => format!("{} ({})", self.base_species, forme),
        }
    }

    /// Utility method for returning the species' two types.
    pub fn types(&self) -> (Type, Option<Type>) {
        (self.primary_type, self.secondary_type)
    }

    /// The base state total (BST) of the species.
    pub fn bst(&self) -> u32 {
        self.base_stats.sum()
    }

    /// Is the species male only?
    pub fn male_only(&self) -> bool {
        self.gender_ratio == 0
    }

    /// Is the species female only?
    pub fn female_only(&self) -> bool {
        self.gender_ratio == 254
    }

    /// Is the species genderless?
    pub fn unknown_gender(&self) -> bool {
        self.gender_ratio == 255
    }

    // Is the species not-fully evolved (has an evolution)?
    pub fn nfe(&self) -> bool {
        !self.evos.is_empty()
    }

    /// Creates cosmetic forme data by moving and modifying this instance of [`SpeciesData`].
    pub fn create_cosmetic_forme_data(mut self, forme: String) -> Self {
        self.name = format!("{}-{forme}", self.base_species);
        self.forme = Some(forme);
        self.base_forme = None;
        self.cosmetic_formes.clear();
        self
    }
}
