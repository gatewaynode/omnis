//! What a map places in its cells (M8.5 step 2): objects of the kinds in `data/objects`, each
//! at an offset from its cell's centre in eighths, turned by a facing, and piles of items lying
//! on the ground. Shapes are checked with the map; kinds, items, ground, spills across edges
//! and the cells where the party arrives are checked when the map is resolved.

use crate::error::DataError;
use crate::footprint::Footprint;
use crate::levels::LevelData;
use crate::limits::{
    MAX_CAPACITY, MAX_CHECK_DC, MAX_COLLECTION, MAX_OBJECTS_PER_CELL, MAX_OBJECTS_PER_MAP,
    MAX_PILE_STACKS,
};
use crate::lore::{self, CellLore, Lore};
use crate::map::{Cell, MapDef};
use crate::object::{Look, ObjectKind};
use crate::registry::{Interner, Registry};
use omnis_core::{Facing, ItemId, Level, ObjectKindId, TextKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// The farthest an offset reaches from a cell's centre, in eighths: to the cell's edge.
pub const MAX_OFFSET: i8 = 4;

const fn north() -> Facing {
    Facing::North
}

/// An object placed in a map, as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    /// Unique in the map, `[a-z0-9_]{1,32}`; the save's key for its state.
    pub id: String,
    /// `pack:object:name`.
    pub kind: String,
    /// The level it stands on.
    #[serde(default)]
    pub level: Level,
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// Offset of its centre from the cell's centre, eighths east and south, each -4..=4.
    #[serde(default)]
    pub pos: (i8, i8),
    /// The way it faces; its look's width runs across it.
    #[serde(default = "north")]
    pub facing: Facing,
    /// The state it starts in, instead of its kind's.
    #[serde(default)]
    pub state: Option<String>,
    /// Hidden until found: the difficulty of finding it, 1..=30.
    #[serde(default)]
    pub hidden: Option<u8>,
    /// Text key of a name of its own, instead of its kind's.
    #[serde(default)]
    pub name: Option<String>,
    /// Items a container starts with.
    #[serde(default)]
    pub contents: Vec<(String, u16)>,
    /// Lore of its own for a search, instead of its kind's.
    #[serde(default)]
    pub lore: Option<Lore>,
}

/// Items lying on the ground, as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundDef {
    /// The level.
    #[serde(default)]
    pub level: Level,
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// Offset from the cell's centre, eighths east and south.
    #[serde(default)]
    pub pos: (i8, i8),
    /// Items and counts.
    pub items: Vec<(String, u16)>,
}

/// Items lying on the ground in one cell.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pile {
    /// Offset from the cell's centre, eighths east and south.
    pub pos: (i8, i8),
    /// Items and counts, in the order they were laid down.
    pub items: Vec<(ItemId, u16)>,
}

/// A placement with its kind and items interned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPlacement {
    /// The map's id for it.
    pub id: String,
    /// Its kind.
    pub kind: ObjectKindId,
    /// The level.
    pub level: Level,
    /// Column.
    pub x: u16,
    /// Row.
    pub y: u16,
    /// Offset in eighths.
    pub pos: (i8, i8),
    /// Facing.
    pub facing: Facing,
    /// The state it starts in; empty when its kind has none.
    pub state: String,
    /// The difficulty of finding it, when hidden.
    pub hidden: Option<u8>,
    /// Its own name, when it has one.
    pub name: Option<TextKey>,
    /// What a container starts with.
    pub contents: Vec<(ItemId, u16)>,
    /// Its own lore, when it has some; else its kind's applies.
    pub lore: Option<Lore>,
}

impl ResolvedPlacement {
    /// The room a look takes where this placement stands.
    #[must_use]
    pub fn footprint(&self, look: &Look) -> Footprint {
        Footprint::of(
            self.x,
            self.y,
            self.pos,
            self.facing,
            look.width,
            look.depth,
        )
    }
}

/// Every look a kind can have: its own and each state's.
pub fn looks(kind: &ObjectKind) -> impl Iterator<Item = &Look> {
    core::iter::once(&kind.look).chain(kind.states.values().filter_map(|s| s.look.as_ref()))
}

/// The look, movement blocking and sight blocking of a kind in a state.
#[must_use]
pub fn in_state<'k>(kind: &'k ObjectKind, state: &str) -> (&'k Look, bool, bool) {
    let def = kind.states.get(state);
    let look = def.and_then(|s| s.look.as_ref()).unwrap_or(&kind.look);
    let moves = def
        .and_then(|s| s.blocks_movement)
        .unwrap_or(kind.attributes.blocks_movement);
    let sight = def
        .and_then(|s| s.blocks_sight)
        .unwrap_or(kind.attributes.blocks_sight);
    (look, moves, sight)
}

