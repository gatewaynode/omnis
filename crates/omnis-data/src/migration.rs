//! Pack migrations (M8.5 7h6): a pack's `migrations/NNNN.ron`, each taking saves made on an
//! earlier version of the pack to the version it names, as a database's schema migrations do.
//! They are data, checked with limits, never code: a pack may come from anyone. A file names
//! the version it leads `to`; versions between files need none (owner, 2026-10-19), so a save
//! made at `v` is offered every file with `v < to <= the pack's version`, in file order.
//!
//! The first three steps act on names (a definition renamed, replaced or removed), the last
//! three on places in the maps (a cell moved, a fixed encounter or an object renamed). A step
//! names only its own pack's definitions and places: a pack cannot rewrite another's.

use crate::Data;
use crate::error::DataError;
use crate::limits::string_fits;
use crate::manifest::is_content_id;
use crate::registry::Registry;
use omnis_core::MapId;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Migrations a pack may keep.
pub const MAX_MIGRATIONS: usize = 1024;
/// Steps one migration may take.
pub const MAX_STEPS: usize = 4096;

/// One `migrations/NNNN.ron`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Migration {
    /// Schema version of this file.
    pub schema: u32,
    /// The version of its pack it leads to.
    pub to: String,
    /// What changed, in order.
    pub steps: Vec<Step>,
}

/// What kind of definition a name step names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A map.
    Map,
    /// A flag.
    Flag,
    /// A region.
    Region,
    /// An item.
    Item,
    /// A spell.
    Spell,
    /// A monster.
    Monster,
    /// A condition.
    Condition,
    /// A race.
    Race,
    /// A class.
    Class,
    /// A background.
    Background,
    /// A service.
    Service,
}

impl Kind {
    /// The kind as an id writes it (`pack:item:name`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Map => "map",
            Kind::Flag => "flag",
            Kind::Region => "region",
            Kind::Item => "item",
            Kind::Spell => "spell",
            Kind::Monster => "monster",
            Kind::Condition => "condition",
            Kind::Race => "race",
            Kind::Class => "class",
            Kind::Background => "background",
            Kind::Service => "service",
        }
    }

    /// Whether a save can do without it: a race, class, background or region is what a
    /// character or a place is, so it can be renamed or replaced, never removed.
    #[must_use]
    pub const fn removable(self) -> bool {
        !matches!(
            self,
            Kind::Race | Kind::Class | Kind::Background | Kind::Region
        )
    }

    /// Every kind, in order.
    pub const ALL: [Kind; 11] = [
        Kind::Map,
        Kind::Flag,
        Kind::Region,
        Kind::Item,
        Kind::Spell,
        Kind::Monster,
        Kind::Condition,
        Kind::Race,
        Kind::Class,
        Kind::Background,
        Kind::Service,
    ];

    /// Every name of this kind the registry numbers, in number order.
    #[must_use]
    pub fn names(self, registry: &Registry) -> Vec<&str> {
        match self {
            Kind::Map => registry.maps.names().collect(),
            Kind::Flag => registry.flags.names().collect(),
            Kind::Region => registry.regions.names().collect(),
            Kind::Item => registry.items.names().collect(),
            Kind::Spell => registry.spells.names().collect(),
            Kind::Monster => registry.monsters.names().collect(),
            Kind::Condition => registry.conditions.names().collect(),
            Kind::Race => registry.races.names().collect(),
            Kind::Class => registry.classes.names().collect(),
            Kind::Background => registry.backgrounds.names().collect(),
            Kind::Service => registry.services.names().collect(),
        }
    }

    /// Whether the registry numbers `name` of this kind.
    #[must_use]
    pub fn defined(self, registry: &Registry, name: &str) -> bool {
        match self {
            Kind::Map => registry.maps.get(name).is_some(),
            Kind::Flag => registry.flags.get(name).is_some(),
            Kind::Region => registry.regions.get(name).is_some(),
            Kind::Item => registry.items.get(name).is_some(),
            Kind::Spell => registry.spells.get(name).is_some(),
            Kind::Monster => registry.monsters.get(name).is_some(),
            Kind::Condition => registry.conditions.get(name).is_some(),
            Kind::Race => registry.races.get(name).is_some(),
            Kind::Class => registry.classes.get(name).is_some(),
            Kind::Background => registry.backgrounds.get(name).is_some(),
            Kind::Service => registry.services.get(name).is_some(),
        }
    }
}

/// A cell of a map: its level (0 the map's own), x and y.
pub type Spot = (i8, u16, u16);

