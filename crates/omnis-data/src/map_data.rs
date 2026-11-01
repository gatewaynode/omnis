//! A map as the simulation reads it: its parsed levels and cells, and the references the
//! loader resolved (portals, encounters, sites, rest events), with the lookups by tile.

use crate::encounter::{self, ResolvedEncounter, ResolvedRandom};
use crate::error::DataError;
use crate::levels::{self, LevelData};
use crate::loader::Data;
use crate::lore::CellLore;
use crate::map::{Cell, MapDef, Portal, Terrain};
use crate::placement::{self, Pile, ResolvedPlacement};
use crate::registry::Interner;
use crate::rest_event::{self, ResolvedRestEvent};
use crate::service::{self, ResolvedSite};
use crate::tileset::{SlotKind, Tileset};
use omnis_core::{Facing, Level, MapId, RegionId, ServiceId, TextKey, TilesetId};
use std::collections::BTreeMap;
use std::path::Path;

/// A portal with its destination resolved to an interned map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPortal {
    /// Trigger level.
    pub level: Level,
    /// Trigger column.
    pub x: u16,
    /// Trigger row.
    pub y: u16,
    /// Destination map.
    pub to_map: MapId,
    /// Destination level.
    pub to_level: Level,
    /// Destination column.
    pub to_x: u16,
    /// Destination row.
    pub to_y: u16,
    /// Facing on arrival.
    pub to_facing: Facing,
}

/// A map ready for the simulation: parsed cells and resolved references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapData {
    /// The file as written.
    pub def: MapDef,
    /// Every level, lowest first, the map's own (level 0) among them; each row-major,
    /// `width × height`, `None` where there is no ground.
    pub levels: Vec<LevelData>,
    /// The interned tileset.
    pub tileset: TilesetId,
    /// The interned display-name key.
    pub name: TextKey,
    /// Portals with interned destinations.
    pub portals: Vec<ResolvedPortal>,
    /// Fixed encounters with interned monsters, in file order.
    pub encounters: Vec<ResolvedEncounter>,
    /// The random table with interned monsters, if the map has one.
    pub random: Option<ResolvedRandom>,
    /// Services placed on tiles, in file order.
    pub sites: Vec<ResolvedSite>,
    /// The region that owns the map.
    pub region: RegionId,
    /// What may happen while resting here, in file order.
    pub rest_events: Vec<ResolvedRestEvent>,
    /// Objects placed in the map, in file order (M8.5 step 2).
    pub objects: Vec<ResolvedPlacement>,
    /// For each cell any of an object's looks reaches into, the objects' indices.
    pub object_cells: BTreeMap<(Level, u16, u16), Vec<u16>>,
    /// Piles placed on the ground, by cell.
    pub ground: BTreeMap<(Level, u16, u16), Pile>,
    /// Lore written on cells, by cell, in file order.
    pub lore: BTreeMap<(Level, u16, u16), Vec<CellLore>>,
}

impl MapData {
    /// The cell at a coordinate of a level, if inside the map, on a level it has, with ground.
    #[must_use]
    pub fn cell(&self, level: Level, x: u16, y: u16) -> Option<&Cell> {
        if !self.inside(x, y) {
            return None;
        }
        self.level(level)?
            .cells
            .get(usize::from(y) * usize::from(self.def.width) + usize::from(x))?
            .as_ref()
    }

    /// Whether the party may stand on a cell as the files describe it: ground on a level the
    /// map has, and terrain it may enter (objects are the engine's to check). The entrances and
    /// exits enhancement (W1): every landing and every start is asked this.
    #[must_use]
    pub fn enterable(&self, level: Level, x: u16, y: u16) -> bool {
        enterable(&self.def, &self.levels, level, x, y)
    }

    /// Whether a coordinate is inside the map.
    #[must_use]
    pub const fn inside(&self, x: u16, y: u16) -> bool {
        x < self.def.width && y < self.def.height
    }

    /// A level of the map, if it has it.
    #[must_use]
    pub fn level(&self, level: Level) -> Option<&LevelData> {
        self.levels.iter().find(|l| l.level == level)
    }

    /// The way the stair at `(x, y)` of `level` rises, if there is one (M8.5 step 1c).
    #[must_use]
    pub fn rise(&self, level: Level, x: u16, y: u16) -> Option<Facing> {
        self.level(level)?.rises.get(&(x, y)).copied()
    }

