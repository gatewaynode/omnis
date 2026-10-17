//! Omnis data: schema structs for every content type, the pack manifest and loader, the ID
//! registry, validation of packs as untrusted input, schema migrations, and RON read and write.
//!
//! This is the only simulation crate permitted to read or write files (ARCHITECTURE.md §3).
//! Everything else receives loaded data. Integers only; ordered collections only.
#![forbid(unsafe_code)]
#![deny(clippy::float_arithmetic)]
#![warn(missing_docs)]

pub mod action;
pub mod character;
pub mod condition;
mod content;
pub mod encounter;
pub mod error;
pub mod footprint;
pub mod item;
pub mod levels;
pub mod limits;
pub mod loader;
pub mod loot;
pub mod lore;
pub mod manifest;
pub mod map;
pub mod map_data;
pub mod monster;
pub mod object;
pub mod placement;
pub mod region;
pub mod registry;
pub mod rest_event;
pub mod ron_io;
pub mod rules;
pub mod sense;
pub mod service;
pub mod spell;
pub mod stairs;
pub mod terms;
pub mod text;
pub mod tileset;

pub use action::{Cost, FeatureEffect, Recharge, Uses};
pub use character::{Background, Casting, Class, ClassFeature, Effect, Feature, Race, SkillChoice};
pub use condition::Condition;
pub use content::DEFAULT_COMPONENT_THRESHOLD;
pub use encounter::{
    Disposition, FixedEncounter, RandomEncounters, RandomEntry, ResolvedEncounter, ResolvedEntry,
    ResolvedRandom, Stand,
};
pub use error::{DataError, LoadReport};
pub use item::{EquipSlot, Item, ItemKind, UseEffect};
pub use levels::{LevelData, LevelDef, NO_GROUND};
pub use loader::{Data, PackFingerprint, load_packs};
pub use loot::{LootEntry, LootTable};
pub use lore::{CellLore, Lore};
pub use manifest::{Attribution, PackManifest};
pub use map::{Cell, Join, MapDef, MapKind, Portal, Terrain, WallSurfaces};
pub use map_data::{MapData, ResolvedPortal};
pub use monster::{Attack, Monster, MonsterCasting};
pub use object::{
    Attributes, Effect as ObjectEffect, Interaction, Look, Material, ObjectKind, Requirement,
    Shape, StateDef, Verb,
};
pub use omnis_expr;
pub use placement::{GroundDef, Pile, Placement, ResolvedPlacement};
pub use region::{Region, RegionDef, RegionKind};
pub use registry::Registry;
pub use rest_event::{ResolvedRestEvent, RestEventDef, RestKind};
pub use rules::{RulesFile, SlotDef};
pub use sense::{Fidelity, Geometry, Persistence, SenseSource};
pub use service::{ResolvedSite, RumorDef, ServiceDef, ServiceKind, Site};
pub use spell::{BuffOn, Reach, Spell, SpellEffect, Utility};
pub use terms::{
    Ability, Alignment, ArmorKind, DamageType, SaveAgainst, School, Size, Skill, WeaponKind,
};
pub use text::TextFile;
pub use tileset::{Slot, SlotKind, Surface, Tileset};

/// The schema version every data type is written at today. Older files are migrated on load.
pub const SCHEMA: u32 = 1;
