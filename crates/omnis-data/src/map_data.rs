//! A map as the simulation reads it: its parsed levels and cells, and the references the
//! loader resolved (portals, encounters, sites, rest events), with the lookups by tile. A
//! portal resolves through its link (the entrances and exits enhancement, W1 step 2): its
//! landing is the cell ahead of its partner; an unlinked portal resolves to nothing.

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

/// A linked portal with its landing derived from its partner and the partner's map interned.
/// An unlinked portal has none: it is a plain tile in play.
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
    /// The linked portals with their landings (an unlinked portal is not among them).
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

    /// The linked portal on a tile, if any (an unlinked one is a plain tile).
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

/// What a map's resolution asks of the other maps: every map (a portal's partner may stand on
/// any of them), and the region that owns a map. The loader answers from the files it read; the
/// editor from loaded [`Data`] with the edited map standing in for its file.
pub(crate) trait MapTargets {
    /// Every map, in id order, with its interned id, definition and parsed levels.
    fn maps(&self) -> Vec<Target<'_>>;
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

// ---- Links (the entrances and exits enhancement, W1 step 2, the owner's direction of
// 2026-11-01): a pair is stated by either side; the landing is ahead of the partner ----

/// The portal `id` names among `maps`, with its map; the first in map order when two share it
/// (which is its own error).
fn find_portal<'a>(maps: &[Target<'a>], id: &str) -> Option<(Target<'a>, &'a Portal)> {
    maps.iter()
        .find_map(|t| t.def.portals.iter().find(|p| p.id == id).map(|p| (*t, p)))
}

/// A portal's partner: the portal its `link` names, or the one portal anywhere whose `link`
/// names it (one side stating the link is enough; both may, agreeing), with the map it stands
/// on. `Ok(None)` for an unlinked portal. `own` is the id of the map `portal` stands on.
///
/// # Errors
/// The loader's words: the link names the portal itself, names no portal of any loaded map, or
/// names a portal that links to a third; or two portals claim this one.
pub(crate) fn pairing<'a>(
    portal: &Portal,
    own: &str,
    maps: &[Target<'a>],
) -> Result<Option<(Target<'a>, &'a Portal)>, String> {
    let this = format!("portal '{}' at ({}, {})", portal.id, portal.x, portal.y);
    let mut partners: Vec<(Target<'a>, &'a Portal)> = Vec::new();
    if let Some(link) = &portal.link {
        if *link == portal.id {
            return Err(format!("{this} links to itself"));
        }
        let Some((far, q)) = find_portal(maps, link) else {
            return Err(format!("{this} links to '{link}', which no loaded map has"));
        };
        if let Some(back) = &q.link
            && *back != portal.id
        {
            return Err(format!(
                "portal '{}' links to '{link}', which links to '{back}'",
                portal.id
            ));
        }
        partners.push((far, q));
    }
    for far in maps {
        for q in &far.def.portals {
            let itself = far.def.id == own && q.id == portal.id;
            let claims = q.link.as_deref() == Some(portal.id.as_str());
            let held = partners
                .iter()
                .any(|(t, p)| t.def.id == far.def.id && p.id == q.id);
            if claims && !itself && !held {
                partners.push((*far, q));
            }
        }
    }
    match partners.as_slice() {
        [] => Ok(None),
        [one] => Ok(Some(*one)),
        [(t1, a), (t2, b), ..] => Err(format!(
            "{this} is linked with both '{}' on '{}' and '{}' on '{}'",
            a.id, t1.def.id, b.id, t2.def.id
        )),
    }
}

/// Where `portal` lands: the cell ahead of its partner `q` on `far`, where the party can stand
/// and no portal stands.
///
/// # Errors
/// The loader's words: the cell ahead is outside the map, has no ground, cannot be entered, or
/// holds a portal.
pub(crate) fn landing(
    portal: &Portal,
    far: Target<'_>,
    q: &Portal,
) -> Result<(Level, u16, u16), String> {
    let at = format!(
        "portal '{}' at ({}, {}) lands ahead of '{}' on '{}'",
        portal.id, portal.x, portal.y, q.id, far.def.id
    );
    let Some((x, y)) = q
        .landing()
        .filter(|&(x, y)| x < far.def.width && y < far.def.height)
    else {
        return Err(format!("{at}, outside the map"));
    };
    match standing_fault(far.def, far.levels, q.level, x, y) {
        Some("is on no ground") => Err(format!("{at} at ({x}, {y}), where there is no ground")),
        Some(_) => Err(format!("{at} at ({x}, {y}), which the party cannot enter")),
        None if far
            .def
            .portals
            .iter()
            .any(|p| p.level == q.level && (p.x, p.y) == (x, y)) =>
        {
            Err(format!("{at} at ({x}, {y}), where a portal stands"))
        }
        None => Ok((q.level, x, y)),
    }
}