    /// The next level up (`up`) or down from `level`, if the map has one.
    #[must_use]
    pub fn beyond(&self, level: Level, up: bool) -> Option<Level> {
        levels::beyond(&self.levels, level, up).map(|l| l.level)
    }

    /// The terrain of a cell.
    #[must_use]
    pub fn terrain(&self, cell: &Cell) -> &Terrain {
        &self.def.terrains[usize::from(cell.terrain)]
    }

    /// The portal on a tile, if any.
    #[must_use]
    pub fn portal_at(&self, level: Level, x: u16, y: u16) -> Option<&ResolvedPortal> {
        self.portals
            .iter()
            .find(|p| p.level == level && p.x == x && p.y == y)
    }

    /// The fixed encounter on a tile, if any.
    #[must_use]
    pub fn encounter_at(&self, level: Level, x: u16, y: u16) -> Option<&ResolvedEncounter> {
        self.encounters
            .iter()
            .find(|e| e.level == level && e.x == x && e.y == y)
    }

    /// The fixed encounter with this id, if any.
    #[must_use]
    pub fn encounter(&self, id: &str) -> Option<&ResolvedEncounter> {
        self.encounters.iter().find(|e| e.id == id)
    }

    /// The `Object` surface marking the portal on a tile, if any.
    #[must_use]
    pub fn marker_at(&self, level: Level, x: u16, y: u16) -> Option<&str> {
        self.def
            .portals
            .iter()
            .find(|p| p.level == level && p.x == x && p.y == y)
            .and_then(|p| p.marker.as_deref())
    }

    /// The objects whose footprints reach into a cell, with their indices, in file order.
    pub fn objects_at(
        &self,
        level: Level,
        x: u16,
        y: u16,
    ) -> impl Iterator<Item = (u16, &ResolvedPlacement)> {
        self.object_cells
            .get(&(level, x, y))
            .into_iter()
            .flatten()
            .filter_map(|&i| self.objects.get(usize::from(i)).map(|p| (i, p)))
    }

    /// The service placed on a tile, if any.
    #[must_use]
    pub fn site_at(&self, level: Level, x: u16, y: u16) -> Option<ServiceId> {
        self.sites
            .iter()
            .find(|s| s.level == level && s.x == x && s.y == y)
            .map(|s| s.service)
    }
}

// ---- Resolution: the loader's per-map pass, shared with the editor (M5.1) ----

/// What a map's resolution asks of the other maps: a portal's destination, and the region that
/// owns a map. The loader answers from the files it read; the editor from loaded [`Data`].
pub(crate) trait MapTargets {
    /// The interned id, definition and parsed levels of the map `id` names.
    fn target(&self, id: &str) -> Option<Target<'_>>;
    /// The region owning `map`.
    fn owner(&self, map: MapId) -> Option<RegionId>;
}

/// A map as another map's portal sees it: its id, its definition and its parsed levels.
#[derive(Clone, Copy)]
pub(crate) struct Target<'a> {
    pub id: MapId,
    pub def: &'a MapDef,
    pub levels: &'a [LevelData],
}

/// The cell at a coordinate of a level as the files describe it, if inside the map, on a level
/// it has, with ground.
fn cell_of<'a>(
    def: &MapDef,
    levels: &'a [LevelData],
    level: Level,
    x: u16,
    y: u16,
) -> Option<&'a Cell> {
    if x >= def.width || y >= def.height {
        return None;
    }
    levels
        .iter()
        .find(|l| l.level == level)?
        .cells
        .get(usize::from(y) * usize::from(def.width) + usize::from(x))?
        .as_ref()
}

/// Whether the party may stand on a cell as the files describe it (see [`MapData::enterable`]).
pub(crate) fn enterable(def: &MapDef, levels: &[LevelData], level: Level, x: u16, y: u16) -> bool {
    cell_of(def, levels, level, x, y)
        .is_some_and(|cell| def.terrains[usize::from(cell.terrain)].passable)
}