pub(crate) fn is_placement_id(id: &str) -> bool {
    (1..=32).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// Shape checks with the map: ids, bounds, offsets, counts.
pub(crate) fn validate(def: &MapDef, file: &Path, errors: &mut Vec<DataError>) {
    let mut error = |message: String| errors.push(DataError::new(file, message));
    let inside = |x: u16, y: u16| x < def.width && y < def.height;
    let near = |pos: (i8, i8)| {
        (-MAX_OFFSET..=MAX_OFFSET).contains(&pos.0) && (-MAX_OFFSET..=MAX_OFFSET).contains(&pos.1)
    };
    if def.objects.len() > MAX_OBJECTS_PER_MAP {
        error(format!("at most {MAX_OBJECTS_PER_MAP} objects in a map"));
        return;
    }
    let mut ids = BTreeSet::new();
    for p in &def.objects {
        if !is_placement_id(&p.id) {
            error(format!("object id '{}' is not [a-z0-9_]{{1,32}}", p.id));
        } else if !ids.insert(p.id.as_str()) {
            error(format!("object '{}' is placed twice", p.id));
        }
        if !inside(p.x, p.y) {
            error(format!(
                "object '{}' at ({}, {}) is outside the map",
                p.id, p.x, p.y
            ));
        }
        if !near(p.pos) {
            error(format!(
                "object '{}' pos must be within -4..=4 eighths",
                p.id
            ));
        }
        if p.hidden.is_some_and(|dc| !(1..=MAX_CHECK_DC).contains(&dc)) {
            error(format!(
                "object '{}' hidden dc must be 1..={MAX_CHECK_DC}",
                p.id
            ));
        }
        if p.contents.len() > usize::from(MAX_CAPACITY) || p.contents.iter().any(|c| c.1 == 0) {
            error(format!(
                "object '{}' contents need counts of at least 1, at most {MAX_CAPACITY} stacks",
                p.id
            ));
        }
        if p.lore.as_ref().is_some_and(|l| !l.dc_in_range()) {
            error(format!(
                "object '{}' lore dc must be 1..={MAX_CHECK_DC}",
                p.id
            ));
        }
    }
    if def.ground.len() > MAX_COLLECTION {
        error("too many piles".to_owned());
        return;
    }
    let mut cells = BTreeSet::new();
    for g in &def.ground {
        let (x, y) = (g.x, g.y);
        if !inside(x, y) {
            error(format!("pile at ({x}, {y}) is outside the map"));
        }
        if !cells.insert((g.level, x, y)) {
            error(format!(
                "pile at ({x}, {y}) shares its cell with another pile"
            ));
        }
        if !near(g.pos) {
            error(format!(
                "pile at ({x}, {y}) pos must be within -4..=4 eighths"
            ));
        }
        if g.items.is_empty() || g.items.len() > MAX_PILE_STACKS || g.items.iter().any(|i| i.1 == 0)
        {
            error(format!(
                "pile at ({x}, {y}) needs 1..={MAX_PILE_STACKS} stacks of at least 1"
            ));
        }
    }
}

/// What a map's placements resolve to.
#[derive(Debug, Default)]
pub(crate) struct Placed {
    pub objects: Vec<ResolvedPlacement>,
    pub object_cells: BTreeMap<(Level, u16, u16), Vec<u16>>,
    pub ground: BTreeMap<(Level, u16, u16), Pile>,
    pub lore: BTreeMap<(Level, u16, u16), Vec<CellLore>>,
}

/// What resolution reads besides the map.
pub(crate) struct Lookup<'a> {
    pub registry: &'a Registry,
    pub kinds: &'a BTreeMap<ObjectKindId, ObjectKind>,
}