/// `portal` resolved through its partner, if it has one and lands clean.
fn resolve_portal(
    portal: &Portal,
    own: &str,
    maps: &[Target<'_>],
) -> Result<Option<ResolvedPortal>, String> {
    let Some((far, q)) = pairing(portal, own, maps)? else {
        return Ok(None);
    };
    let (to_level, to_x, to_y) = landing(portal, far, q)?;
    Ok(Some(ResolvedPortal {
        level: portal.level,
        x: portal.x,
        y: portal.y,
        to_map: far.id,
        to_level,
        to_x,
        to_y,
        to_facing: q.facing,
    }))
}

/// Every loaded map with `edited` standing in for its id (or added, for a map the packs lack),
/// in id order: what the editor's resolution sees.
fn data_maps<'a>(data: &'a Data, edited: Target<'a>) -> Vec<Target<'a>> {
    let mut out: Vec<Target<'a>> = data
        .maps
        .iter()
        .filter(|(_, m)| m.def.id != edited.def.id)
        .map(|(id, m)| Target {
            id: *id,
            def: &m.def,
            levels: &m.levels,
        })
        .collect();
    out.push(edited);
    out.sort_by(|a, b| a.def.id.cmp(&b.def.id));
    out
}

/// The maps as the editor sees them: `def` standing in for its id among the loaded maps.
fn editor_maps<'a>(def: &'a MapDef, data: &'a Data) -> Vec<Target<'a>> {
    let id = data
        .registry
        .maps
        .get(&def.id)
        .unwrap_or_else(|| MapId::from(u32::MAX));
    data_maps(
        data,
        Target {
            id,
            def,
            levels: &[],
        },
    )
}

/// A portal's partner as the editor sees it: among the loaded maps, with `def` standing in for
/// its id. `None` when the portal is unlinked or its links are in error (the loader's words say
/// which).
#[must_use]
pub fn partner_of<'a>(
    portal: &Portal,
    def: &'a MapDef,
    data: &'a Data,
) -> Option<(&'a str, &'a Portal)> {
    let maps = editor_maps(def, data);
    pairing(portal, &def.id, &maps)
        .ok()
        .flatten()
        .map(|(far, q)| (far.def.id.as_str(), q))
}

/// Every portal nothing links to, as the editor sees it, with its map: what a new link may
/// name. In map order, then file order.
#[must_use]
pub fn unlinked_portals<'a>(def: &'a MapDef, data: &'a Data) -> Vec<(&'a str, &'a Portal)> {
    unlinked_among(&editor_maps(def, data))
}

/// Every portal nothing links to among the loaded maps as the files hold them, with its map.
#[must_use]
pub fn unlinked_portals_in(data: &Data) -> Vec<(&str, &Portal)> {
    let maps: Vec<Target<'_>> = data
        .maps
        .iter()
        .map(|(id, m)| Target {
            id: *id,
            def: &m.def,
            levels: &m.levels,
        })
        .collect();
    unlinked_among(&maps)
}

fn unlinked_among<'a>(maps: &[Target<'a>]) -> Vec<(&'a str, &'a Portal)> {
    let mut out = Vec::new();
    for t in maps {
        for p in &t.def.portals {
            if matches!(pairing(p, &t.def.id, maps), Ok(None)) {
                out.push((t.def.id.as_str(), p));
            }
        }
    }
    out
}