/// Why the party cannot stand where it would arrive, if it cannot: no ground, or terrain it may
/// not enter. Nothing when the level's layout did not parse: its own errors say what is wrong.
fn standing_fault(
    def: &MapDef,
    levels: &[LevelData],
    level: Level,
    x: u16,
    y: u16,
) -> Option<&'static str> {
    levels.iter().find(|l| l.level == level)?;
    match cell_of(def, levels, level, x, y) {
        None => Some("is on no ground"),
        Some(cell) if !def.terrains[usize::from(cell.terrain)].passable => {
            Some("is on a tile the party cannot enter")
        }
        Some(_) => None,
    }
}

/// The way back from a portal's landing (the entrances and exits enhancement, W1, the owner's
/// strict mirror of 2026-11-01): the portal of `far` standing directly behind the landing, so
/// the party arrives facing away from it, and leading back to `from`. `None` when no portal
/// stands there, or nothing stands behind the landing (the map's edge).
#[must_use]
pub fn way_back<'a>(portal: &Portal, from: &str, far: &'a MapDef) -> Option<&'a Portal> {
    let (x, y) = portal.to_facing.behind((portal.to_x, portal.to_y))?;
    far.portals
        .iter()
        .find(|p| p.level == portal.to_level && (p.x, p.y) == (x, y) && p.to_map == from)
}

fn no_way_back(portal: &Portal, from: &str) -> String {
    format!(
        "portal at ({}, {}) has no way back: no portal behind its landing ({}, {}) on '{}' leads to '{from}'",
        portal.x, portal.y, portal.to_x, portal.to_y, portal.to_map
    )
}

/// The maps of `pack` that `entry` does not reach through portals, among the maps that have a
/// portal at all (a map with none is unfinished, not unreached), in id order. Nothing when
/// `entry` is not among `maps`: that is reported on its own.
#[must_use]
pub fn unreached<'a>(
    maps: impl Iterator<Item = &'a MapDef>,
    entry: &str,
    pack: &str,
) -> Vec<String> {
    let by_id: BTreeMap<&str, &MapDef> = maps.map(|d| (d.id.as_str(), d)).collect();
    if !by_id.contains_key(entry) {
        return Vec::new();
    }
    let mut reached = std::collections::BTreeSet::from([entry]);
    let mut queue = vec![entry];
    while let Some(id) = queue.pop() {
        for portal in by_id
            .get(id)
            .map(|d| d.portals.as_slice())
            .unwrap_or_default()
        {
            if let Some(&to) = by_id.get_key_value(portal.to_map.as_str()).map(|(k, _)| k)
                && reached.insert(to)
            {
                queue.push(to);
            }
        }
    }
    by_id
        .values()
        .filter(|d| {
            !reached.contains(d.id.as_str())
                && d.id.split(':').next() == Some(pack)
                && !d.portals.is_empty()
        })
        .map(|d| d.id.clone())
        .collect()
}

/// The loader's words for a map `unreached` names.
#[must_use]
pub fn unreached_message(map: &str, entry: &str) -> String {
    format!("map '{map}' cannot be reached from the entry map '{entry}' through portals")
}

/// One map resolved, with the cells where an object blocks the centre (what a portal landing
/// there is checked against).
pub(crate) struct Resolved {
    pub map: MapData,
    pub centres: BTreeMap<(Level, u16, u16), String>,
}

/// Resolve one map's references against the loaded content: the tileset's surfaces, the text
/// keys, portals, encounters, sites, rest events, placements, beings and the owning region.
/// `None` when the tileset is unknown (nothing else can be checked); otherwise the map, with
/// every problem recorded in `errors`.
pub(crate) fn resolve_one(
    id: MapId,
    file: &Path,
    def: &MapDef,
    levels: &[LevelData],
    data: &Data,
    targets: &dyn MapTargets,
    errors: &mut Vec<DataError>,
) -> Option<Resolved> {
    let Some(tileset) = data.registry.tilesets.get(&def.tileset) else {
        errors.push(DataError::new(
            file,
            format!(
                "tileset '{}' is not defined by any loaded pack",
                def.tileset
            ),
        ));
        return None;
    };
    check_surfaces(def, &data.tilesets[&tileset], file, errors);
    let (sx, sy, _) = def.start;
    if sx < def.width
        && sy < def.height
        && let Some(fault) = standing_fault(def, levels, Level::GROUND, sx, sy)
    {
        errors.push(DataError::new(file, format!("start ({sx}, {sy}) {fault}")));
    }
    let name = names(def, levels, &data.registry.text, file, errors);
    let portals = resolve_portals(def, file, targets, errors);
    let (encounters, random) = encounter::resolve(
        &def.encounters,
        def.random.as_ref(),
        file,
        &data.registry.monsters,
        errors,
    );
    let sites = service::resolve_sites(def, levels, &data.registry.services, file, errors);
    let rest_events = rest_event::resolve(def, &data.registry.text, file, errors);
    let lookup = placement::Lookup {
        registry: &data.registry,
        kinds: &data.objects,
    };
    let placed = placement::resolve(def, levels, &lookup, file, errors);
    placement::check_beings(def, levels, &encounters, &data.monsters, file, errors);
    let centres = placement::blocked_centres(&placed.objects, &placed.object_cells, &data.objects);
    let region = targets.owner(id);
    if region.is_none() {
        errors.push(DataError::new(
            file,
            format!("map '{}' belongs to no region", def.id),
        ));
    }
    Some(Resolved {
        map: MapData {
            def: def.clone(),
            levels: levels.to_vec(),
            tileset,
            name,
            portals,
            encounters,
            random,
            sites,
            region: region.unwrap_or(RegionId(0)),
            rest_events,
            objects: placed.objects,
            object_cells: placed.object_cells,
            ground: placed.ground,
            lore: placed.lore,
        },
        centres,
    })
}