/// Intern every placement and pile and check them against the kinds, the items, the ground,
/// the edges and the cells where the party arrives.
pub(crate) fn resolve(
    def: &MapDef,
    levels: &[LevelData],
    lookup: &Lookup<'_>,
    file: &Path,
    errors: &mut Vec<DataError>,
) -> Placed {
    let mut error = |message: String| errors.push(DataError::new(file, message));
    let map = Grid { def, levels };
    let arrivals = arrivals(def, levels);
    let mut placed = Placed::default();
    for p in &def.objects {
        let Some(resolved) = resolve_one(p, &map, lookup, &mut error) else {
            continue;
        };
        let kind = &lookup.kinds[&resolved.kind];
        let cells = reach(&resolved, kind, &map, &mut error);
        let (look, moves, _) = in_state(kind, &resolved.state);
        if moves {
            let footprint = resolved.footprint(look);
            for &(x, y) in arrivals.get(&resolved.level).into_iter().flatten() {
                if covers_centre(&footprint, x, y) {
                    error(format!(
                        "object '{}' blocks the centre of ({x}, {y}), where the party arrives",
                        p.id
                    ));
                }
            }
        }
        let index = u16::try_from(placed.objects.len()).unwrap_or(u16::MAX);
        for (x, y) in cells {
            placed
                .object_cells
                .entry((resolved.level, x, y))
                .or_default()
                .push(index);
        }
        placed.objects.push(resolved);
    }
    for ((level, x, y), list) in &placed.object_cells {
        if list.len() > MAX_OBJECTS_PER_CELL {
            error(format!(
                "cell ({x}, {y}) on level {level} holds more than {MAX_OBJECTS_PER_CELL} objects"
            ));
        }
    }
    for g in &def.ground {
        let items = intern_items(&g.items, &lookup.registry.items, &mut error);
        if map.cell(g.level, g.x, g.y).is_none() {
            error(format!("pile at ({}, {}) has no ground", g.x, g.y));
        }
        placed
            .ground
            .insert((g.level, g.x, g.y), Pile { pos: g.pos, items });
    }
    placed.lore = lore::resolve(
        def,
        |level| levels.iter().any(|l| l.level == level),
        |level, x, y| map.cell(level, x, y).is_some(),
        &lookup.registry.text,
        &mut error,
    );
    placed
}

/// Every cell whose centre a placement blocks in its starting state, with the placement's id.
pub(crate) fn blocked_centres(
    placed: &Placed,
    kinds: &BTreeMap<ObjectKindId, ObjectKind>,
) -> BTreeMap<(Level, u16, u16), String> {
    let mut out = BTreeMap::new();
    for (&(level, x, y), list) in &placed.object_cells {
        for p in list
            .iter()
            .filter_map(|&i| placed.objects.get(usize::from(i)))
        {
            let Some(kind) = kinds.get(&p.kind) else {
                continue;
            };
            let (look, moves, _) = in_state(kind, &p.state);
            if moves && covers_centre(&p.footprint(look), x, y) {
                out.entry((level, x, y)).or_insert_with(|| p.id.clone());
            }
        }
    }
    out
}

/// Whether a footprint covers a cell's centre point.
#[must_use]
pub fn covers_centre(footprint: &Footprint, x: u16, y: u16) -> bool {
    let (cx, cy) = (16 * i32::from(x) + 8, 16 * i32::from(y) + 8);
    footprint.x0 < cx && cx < footprint.x1 && footprint.y0 < cy && cy < footprint.y1
}

struct Grid<'a> {
    def: &'a MapDef,
    levels: &'a [LevelData],
}

impl Grid<'_> {
    fn cell(&self, level: Level, x: u16, y: u16) -> Option<&Cell> {
        if x >= self.def.width || y >= self.def.height {
            return None;
        }
        self.levels
            .iter()
            .find(|l| l.level == level)?
            .cells
            .get(usize::from(y) * usize::from(self.def.width) + usize::from(x))?
            .as_ref()
    }
}

