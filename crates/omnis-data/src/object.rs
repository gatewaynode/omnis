//! `data/objects`: kinds of things that stand in a cell (M8.5 step 2). A kind says how it looks
//! (a shape primitive measured in eighths of a cell), whether it blocks movement and sight, the
//! states it can be in, and what the party can do with it: each interaction is a verb with
//! requirements and effects. Maps place kinds at an offset inside a cell (`placement.rs`).
//!
//! Size, material, weight, light, noise, smell, value and tags are stored and checked but do
//! nothing yet; capacity makes a container (`tasks/plans/m8.5-objects.md`).

mod check;

pub(crate) use check::check_objects;

use crate::error::DataError;
use crate::limits::{
    MAX_CAPACITY, MAX_CHECK_DC, MAX_EFFECTS, MAX_INTERACTIONS, MAX_OBJECT_HEIGHT, MAX_REQUIREMENTS,
    MAX_STATES, MAX_TAGS, string_fits,
};
use crate::terms::{Ability, School, Size, Skill};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// What the party does to an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum Verb {
    Use,
    Open,
    Close,
    Search,
    Examine,
    Take,
    Push,
    Pull,
    Break,
    Read,
    Drink,
    Light,
    Extinguish,
}

impl Verb {
    /// Every verb, in declaration order.
    pub const ALL: [Verb; 13] = [
        Verb::Use,
        Verb::Open,
        Verb::Close,
        Verb::Search,
        Verb::Examine,
        Verb::Take,
        Verb::Push,
        Verb::Pull,
        Verb::Break,
        Verb::Read,
        Verb::Drink,
        Verb::Light,
        Verb::Extinguish,
    ];
}

/// The shape primitive an object is drawn as (M8.5 step 2b draws them).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Shape {
    /// A box: a crate, a chest, a table.
    Box,
    /// An upright post: a pillar, a torch stand, a statue's plinth.
    Post,
    /// A low flat slab: an altar, a bench, a grave.
    Slab,
    /// An arch: a gate, a doorway standing alone.
    Arch,
    /// A board on a post: a sign, a plaque.
    Sign,
    /// A heap: rubble, a pile, a bush.
    Heap,
}

/// What an object is made of. Stored; nothing reads it yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum Material {
    Wood,
    Stone,
    Metal,
    Cloth,
    Glass,
    Bone,
    Plant,
    Earth,
    Paper,
    Liquid,
}

/// How an object is drawn and how much room it takes, in eighths of a cell. `width` runs
/// across the object's facing and `depth` along it, so a facing turns the footprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Look {
    /// The shape primitive.
    pub shape: Shape,
    /// Across the facing, 1..=8.
    pub width: u8,
    /// Along the facing, 1..=8.
    pub depth: u8,
    /// Above the ground, 1..=`MAX_OBJECT_HEIGHT` (a wall is 8).
    pub height: u8,
}

/// What a kind is, besides how it looks. Everything but blocking and capacity is stored for
/// later.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Attributes {
    /// The party cannot pass through its footprint.
    pub blocks_movement: bool,
    /// Sight does not pass through it, below its height.
    pub blocks_sight: bool,
    /// Size category.
    pub size: Option<Size>,
    /// What it is made of.
    pub material: Option<Material>,
    /// Weight in tenths of a pound.
    pub weight_tenths: u32,
    /// Item stacks it holds; above 0 makes it a container.
    pub capacity: u16,
    /// Light it gives.
    pub light: u8,
    /// Noise it makes.
    pub noise: u8,
    /// Smell it gives off.
    pub smell: u8,
    /// Value in copper pieces.
    pub value_cp: u32,
    /// Free-form tags.
    pub tags: BTreeSet<String>,
    /// A magical aura, which a reveal spell senses.
    pub aura: Option<School>,
}

/// One named state of a kind. Each field left out keeps the kind's own.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct StateDef {
    /// The look in this state.
    pub look: Option<Look>,
    /// Blocking movement in this state.
    pub blocks_movement: Option<bool>,
    /// Blocking sight in this state.
    pub blocks_sight: Option<bool>,
    /// A container's contents can be reached in this state.
    pub open: bool,
}

/// What must hold before an interaction succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Requirement {
    /// The best member who can act passes a skill check.
    Skill {
        /// The skill.
        skill: Skill,
        /// Difficulty, 1..=30.
        dc: u8,
    },
    /// The best member who can act passes an ability check.
    Ability {
        /// The ability.
        ability: Ability,
        /// Difficulty, 1..=30.
        dc: u8,
    },
    /// The party carries an item.
    Item {
        /// `pack:item:name`.
        id: String,
        /// One is used up on success.
        #[serde(default)]
        consume: bool,
    },
    /// A world flag is at least `min`.
    Flag {
        /// `pack:flag:name`.
        name: String,
        /// The lowest value that passes.
        min: i64,
    },
    /// A living member knows a spell of the school.
    School(School),
    /// A rules slot answers true; its inputs are `party_level` and `party_size`.
    Rule {
        /// The slot's name.
        slot: String,
    },
}

/// What an interaction does when it succeeds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Effect {
    /// Move the object to a named state.
    SetState(String),
    /// Set a world flag.
    SetFlag {
        /// `pack:flag:name`.
        name: String,
        /// The value.
        value: i64,
    },
    /// Show a text key.
    Message(String),
    /// Give the party gold, in copper pieces.
    GiveGold(u32),
    /// Give the party an item.
    GiveItem {
        /// `pack:item:name`.
        id: String,
        /// How many, at least 1.
        count: u16,
    },
    /// Take the object away for good.
    Remove,
}