/// The map's name key, and each level's name key checked; a missing map name reads as key 0
/// so resolution can go on and report the rest.
fn names(
    def: &MapDef,
    levels: &[LevelData],
    text: &Interner<TextKey>,
    file: &Path,
    errors: &mut Vec<DataError>,
) -> TextKey {
    let name = match text.get(&def.name) {
        Some(key) => key,
        None => {
            errors.push(DataError::new(
                file,
                format!(
                    "name text key '{}' is not defined in any language",
                    def.name
                ),
            ));
            TextKey(0)
        }
    };
    for level in levels.iter().filter(|l| !l.name.is_empty()) {
        if text.get(&level.name).is_none() {
            errors.push(DataError::new(
                file,
                format!(
                    "level {} name text key '{}' is not defined in any language",
                    level.level, level.name
                ),
            ));
        }
    }
    name
}

/// Every surface a map names must exist in its tileset with the right kind.
fn check_surfaces(def: &MapDef, tileset: &Tileset, file: &Path, errors: &mut Vec<DataError>) {
    let mut surface = |name: &str, kind: SlotKind, what: &str| match tileset.surfaces.get(name) {
        None => errors.push(DataError::new(
            file,
            format!(
                "{what} surface '{name}' is not in tileset '{}'",
                def.tileset
            ),
        )),
        Some(s) if s.kind != kind => errors.push(DataError::new(
            file,
            format!(
                "{what} surface '{name}' is a {:?} surface, not {kind:?}",
                s.kind
            ),
        )),
        Some(_) => {}
    };
    surface(&def.wall.front, SlotKind::WallFront, "wall front");
    surface(&def.wall.left, SlotKind::WallLeft, "wall left");
    surface(&def.wall.right, SlotKind::WallRight, "wall right");
    surface(&def.door, SlotKind::Door, "door");
    if let Some(open) = &def.door_open {
        surface(open, SlotKind::DoorFrame, "open door");
    }
    for terrain in &def.terrains {
        surface(&terrain.floor, SlotKind::Floor, "terrain floor");
        if let Some(ceiling) = &terrain.ceiling {
            surface(ceiling, SlotKind::Ceiling, "terrain ceiling");
        }
        if let Some(block) = &terrain.block {
            surface(block, SlotKind::Block, "terrain block");
        }
    }
    for marker in def.portals.iter().filter_map(|p| p.marker.as_deref()) {
        surface(marker, SlotKind::Object, "portal marker");
    }
}