fn resolve_one(
    p: &Placement,
    map: &Grid<'_>,
    lookup: &Lookup<'_>,
    error: &mut impl FnMut(String),
) -> Option<ResolvedPlacement> {
    let Some(kind_id) = lookup.registry.objects.get(&p.kind) else {
        error(format!(
            "object '{}' kind '{}' is not defined by any loaded pack",
            p.id, p.kind
        ));
        return None;
    };
    let kind = &lookup.kinds[&kind_id];
    if !map.levels.iter().any(|l| l.level == p.level) {
        error(format!(
            "object '{}' is on level {}, which the map does not have",
            p.id, p.level
        ));
        return None;
    }
    if map.cell(p.level, p.x, p.y).is_none() {
        error(format!(
            "object '{}' at ({}, {}) has no ground",
            p.id, p.x, p.y
        ));
    }
    let state = p
        .state
        .clone()
        .unwrap_or_else(|| kind.initial_state.clone());
    if p.state.is_some() && !kind.states.contains_key(&state) {
        error(format!(
            "object '{}' state '{state}' is not one of its kind's states",
            p.id
        ));
    }
    let name = p.name.as_ref().and_then(|key| {
        let found = lookup.registry.text.get(key);
        if found.is_none() {
            error(format!("text key '{key}' is not defined in any language"));
        }
        found
    });
    let capacity = usize::from(kind.attributes.capacity);
    if !p.contents.is_empty() && capacity == 0 {
        error(format!(
            "object '{}' holds items but its kind is not a container",
            p.id
        ));
    } else if p.contents.len() > capacity {
        error(format!(
            "object '{}' holds {} stacks; its kind holds {capacity}",
            p.id,
            p.contents.len()
        ));
    }
    let (_, moves, sight) = in_state(kind, &state);
    if p.hidden.is_some() && (moves || sight) {
        error(format!("object '{}' is hidden but blocks", p.id));
    }
    if let Some(l) = &p.lore
        && lookup.registry.text.get(&l.text).is_none()
    {
        error(format!(
            "text key '{}' is not defined in any language",
            l.text
        ));
    }
    Some(ResolvedPlacement {
        id: p.id.clone(),
        kind: kind_id,
        level: p.level,
        x: p.x,
        y: p.y,
        pos: p.pos,
        facing: p.facing,
        state,
        hidden: p.hidden,
        name,
        contents: intern_items(&p.contents, &lookup.registry.items, error),
        lore: p.lore.clone(),
    })
}

/// Every cell any of the kind's looks reaches into, after checking that each is on the map, has
/// ground, and is not across a wall, a door or a cliff from its neighbour in the footprint.
fn reach(
    p: &ResolvedPlacement,
    kind: &ObjectKind,
    map: &Grid<'_>,
    error: &mut impl FnMut(String),
) -> BTreeSet<(u16, u16)> {
    let mut cells = BTreeSet::new();
    for look in looks(kind) {
        for (cx, cy) in p.footprint(look).cells() {
            match (u16::try_from(cx), u16::try_from(cy)) {
                (Ok(x), Ok(y)) if map.cell(p.level, x, y).is_some() => {
                    cells.insert((x, y));
                }
                _ => {
                    error(format!(
                        "object '{}' reaches into ({cx}, {cy}), which has no ground",
                        p.id
                    ));
                    return BTreeSet::new();
                }
            }
        }
    }
    for &(x, y) in &cells {
        let Some(cell) = map.cell(p.level, x, y) else {
            continue;
        };
        for (facing, next) in [(Facing::East, (x + 1, y)), (Facing::South, (x, y + 1))] {
            let shut = cell.walls.has(facing) || cell.doors.has(facing) || cell.cliffs.has(facing);
            if shut && cells.contains(&next) {
                error(format!(
                    "object '{}' reaches across the {facing:?} edge of ({x}, {y})",
                    p.id
                ));
            }
        }
    }
    cells
}

/// The cells where the party is put without walking a path to the centre: the start, every
/// join (a stair, a ladder, a hole), every site and every fixed encounter, by level.
fn arrivals(def: &MapDef, levels: &[LevelData]) -> BTreeMap<Level, Vec<(u16, u16)>> {
    let mut out: BTreeMap<Level, Vec<(u16, u16)>> = BTreeMap::new();
    out.entry(Level::GROUND)
        .or_default()
        .push((def.start.0, def.start.1));
    for level in levels {
        for (i, cell) in level.cells.iter().enumerate() {
            let joined = cell
                .as_ref()
                .and_then(|c| def.terrains.get(usize::from(c.terrain)))
                .is_some_and(|t| t.join.is_some());
            if joined && def.width > 0 {
                let x = u16::try_from(i % usize::from(def.width)).unwrap_or(0);
                let y = u16::try_from(i / usize::from(def.width)).unwrap_or(0);
                out.entry(level.level).or_default().push((x, y));
            }
        }
    }
    for s in &def.sites {
        out.entry(s.level).or_default().push((s.x, s.y));
    }
    for e in &def.encounters {
        out.entry(e.level).or_default().push((e.x, e.y));
    }
    out
}

fn intern_items(
    items: &[(String, u16)],
    interner: &Interner<ItemId>,
    error: &mut impl FnMut(String),
) -> Vec<(ItemId, u16)> {
    items
        .iter()
        .filter_map(|(id, count)| {
            let found = interner.get(id);
            if found.is_none() {
                error(format!("item '{id}' is not defined by any loaded pack"));
            }
            found.map(|item| (item, *count))
        })
        .collect()
}