/// The maps of `pack` that `entry` does not reach through linked portals, among the maps that
/// have a linked portal at all (a map with none is unfinished, not unreached), in id order.
/// Nothing when `entry` is not among `maps`: that is reported on its own.
pub(crate) fn unreached(maps: &[Target<'_>], entry: &str, pack: &str) -> Vec<String> {
    if !maps.iter().any(|t| t.def.id == entry) {
        return Vec::new();
    }
    let mut edges: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut linked = std::collections::BTreeSet::new();
    for t in maps {
        for portal in &t.def.portals {
            if let Ok(Some((far, _))) = pairing(portal, &t.def.id, maps) {
                let (a, b) = (t.def.id.as_str(), far.def.id.as_str());
                edges.entry(a).or_default().push(b);
                edges.entry(b).or_default().push(a);
                linked.insert(a);
                linked.insert(b);
            }
        }
    }
    let mut reached = std::collections::BTreeSet::from([entry]);
    let mut queue = vec![entry];
    while let Some(id) = queue.pop() {
        for &to in edges.get(id).map(Vec::as_slice).unwrap_or_default() {
            if reached.insert(to) {
                queue.push(to);
            }
        }
    }
    maps.iter()
        .map(|t| t.def.id.as_str())
        .filter(|id| {
            !reached.contains(id) && id.split(':').next() == Some(pack) && linked.contains(id)
        })
        .map(str::to_owned)
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
        && let Some(fault) = standing_fault(def, levels, def.start_level, sx, sy)
    {
        let level = def.start_level;
        let at = if level.is_ground() {
            format!("({sx}, {sy})")
        } else {
            format!("({sx}, {sy}) on level {}", level.0)
        };
        errors.push(DataError::new(file, format!("start {at} {fault}")));
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

/// Every portal's id is its own across the loaded maps; a linked portal lands ahead of its
/// partner where the party can stand and no portal stands; an unlinked one resolves to nothing.
fn resolve_portals(
    def: &MapDef,
    file: &Path,
    targets: &dyn MapTargets,
    errors: &mut Vec<DataError>,
) -> Vec<ResolvedPortal> {
    let maps = targets.maps();
    let mut portals = Vec::new();
    for portal in &def.portals {
        for other in maps.iter().filter(|t| t.def.id != def.id) {
            if other.def.portals.iter().any(|p| p.id == portal.id) {
                errors.push(DataError::new(
                    file,
                    format!("portal id '{}' is also on '{}'", portal.id, other.def.id),
                ));
            }
        }
        match resolve_portal(portal, &def.id, &maps) {
            Ok(Some(resolved)) => portals.push(resolved),
            Ok(None) => {}
            Err(message) => errors.push(DataError::new(file, message)),
        }
    }
    portals
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
    fn maps(&self) -> Vec<Target<'_>> {
        data_maps(self.data, self.edited)
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
    /// The editor's path: `def`, the map in `file` (pack-relative, with `/`, as the session names
    /// its files), resolved against the loaded `data` exactly as the loader would resolve that
    /// file, without touching disk. Shape and layout first (the loader's per-file checks), then
    /// the references, then the other maps' portals that land on this one and this map's portals
    /// that land among their objects. `Err` carries every problem, as a load does, each naming
    /// `file` as given: a name made here with `Path::join` carried a backslash on Windows.
    pub fn resolve(def: &MapDef, file: &Path, data: &Data) -> Result<MapData, Vec<DataError>> {
        let mut errors = Vec::new();
        crate::loader::check_id(&def.id, "map", file, &mut errors);
        def.validate(file, &mut errors);
        let levels = levels::parse(def, file, &mut errors);
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
        let Some(resolved) = resolve_one(id, file, def, &levels, data, &targets, &mut errors)
        else {
            return Err(errors);
        };
        check_inbound(id, def, &resolved.centres, &targets, file, &mut errors);
        check_outbound(id, &resolved, data, file, &mut errors);
        // Reachability over sound pairs only: a link in error would otherwise cut off every map
        // beyond it and say so for each.
        if errors.is_empty() {
            check_reached(def, data, &targets, file, &mut errors);
        }
        if errors.is_empty() {
            Ok(resolved.map)
        } else {
            Err(errors)
        }
    }
}

/// The other maps' portals as this map now stands: their links still resolve (a link this map
/// held or answered may be gone), and those landing on this map still land where the party
/// can stand and no object blocks the centre. Reported against this map's file, since it is
/// the one being changed, with the other map's id first.
fn check_inbound(
    id: MapId,
    def: &MapDef,
    centres: &BTreeMap<(Level, u16, u16), String>,
    targets: &dyn MapTargets,
    file: &Path,
    errors: &mut Vec<DataError>,
) {
    let maps = targets.maps();
    for other in maps.iter().filter(|t| t.def.id != def.id) {
        for portal in &other.def.portals {
            let message = match pairing(portal, &other.def.id, &maps) {
                Err(message) => Some(message),
                Ok(Some((far, q))) if far.def.id == def.id => match landing(portal, far, q) {
                    Err(message) => Some(message),
                    Ok((to_level, to_x, to_y)) => landing_message(
                        &ResolvedPortal {
                            level: portal.level,
                            x: portal.x,
                            y: portal.y,
                            to_map: id,
                            to_level,
                            to_x,
                            to_y,
                            to_facing: q.facing,
                        },
                        centres,
                    ),
                },
                Ok(_) => None,
            };
            if let Some(message) = message {
                errors.push(DataError::new(file, format!("{}: {message}", other.def.id)));
            }
        }
    }
}

/// Every map of the pack that sets the entry is reached from the entry map through linked
/// portals, the edited map standing in for its file (the entrances and exits enhancement, W1):
/// the editor hears it when a change cuts a map off, as the loader would on the next load.
fn check_reached(
    def: &MapDef,
    data: &Data,
    targets: &dyn MapTargets,
    file: &Path,
    errors: &mut Vec<DataError>,
) {
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
    for missing in unreached(&targets.maps(), entry, &pack.id) {
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