/// One change a migration makes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Step {
    /// A definition renamed, or moved to another pack: what the save holds of `from` it holds
    /// of `to`.
    Rename {
        /// Its kind.
        kind: Kind,
        /// Its old id, of this pack.
        from: String,
        /// Its id now.
        to: String,
    },
    /// A definition gone, another standing in for it.
    Replace {
        /// Their kind.
        kind: Kind,
        /// The gone one, of this pack.
        from: String,
        /// The one in its place.
        to: String,
    },
    /// A definition gone from the world: it leaves every list that holds it, a removed spell
    /// giving its member back a pick. Nothing is refunded (owner, 2026-10-15).
    Remove {
        /// Its kind.
        kind: Kind,
        /// Its id, of this pack.
        id: String,
    },
    /// A cell moved within a map: the party standing on it, the pile on it, its open doors
    /// and what the automap knows of it go with it.
    MovePlace {
        /// The map, of this pack.
        map: String,
        /// Where it was.
        from: Spot,
        /// Where it is.
        to: Spot,
    },
    /// A fixed encounter's id changed.
    RenameEncounter {
        /// The map, of this pack.
        map: String,
        /// Its old id.
        from: String,
        /// Its id now.
        to: String,
    },
    /// An object placement's id changed.
    RenameObject {
        /// The map, of this pack.
        map: String,
        /// Its old id.
        from: String,
        /// Its id now.
        to: String,
    },
}

impl Step {
    /// The ids of its own pack it names: what it renames or removes, and the map it acts on.
    fn own(&self) -> &str {
        match self {
            Step::Rename { from, .. } | Step::Replace { from, .. } => from,
            Step::Remove { id, .. } => id,
            Step::MovePlace { map, .. }
            | Step::RenameEncounter { map, .. }
            | Step::RenameObject { map, .. } => map,
        }
    }

    /// The kind its own id must have.
    const fn own_kind(&self) -> Kind {
        match self {
            Step::Rename { kind, .. } | Step::Replace { kind, .. } | Step::Remove { kind, .. } => {
                *kind
            }
            _ => Kind::Map,
        }
    }
}

impl core::fmt::Display for Step {
    /// The step in a player's words, ASCII (the window's face has no arrows).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Step::Rename { kind, from, to } => {
                write!(f, "{} '{from}' is now '{to}'", kind.as_str())
            }
            Step::Replace { kind, from, to } => {
                write!(f, "{} '{from}' is replaced by '{to}'", kind.as_str())
            }
            Step::Remove {
                kind: Kind::Spell,
                id,
            } => write!(f, "spell '{id}' is removed; its pick is given back"),
            Step::Remove { kind, id } => write!(f, "{} '{id}' is removed", kind.as_str()),
            Step::MovePlace { map, from, to } => write!(
                f,
                "map '{map}': the cell ({}, {}, {}) moved to ({}, {}, {})",
                from.0, from.1, from.2, to.0, to.1, to.2
            ),
            Step::RenameEncounter { map, from, to } => {
                write!(f, "map '{map}': encounter '{from}' is now '{to}'")
            }
            Step::RenameObject { map, from, to } => {
                write!(f, "map '{map}': object '{from}' is now '{to}'")
            }
        }
    }
}

/// A migration as loaded: its file's name and what it holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackMigration {
    /// Its file, under the pack's `migrations/` (`0001.ron`).
    pub file: String,
    /// The version it leads to.
    pub to: String,
    /// Its steps.
    pub steps: Vec<Step>,
}

/// A version `x.y.z` as numbers, compared part by part.
#[must_use]
pub fn version(text: &str) -> Option<(u64, u64, u64)> {
    let mut parts = text.split('.').map(|p| {
        (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            .then(|| p.parse::<u64>().ok())
            .flatten()
    });
    let v = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(v)
}

/// Compare two versions; `None` when either is not `x.y.z`.
#[must_use]
pub fn compare(a: &str, b: &str) -> Option<Ordering> {
    Some(version(a)?.cmp(&version(b)?))
}

/// The migrations a save made at `saved` is offered for `pack`, at the version the pack is
/// loaded at: every file with `saved < to <= loaded`, in file order. None when either version
/// is not `x.y.z`.
#[must_use]
pub fn chain<'a>(data: &'a Data, pack: &str, saved: &str) -> Vec<&'a PackMigration> {
    let Some(loaded) = data.packs.iter().find(|p| p.id == pack) else {
        return Vec::new();
    };
    let (Some(from), Some(upto)) = (version(saved), version(&loaded.version)) else {
        return Vec::new();
    };
    data.migrations
        .get(pack)
        .into_iter()
        .flatten()
        .filter(|m| version(&m.to).is_some_and(|to| from < to && to <= upto))
        .collect()
}