/// Portals must lead to a known map, land inside it where the party can stand and no portal
/// stands, and have their way back behind the landing.
fn resolve_portals(
    def: &MapDef,
    file: &Path,
    targets: &dyn MapTargets,
    errors: &mut Vec<DataError>,
) -> Vec<ResolvedPortal> {
    let mut portals = Vec::with_capacity(def.portals.len());
    for portal in &def.portals {
        let Some(target) = targets.target(&portal.to_map) else {
            errors.push(DataError::new(
                file,
                format!(
                    "portal at ({}, {}) leads to unknown map '{}'",
                    portal.x, portal.y, portal.to_map
                ),
            ));
            continue;
        };
        let messages = arrival_errors(portal, target.def, target.levels);
        let on_map = portal.x < def.width && portal.y < def.height;
        if on_map && messages.is_empty() && way_back(portal, &def.id, target.def).is_none() {
            errors.push(DataError::new(file, no_way_back(portal, &def.id)));
        }
        for message in messages {
            errors.push(DataError::new(file, message));
        }
        portals.push(ResolvedPortal {
            level: portal.level,
            x: portal.x,
            y: portal.y,
            to_map: target.id,
            to_level: portal.to_level,
            to_x: portal.to_x,
            to_y: portal.to_y,
            to_facing: portal.to_facing,
        });
    }
    portals
}

/// Why a portal cannot land on `target`: a level it does not have, a cell outside it, no
/// ground, terrain the party cannot enter, or a portal standing there.
fn arrival_errors(portal: &Portal, target: &MapDef, levels: &[LevelData]) -> Vec<String> {
    let mut out = Vec::new();
    let has_level = portal.to_level.is_ground()
        || target
            .levels
            .iter()
            .any(|l| Level(l.level) == portal.to_level);
    if !has_level {
        out.push(format!(
            "portal at ({}, {}) lands on level {}, which map '{}' does not have",
            portal.x, portal.y, portal.to_level, portal.to_map
        ));
    }
    if portal.to_x >= target.width || portal.to_y >= target.height {
        out.push(format!(
            "portal at ({}, {}) lands outside map '{}'",
            portal.x, portal.y, portal.to_map
        ));
    }
    if !out.is_empty() {
        return out;
    }
    let (level, x, y) = (portal.to_level, portal.to_x, portal.to_y);
    let at = format!(
        "portal at ({}, {}) lands on ({x}, {y}) of '{}'",
        portal.x, portal.y, portal.to_map
    );
    match standing_fault(target, levels, level, x, y) {
        Some("is on no ground") => out.push(format!("{at}, where there is no ground")),
        Some(_) => out.push(format!("{at}, which the party cannot enter")),
        None => {
            if target
                .portals
                .iter()
                .any(|p| p.level == level && (p.x, p.y) == (x, y))
            {
                out.push(format!("{at}, where a portal stands"));
            }
        }
    }
    out
}

/// Why a resolved portal cannot land where it points: an object blocks the centre of the cell
/// (`centres` are the destination map's).
pub(crate) fn landing_message(
    portal: &ResolvedPortal,
    centres: &BTreeMap<(Level, u16, u16), String>,
) -> Option<String> {
    let at = (portal.to_level, portal.to_x, portal.to_y);
    centres.get(&at).map(|object| {
        format!(
            "portal at ({}, {}) lands on ({}, {}), where object '{object}' blocks the centre",
            portal.x, portal.y, at.1, at.2
        )
    })
}

/// The loaded content, with one map's definition standing in for what the files hold.
struct DataTargets<'a> {
    data: &'a Data,
    edited: Target<'a>,
}

impl MapTargets for DataTargets<'_> {
    fn target(&self, id: &str) -> Option<Target<'_>> {
        if id == self.edited.def.id {
            return Some(self.edited);
        }
        let map = self.data.registry.maps.get(id)?;
        self.data.maps.get(&map).map(|m| Target {
            id: map,
            def: &m.def,
            levels: &m.levels,
        })
    }

    fn owner(&self, map: MapId) -> Option<RegionId> {
        self.data
            .regions
            .iter()
            .find(|(_, region)| region.maps.contains(&map))
            .map(|(id, _)| *id)
    }
}