/// One thing the party can do with a kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Interaction {
    /// The verb.
    pub verb: Verb,
    /// Text key shown instead of the verb's name.
    #[serde(default)]
    pub label: Option<String>,
    /// Offered only in this state.
    #[serde(default)]
    pub when_state: Option<String>,
    /// All must hold.
    #[serde(default)]
    pub requires: Vec<Requirement>,
    /// Done in order on success.
    pub effects: Vec<Effect>,
    /// Text key shown when a requirement fails.
    #[serde(default)]
    pub failure: Option<String>,
    /// Succeeds once per placement.
    #[serde(default)]
    pub once: bool,
    /// Minutes it takes; the interact cost when left out.
    #[serde(default)]
    pub minutes: Option<u32>,
}

/// A kind of object, one file under `data/objects`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectKind {
    /// Schema version.
    pub schema: u32,
    /// `pack:object:name`.
    pub id: String,
    /// Text key of the display name.
    pub name: String,
    /// How it looks and how much room it takes.
    pub look: Look,
    /// The state a placement starts in; empty when the kind has no states.
    #[serde(default)]
    pub initial_state: String,
    /// Named states.
    #[serde(default)]
    pub states: BTreeMap<String, StateDef>,
    /// Blocking and the stored attributes.
    #[serde(default)]
    pub attributes: Attributes,
    /// What the party can do with it.
    #[serde(default)]
    pub interactions: Vec<Interaction>,
}

impl ObjectKind {
    /// Shape checks that need no other file.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        let mut error = |message: String| errors.push(DataError::new(file, message));
        check_look(&self.look, "look", &mut error);
        self.check_states(&mut error);
        self.check_attributes(&mut error);
        if self.interactions.len() > MAX_INTERACTIONS {
            error(format!("at most {MAX_INTERACTIONS} interactions"));
            return;
        }
        for (i, interaction) in self.interactions.iter().enumerate() {
            self.check_interaction(i, interaction, &mut error);
        }
    }

    fn check_states(&self, error: &mut impl FnMut(String)) {
        if self.states.len() > MAX_STATES {
            error(format!("at most {MAX_STATES} states"));
        }
        for (name, state) in &self.states {
            if name.is_empty() || !string_fits(name) {
                error("a state needs a name within the size limit".to_owned());
            }
            if let Some(look) = &state.look {
                check_look(look, &format!("state '{name}' look"), error);
            }
        }
        if self.states.is_empty() {
            if !self.initial_state.is_empty() {
                error(format!(
                    "initial_state '{}' needs states",
                    self.initial_state
                ));
            }
        } else if !self.states.contains_key(&self.initial_state) {
            error(format!(
                "initial_state '{}' is not one of the states",
                self.initial_state
            ));
        }
    }

    fn check_attributes(&self, error: &mut impl FnMut(String)) {
        let a = &self.attributes;
        if a.capacity > MAX_CAPACITY {
            error(format!("capacity must be 0..={MAX_CAPACITY}"));
        }
        if a.tags.len() > MAX_TAGS {
            error(format!("at most {MAX_TAGS} tags"));
        }
        if a.tags.iter().any(|t| t.is_empty() || !string_fits(t)) {
            error("a tag must be within the size limit and not empty".to_owned());
        }
    }

    fn check_interaction(&self, i: usize, it: &Interaction, error: &mut impl FnMut(String)) {
        if let Some(state) = &it.when_state
            && !self.states.contains_key(state)
        {
            error(format!(
                "interaction {i} is offered in state '{state}', which the kind does not have"
            ));
        }
        if it.effects.is_empty() {
            error(format!("interaction {i} needs at least one effect"));
        }
        if it.requires.len() > MAX_REQUIREMENTS {
            error(format!(
                "interaction {i} has more than {MAX_REQUIREMENTS} requirements"
            ));
        }
        if it.effects.len() > MAX_EFFECTS {
            error(format!(
                "interaction {i} has more than {MAX_EFFECTS} effects"
            ));
        }
        for requirement in &it.requires {
            if let Requirement::Skill { dc, .. } | Requirement::Ability { dc, .. } = requirement
                && !(1..=MAX_CHECK_DC).contains(dc)
            {
                error(format!(
                    "interaction {i} check dc must be 1..={MAX_CHECK_DC}"
                ));
            }
        }
        for effect in &it.effects {
            match effect {
                Effect::SetState(state) if !self.states.contains_key(state) => error(format!(
                    "interaction {i} sets state '{state}', which the kind does not have"
                )),
                Effect::GiveItem { count: 0, .. } => {
                    error(format!("interaction {i} gives an item count of 0"));
                }
                _ => {}
            }
        }
    }
}

fn check_look(look: &Look, what: &str, error: &mut impl FnMut(String)) {
    if !(1..=8).contains(&look.width) || !(1..=8).contains(&look.depth) {
        error(format!("{what} width and depth must be 1..=8 eighths"));
    }
    if !(1..=MAX_OBJECT_HEIGHT).contains(&look.height) {
        error(format!(
            "{what} height must be 1..={MAX_OBJECT_HEIGHT} eighths"
        ));
    }
}