/// Check one pack's migrations by themselves, in file order: names `NNNN.ron`, each `to` an
/// `x.y.z` rising with the file number and never past the pack's own version, each step naming
/// only this pack's definitions and places, ids well formed, the limits kept.
pub(crate) fn check_pack(
    pack: &str,
    version_now: &str,
    files: &[(PathBuf, PackMigration)],
    errors: &mut Vec<DataError>,
) {
    if files.len() > MAX_MIGRATIONS {
        errors.push(DataError::new(
            "migrations",
            format!("pack '{pack}' has more than {MAX_MIGRATIONS} migrations"),
        ));
    }
    let now = version(version_now);
    if !files.is_empty() && now.is_none() {
        errors.push(DataError::new(
            "pack.ron",
            format!("pack '{pack}' has migrations, so its version '{version_now}' must be x.y.z"),
        ));
    }
    let mut last: Option<(u64, u64, u64)> = None;
    for (path, m) in files {
        let mut error = |message: String| errors.push(DataError::new(path, message));
        if !numbered(&m.file) {
            error(format!("migration file '{}' is not NNNN.ron", m.file));
        }
        match version(&m.to) {
            None => error(format!("to '{}' is not x.y.z", m.to)),
            Some(to) => {
                if last.is_some_and(|l| to <= l) {
                    error(format!(
                        "to '{}' must be after the version the migration before leads to",
                        m.to
                    ));
                }
                if now.is_some_and(|n| to > n) {
                    error(format!(
                        "to '{}' is after the pack's version '{version_now}'",
                        m.to
                    ));
                }
                last = Some(to);
            }
        }
        if m.steps.is_empty() || m.steps.len() > MAX_STEPS {
            error(format!("a migration takes 1 to {MAX_STEPS} steps"));
        }
        for step in &m.steps {
            check_step(pack, step, &mut error);
        }
    }
}

fn numbered(file: &str) -> bool {
    file.strip_suffix(".ron")
        .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()))
}

/// One step by itself.
fn check_step(pack: &str, step: &Step, error: &mut impl FnMut(String)) {
    let kind = step.own_kind();
    let own = step.own();
    let mut id = |id: &str, kind: Kind| {
        if !is_content_id(id) || !string_fits(id) || id.split(':').nth(1) != Some(kind.as_str()) {
            error(format!(
                "'{id}' is not of the form pack:{}:name",
                kind.as_str()
            ));
        }
    };
    id(own, kind);
    match step {
        Step::Rename { to, .. } | Step::Replace { to, .. } => id(to, kind),
        Step::RenameEncounter { from, to, .. } | Step::RenameObject { from, to, .. } => {
            for local in [from, to] {
                if !is_local_id(local) {
                    error(format!("'{local}' is not [a-z0-9_]{{1,32}}"));
                }
            }
        }
        Step::Remove { .. } | Step::MovePlace { .. } => {}
    }
    if own.split(':').next() != Some(pack) {
        error(format!(
            "'{own}' is not this pack's: a migration names only its own pack's ids"
        ));
    }
    if let Step::Remove { kind, id } = step
        && !kind.removable()
    {
        error(format!(
            "{} '{id}' cannot be removed: rename or replace it",
            kind.as_str()
        ));
    }
    let same = match step {
        Step::Rename { from, to, .. }
        | Step::Replace { from, to, .. }
        | Step::RenameEncounter { from, to, .. }
        | Step::RenameObject { from, to, .. } => from == to,
        Step::MovePlace { from, to, .. } => from == to,
        Step::Remove { .. } => false,
    };
    if same {
        error(format!("a step from '{own}' changes nothing"));
    }
}