impl MapData {
    /// The editor's path: `def` resolved against the loaded `data` exactly as the loader would
    /// resolve its file, without touching disk. Shape and layout first (the loader's per-file
    /// checks), then the references, then the other maps' portals that land on this one and this
    /// map's portals that land among their objects. `Err` carries every problem, as a load does.
    pub fn resolve(def: &MapDef, data: &Data) -> Result<MapData, Vec<DataError>> {
        let name = def.id.rsplit(':').next().unwrap_or(&def.id);
        let file = Path::new("data/maps").join(format!("{name}.ron"));
        let mut errors = Vec::new();
        crate::loader::check_id(&def.id, "map", &file, &mut errors);
        def.validate(&file, &mut errors);
        let levels = levels::parse(def, &file, &mut errors);
        let (Some(levels), true) = (levels, errors.is_empty()) else {
            return Err(errors);
        };
        let id = data.registry.maps.get(&def.id).unwrap_or_else(|| {
            MapId::from(u32::try_from(data.registry.maps.len()).unwrap_or(u32::MAX))
        });
        let targets = DataTargets {
            data,
            edited: Target {
                id,
                def,
                levels: &levels,
            },
        };
        let Some(resolved) = resolve_one(id, &file, def, &levels, data, &targets, &mut errors)
        else {
            return Err(errors);
        };
        check_inbound(id, def, &resolved.centres, data, &file, &mut errors);
        check_outbound(id, &resolved, data, &file, &mut errors);
        check_reached(def, data, &file, &mut errors);
        if errors.is_empty() {
            Ok(resolved.map)
        } else {
            Err(errors)
        }
    }
}

/// The other maps' portals that land on this map still land inside it, on a level it has, and
/// not where an object blocks the centre. Reported against this map's file, since it is the
/// one being changed, with the other map's id first.
fn check_inbound(
    id: MapId,
    def: &MapDef,
    centres: &BTreeMap<(Level, u16, u16), String>,
    data: &Data,
    file: &Path,
    errors: &mut Vec<DataError>,
) {
    let levels = resolved_levels(def);
    for other in data.maps.values().filter(|m| m.def.id != def.id) {
        for portal in other.def.portals.iter().filter(|p| p.to_map == def.id) {
            let mut messages = arrival_errors(portal, def, &levels);
            if messages.is_empty() {
                if way_back(portal, &other.def.id, def).is_none() {
                    messages.push(no_way_back(portal, &other.def.id));
                }
                let resolved = ResolvedPortal {
                    level: portal.level,
                    x: portal.x,
                    y: portal.y,
                    to_map: id,
                    to_level: portal.to_level,
                    to_x: portal.to_x,
                    to_y: portal.to_y,
                    to_facing: portal.to_facing,
                };
                messages.extend(landing_message(&resolved, centres));
            }
            for message in messages {
                errors.push(DataError::new(file, format!("{}: {message}", other.def.id)));
            }
        }
    }
}

/// The edited map's levels as `resolve` parsed them, for the other maps' portals landing on it.
/// Parsed again here (cheaply: the layout is text) rather than threaded through, since the
/// shape passed before this is reached.
fn resolved_levels(def: &MapDef) -> Vec<LevelData> {
    let mut errors = Vec::new();
    levels::parse(def, Path::new(""), &mut errors).unwrap_or_default()
}

/// Every map of the pack that sets the entry is reached from the entry map through portals, the
/// edited map standing in for its file (the entrances and exits enhancement, W1): the editor
/// hears it when a change cuts a map off, as the loader would on the next load.
fn check_reached(def: &MapDef, data: &Data, file: &Path, errors: &mut Vec<DataError>) {
    let Some(entry) = data.entry else {
        return;
    };
    let Some(pack) = data.packs.iter().rev().find(|p| p.entry.is_some()) else {
        return;
    };
    let entry = match data.maps.get(&entry) {
        Some(map) if map.def.id != def.id => map.def.id.as_str(),
        Some(_) => def.id.as_str(),
        None => return,
    };
    let maps = data
        .maps
        .values()
        .map(|m| &m.def)
        .filter(|d| d.id != def.id)
        .chain(std::iter::once(def));
    for missing in unreached(maps, entry, &pack.id) {
        errors.push(DataError::new(file, unreached_message(&missing, entry)));
    }
}

/// This map's portals land among the destination's objects as the loader checks after every
/// map is read: against this map's own centres when a portal comes back to it.
fn check_outbound(
    id: MapId,
    resolved: &Resolved,
    data: &Data,
    file: &Path,
    errors: &mut Vec<DataError>,
) {
    for portal in &resolved.map.portals {
        let message = if portal.to_map == id {
            landing_message(portal, &resolved.centres)
        } else {
            data.maps.get(&portal.to_map).and_then(|target| {
                let centres = placement::blocked_centres(
                    &target.objects,
                    &target.object_cells,
                    &data.objects,
                );
                landing_message(portal, &centres)
            })
        };
        if let Some(message) = message {
            errors.push(DataError::new(file, message));
        }
    }
}