/// An encounter's or object's id in its map.
fn is_local_id(id: &str) -> bool {
    (1..=32).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Check every pack's migrations against the loaded packs: each `to` name is defined, no
/// `from` name still is; a map step's map is loaded, its new cell inside the map on a level
/// it has, its new encounter or object id there and its old one gone.
pub(crate) fn check_loaded(
    files: &[(PathBuf, String, PackMigration)],
    data: &Data,
    errors: &mut Vec<DataError>,
) {
    let mut packs: BTreeMap<&str, Vec<&PackMigration>> = BTreeMap::new();
    for (path, pack, m) in files {
        packs.entry(pack).or_default().push(m);
        let mut error = |message: String| errors.push(DataError::new(path, message));
        for step in &m.steps {
            check_against(step, data, &mut error);
        }
    }
    // A name renamed on the way (`a` to `b` in one file, `b` to `c` in the next) need not be
    // defined; where the whole chain leads must be. A later file's chain is the end of it.
    for (pack, chain) in packs {
        for ((kind, from), to) in names(&chain) {
            if let Target::To(to) = to
                && !kind.defined(&data.registry, &to)
            {
                errors.push(DataError::new(
                    "migrations",
                    format!(
                        "pack '{pack}' renames {} '{from}' to '{to}', which is not defined by any \
                         loaded pack",
                        kind.as_str()
                    ),
                ));
            }
        }
    }
}

/// Where a name a migration chain names leads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Target {
    /// Another name.
    To(String),
    /// Out of the world.
    Removed,
}

/// The name steps of `chain`, in order, composed: each old name with where it ends (`a` to `b`
/// then `b` to `c` is `a` to `c`, and `b` to `c`).
#[must_use]
pub fn names(chain: &[&PackMigration]) -> BTreeMap<(Kind, String), Target> {
    let mut out: BTreeMap<(Kind, String), Target> = BTreeMap::new();
    for step in chain.iter().flat_map(|m| &m.steps) {
        let (kind, from, target) = match step {
            Step::Rename { kind, from, to } | Step::Replace { kind, from, to } => {
                (*kind, from, Target::To(to.clone()))
            }
            Step::Remove { kind, id } => (*kind, id, Target::Removed),
            _ => continue,
        };
        for ((k, _), t) in &mut out {
            if *k == kind && *t == Target::To(from.clone()) {
                *t = target.clone();
            }
        }
        out.entry((kind, from.clone())).or_insert(target);
    }
    out
}

fn check_against(step: &Step, data: &Data, error: &mut impl FnMut(String)) {
    let registry = &data.registry;
    match step {
        Step::Rename { kind, from, .. } | Step::Replace { kind, from, .. } => {
            if kind.defined(registry, from) {
                error(format!(
                    "{} '{from}' is still defined: a migration names only what is gone",
                    kind.as_str()
                ));
            }
        }
        Step::Remove { kind, id } => {
            if kind.defined(registry, id) {
                error(format!(
                    "{} '{id}' is still defined: a migration names only what is gone",
                    kind.as_str()
                ));
            }
        }
        Step::MovePlace { map, to, .. } => {
            let Some(found) = loaded_map(data, map, error) else {
                return;
            };
            let def = &data.maps[&found].def;
            let (level, x, y) = *to;
            let has_level = level == 0 || def.levels.iter().any(|l| l.level == level);
            if !has_level || x >= def.width || y >= def.height {
                error(format!("({level}, {x}, {y}) is not a cell of map '{map}'"));
            }
        }
        Step::RenameEncounter { map, from, to } => {
            let Some(found) = loaded_map(data, map, error) else {
                return;
            };
            let ids: Vec<&str> = data.maps[&found]
                .def
                .encounters
                .iter()
                .map(|e| e.id.as_str())
                .collect();
            local(&ids, "encounter", map, from, to, error);
        }
        Step::RenameObject { map, from, to } => {
            let Some(found) = loaded_map(data, map, error) else {
                return;
            };
            let ids: Vec<&str> = data.maps[&found]
                .def
                .objects
                .iter()
                .map(|o| o.id.as_str())
                .collect();
            local(&ids, "object", map, from, to, error);
        }
    }
}

fn loaded_map(data: &Data, map: &str, error: &mut impl FnMut(String)) -> Option<MapId> {
    let found = data
        .registry
        .maps
        .get(map)
        .filter(|id| data.maps.contains_key(id));
    if found.is_none() {
        error(format!("map '{map}' is not defined by any loaded pack"));
    }
    found
}

fn local(
    ids: &[&str],
    what: &str,
    map: &str,
    from: &str,
    to: &str,
    error: &mut impl FnMut(String),
) {
    if !ids.contains(&to) {
        error(format!("map '{map}' has no {what} '{to}'"));
    }
    if ids.contains(&from) {
        error(format!(
            "map '{map}' still has {what} '{from}': a migration names only what is gone"
        ));
    }
}

/// The file name of a migration's path.
pub(crate) fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_owned()
}

/// The migrations of every loaded pack, by pack id.
pub type Migrations = BTreeMap<String, Vec<PackMigration>>;
