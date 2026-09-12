//! The pack loader: read every file under each pack root, validate it as untrusted input,
//! collect every error, then resolve cross-references and intern ids into a `Data` the
//! simulation can use without touching the file system again.

use crate::SCHEMA;
use crate::character::{Background, Class, Race};
use crate::condition::Condition;
use crate::content::{Content, Files, RawContent, resolve_content};
use crate::error::{DataError, LoadReport};
use crate::item::Item;
use crate::limits::{MAX_COLLECTION, MAX_PACK_BYTES, string_fits};
use crate::manifest::{PackManifest, is_content_id, is_pack_id};
use crate::map::{Cell, MapDef, Terrain};
use crate::monster::Monster;
use crate::registry::Registry;
use crate::ron_io::{from_str, read_text};
use crate::rules::RulesFile;
use crate::spell::Spell;
use crate::text::TextFile;
use crate::tileset::{SlotKind, Tileset};
use omnis_core::{
    BackgroundId, ClassId, ConditionId, Facing, ItemId, MapId, MonsterId, RaceId, SpellId, TextKey,
    TilesetId, fnv1a64,
};
use omnis_expr::Rules;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Identity of a loaded pack, stored in saves so a load with different content is detected.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PackFingerprint {
    /// Pack id.
    pub id: String,
    /// Pack version string.
    pub version: String,
    /// FNV-1a 64 over every data and text file, in path order.
    pub hash: u64,
}

/// A portal with its destination resolved to an interned map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedPortal {
    /// Trigger column.
    pub x: u16,
    /// Trigger row.
    pub y: u16,
    /// Destination map.
    pub to_map: MapId,
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
    /// Row-major cells, `width × height`.
    pub cells: Vec<Cell>,
    /// The interned tileset.
    pub tileset: TilesetId,
    /// The interned display-name key.
    pub name: TextKey,
    /// Portals with interned destinations.
    pub portals: Vec<ResolvedPortal>,
}

impl MapData {
    /// The cell at a coordinate, if inside the map.
    #[must_use]
    pub fn cell(&self, x: u16, y: u16) -> Option<&Cell> {
        if x >= self.def.width || y >= self.def.height {
            return None;
        }
        self.cells
            .get(usize::from(y) * usize::from(self.def.width) + usize::from(x))
    }

    /// The terrain of a cell.
    #[must_use]
    pub fn terrain(&self, cell: &Cell) -> &Terrain {
        &self.def.terrains[usize::from(cell.terrain)]
    }

    /// The portal on a tile, if any.
    #[must_use]
    pub fn portal_at(&self, x: u16, y: u16) -> Option<&ResolvedPortal> {
        self.portals.iter().find(|p| p.x == x && p.y == y)
    }
}

/// Everything loaded from a set of packs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Data {
    /// Manifests in load order.
    pub packs: Vec<PackManifest>,
    /// One fingerprint per pack, in load order.
    pub fingerprints: Vec<PackFingerprint>,
    /// Interned ids.
    pub registry: Registry,
    /// Tilesets by id.
    pub tilesets: BTreeMap<TilesetId, Tileset>,
    /// Maps by id.
    pub maps: BTreeMap<MapId, MapData>,
    /// Language code to text key to string.
    pub text: BTreeMap<String, BTreeMap<TextKey, String>>,
    /// Where a new game starts, from the last manifest that set `entry`.
    pub entry: Option<MapId>,
    /// Races by id.
    pub races: BTreeMap<RaceId, Race>,
    /// Classes by id.
    pub classes: BTreeMap<ClassId, Class>,
    /// Backgrounds by id.
    pub backgrounds: BTreeMap<BackgroundId, Background>,
    /// Items by id.
    pub items: BTreeMap<ItemId, Item>,
    /// Conditions by id.
    pub conditions: BTreeMap<ConditionId, Condition>,
    /// Spells by id.
    pub spells: BTreeMap<SpellId, Spell>,
    /// Monsters by id.
    pub monsters: BTreeMap<MonsterId, Monster>,
    /// The compiled rule set from every `data/rules` file.
    pub rules: Rules,
}

impl Data {
    /// A localized string, falling back to the key's name when the language lacks it.
    #[must_use]
    pub fn text(&self, lang: &str, key: TextKey) -> &str {
        self.text.get(lang).and_then(|t| t.get(&key)).map_or_else(
            || self.registry.text.name(key).unwrap_or("?"),
            String::as_str,
        )
    }

    /// A localized string by key string, for content whose keys are stored as written; the key
    /// itself when it is unknown.
    #[must_use]
    pub fn label<'a>(&'a self, lang: &str, key: &'a str) -> &'a str {
        self.registry
            .text
            .get(key)
            .map_or(key, |k| self.text(lang, k))
    }
}

/// Files gathered from the packs before resolution, keyed by content id so later packs
/// override earlier ones.
#[derive(Default)]
struct Raw {
    tilesets: Files<Tileset>,
    maps: BTreeMap<String, (PathBuf, MapDef, Vec<Cell>)>,
    text: BTreeMap<String, BTreeMap<String, String>>,
    content: RawContent,
}

/// Load packs from `roots`, in order. A pack's dependencies must appear earlier in the list.
pub fn load_packs(roots: &[&Path]) -> Result<Data, LoadReport> {
    let mut report = LoadReport::default();
    let mut raw = Raw::default();
    let mut data = Data::default();
    for root in roots {
        load_one(root, &mut raw, &mut data, &mut report.errors);
    }
    resolve(raw, &mut data, &mut report.errors);
    if report.is_empty() {
        Ok(data)
    } else {
        Err(report)
    }
}

fn load_one(root: &Path, raw: &mut Raw, data: &mut Data, errors: &mut Vec<DataError>) {
    let mut hasher = PackHasher::default();
    let manifest_rel = Path::new("pack.ron");
    let Some(manifest) = read_checked::<PackManifest>(root, manifest_rel, &mut hasher, errors)
    else {
        return;
    };
    if !is_pack_id(&manifest.id) {
        errors.push(DataError::new(
            root.join(manifest_rel),
            format!("pack id '{}' is not [a-z0-9_-]+", manifest.id),
        ));
    }
    for field in [&manifest.version, &manifest.name, &manifest.license] {
        if field.is_empty() || !string_fits(field) {
            errors.push(DataError::new(
                root.join(manifest_rel),
                "version, name, and license must be non-empty and within the size limit",
            ));
            break;
        }
    }
    for dep in &manifest.depends {
        if !data.packs.iter().any(|p| &p.id == dep) {
            errors.push(DataError::new(
                root.join(manifest_rel),
                format!("depends on '{dep}', which is not loaded before this pack"),
            ));
        }
    }
    let pack = manifest.id.clone();

    gather(
        root,
        "data/tiles",
        "tileset",
        &mut hasher,
        errors,
        &mut raw.tilesets,
    );
    for rel in list_ron(root, Path::new("data/maps"), errors) {
        let Some(map) = read_checked::<MapDef>(root, &rel, &mut hasher, errors) else {
            continue;
        };
        let file = root.join(&rel);
        check_id(&map.id, "map", &file, errors);
        map.validate(&file, errors);
        let cells = map.cells(&file, errors).unwrap_or_default();
        raw.maps.insert(map.id.clone(), (file, map, cells));
    }
    let content = &mut raw.content;
    gather(
        root,
        "data/races",
        "race",
        &mut hasher,
        errors,
        &mut content.races,
    );
    gather(
        root,
        "data/classes",
        "class",
        &mut hasher,
        errors,
        &mut content.classes,
    );
    gather(
        root,
        "data/backgrounds",
        "background",
        &mut hasher,
        errors,
        &mut content.backgrounds,
    );
    gather(
        root,
        "data/items",
        "item",
        &mut hasher,
        errors,
        &mut content.items,
    );
    gather(
        root,
        "data/conditions",
        "condition",
        &mut hasher,
        errors,
        &mut content.conditions,
    );
    gather(
        root,
        "data/spells",
        "spell",
        &mut hasher,
        errors,
        &mut content.spells,
    );
    gather(
        root,
        "data/monsters",
        "monster",
        &mut hasher,
        errors,
        &mut content.monsters,
    );
    gather(
        root,
        "data/rules",
        "rules",
        &mut hasher,
        errors,
        &mut content.rules,
    );
    for lang in list_dirs(root, Path::new("text"), errors) {
        let lang_name = lang
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_owned();
        for rel in list_ron(root, &lang, errors) {
            let Some(text) = read_checked::<TextFile>(root, &rel, &mut hasher, errors) else {
                continue;
            };
            let file = root.join(&rel);
            if text.entries.len() > MAX_COLLECTION {
                errors.push(DataError::new(&file, "too many entries"));
                continue;
            }
            let table = raw.text.entry(lang_name.clone()).or_default();
            for (key, value) in text.entries {
                check_id(&key, "text", &file, errors);
                if !string_fits(&value) {
                    errors.push(DataError::new(
                        &file,
                        format!("text '{key}' is over the size limit"),
                    ));
                }
                table.insert(key, value);
            }
        }
    }
    if hasher.bytes > MAX_PACK_BYTES {
        errors.push(DataError::new(
            root.join(manifest_rel),
            format!("pack is {} bytes; limit is {MAX_PACK_BYTES}", hasher.bytes),
        ));
    }
    data.fingerprints.push(PackFingerprint {
        id: pack,
        version: manifest.version.clone(),
        hash: hasher.finish(),
    });
    data.packs.push(manifest);
}

/// Accumulates the pack fingerprint: files are hashed in the order the loader visits them,
/// which is sorted within each directory, so the hash is a function of content alone.
#[derive(Default)]
struct PackHasher {
    entries: Vec<(String, u64)>,
    bytes: u64,
}

impl PackHasher {
    fn add(&mut self, rel: &Path, text: &str) {
        let name = rel.to_string_lossy().replace('\\', "/");
        self.entries.push((name, fnv1a64(text.as_bytes())));
        self.bytes += text.len() as u64;
    }

    fn finish(mut self) -> u64 {
        self.entries.sort();
        let mut buf = Vec::new();
        for (name, hash) in &self.entries {
            buf.extend_from_slice(name.as_bytes());
            buf.push(0);
            buf.extend_from_slice(&hash.to_le_bytes());
        }
        fnv1a64(&buf)
    }
}

/// Read, hash, parse, and check the schema of one file. `None` means an error was recorded.
fn read_checked<T: DeserializeOwned + HasSchema>(
    root: &Path,
    rel: &Path,
    hasher: &mut PackHasher,
    errors: &mut Vec<DataError>,
) -> Option<T> {
    let display = root.join(rel);
    let text = match read_text(&root.join(rel), &display) {
        Ok(t) => t,
        Err(e) => {
            errors.push(e);
            return None;
        }
    };
    hasher.add(rel, &text);
    let value: T = match from_str(&text, &display) {
        Ok(v) => v,
        Err(e) => {
            errors.push(e);
            return None;
        }
    };
    if value.schema() != SCHEMA {
        // Migrations `vN -> vN+1` register here as schema versions accumulate.
        errors.push(DataError::new(
            display,
            format!(
                "schema {} is not supported; this build reads schema {SCHEMA}",
                value.schema()
            ),
        ));
        return None;
    }
    Some(value)
}

/// Read, check, and validate every `.ron` file of one content type under `root/dir`, keyed
/// by id so a later pack overrides an earlier one.
fn gather<T: DeserializeOwned + HasSchema + Content>(
    root: &Path,
    dir: &str,
    kind: &str,
    hasher: &mut PackHasher,
    errors: &mut Vec<DataError>,
    out: &mut Files<T>,
) {
    for rel in list_ron(root, Path::new(dir), errors) {
        let Some(value) = read_checked::<T>(root, &rel, hasher, errors) else {
            continue;
        };
        let file = root.join(&rel);
        check_id(value.id(), kind, &file, errors);
        value.validate(&file, errors);
        out.insert(value.id().to_owned(), (file, value));
    }
}

trait HasSchema {
    fn schema(&self) -> u32;
}
macro_rules! has_schema {
    ($($t:ty),* $(,)?) => {$(
        impl HasSchema for $t {
            fn schema(&self) -> u32 {
                self.schema
            }
        }
    )*};
}
has_schema!(
    PackManifest,
    Tileset,
    MapDef,
    TextFile,
    Race,
    Class,
    Background,
    Item,
    Condition,
    Spell,
    Monster,
    RulesFile,
);

fn check_id(id: &str, kind: &str, file: &Path, errors: &mut Vec<DataError>) {
    if !is_content_id(id) {
        errors.push(DataError::new(
            file,
            format!("id '{id}' is not of the form pack:{kind}:name"),
        ));
    } else if id.split(':').nth(1) != Some(kind) {
        errors.push(DataError::new(
            file,
            format!("id '{id}' must have type '{kind}'"),
        ));
    }
}

/// `.ron` files directly under `root/dir`, sorted, as paths relative to `root`. A missing
/// directory is empty, not an error.
fn list_ron(root: &Path, dir: &Path, errors: &mut Vec<DataError>) -> Vec<PathBuf> {
    list_entries(root, dir, errors, |entry| {
        entry.is_file() && entry.extension().is_some_and(|e| e == "ron")
    })
}

fn list_dirs(root: &Path, dir: &Path, errors: &mut Vec<DataError>) -> Vec<PathBuf> {
    list_entries(root, dir, errors, Path::is_dir)
}

fn list_entries(
    root: &Path,
    dir: &Path,
    errors: &mut Vec<DataError>,
    keep: fn(&Path) -> bool,
) -> Vec<PathBuf> {
    let full = root.join(dir);
    if !full.exists() {
        return Vec::new();
    }
    let read = match std::fs::read_dir(&full) {
        Ok(r) => r,
        Err(e) => {
            errors.push(DataError::new(full, format!("cannot list: {e}")));
            return Vec::new();
        }
    };
    let mut out: Vec<PathBuf> = read
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| !t.is_symlink()))
        .filter(|e| keep(&e.path()))
        .map(|e| dir.join(e.file_name()))
        .collect();
    out.sort();
    out
}

/// Intern ids and check every cross-reference. Runs once after all packs are read.
fn resolve(raw: Raw, data: &mut Data, errors: &mut Vec<DataError>) {
    let before = errors.len();
    for (id, (_, tileset)) in raw.tilesets {
        let tid = data.registry.tilesets.intern(&id);
        data.tilesets.insert(tid, tileset);
    }
    for (lang, table) in raw.text {
        let mut interned = BTreeMap::new();
        for (key, value) in table {
            interned.insert(data.registry.text.intern(&key), value);
        }
        data.text.insert(lang, interned);
    }
    // Two passes so portals can refer to maps defined later in the order.
    let map_ids: BTreeMap<&str, MapId> = raw
        .maps
        .keys()
        .map(|id| (id.as_str(), data.registry.maps.intern(id)))
        .collect();
    for (id, (file, def, cells)) in &raw.maps {
        let Some(tileset_id) = data.registry.tilesets.get(&def.tileset) else {
            errors.push(DataError::new(
                file,
                format!(
                    "tileset '{}' is not defined by any loaded pack",
                    def.tileset
                ),
            ));
            continue;
        };
        check_surfaces(def, &data.tilesets[&tileset_id], file, errors);
        let name = match data.registry.text.get(&def.name) {
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
        let portals = resolve_portals(def, file, &map_ids, &raw.maps, errors);
        if errors.len() == before {
            data.maps.insert(
                map_ids[id.as_str()],
                MapData {
                    def: def.clone(),
                    cells: cells.clone(),
                    tileset: tileset_id,
                    name,
                    portals,
                },
            );
        }
    }
    if let Some(entry) = data.packs.iter().rev().find_map(|p| p.entry.as_deref()) {
        match data.registry.maps.get(entry) {
            Some(id) => data.entry = Some(id),
            None => errors.push(DataError::new(
                "pack.ron",
                format!("entry map '{entry}' is not defined by any loaded pack"),
            )),
        }
    }
    resolve_content(raw.content, data, errors);
    if errors.len() > before {
        data.maps.clear();
    }
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
}

/// Portals must lead to a known map and land inside it.
fn resolve_portals(
    def: &MapDef,
    file: &Path,
    map_ids: &BTreeMap<&str, MapId>,
    maps: &BTreeMap<String, (PathBuf, MapDef, Vec<Cell>)>,
    errors: &mut Vec<DataError>,
) -> Vec<ResolvedPortal> {
    let mut portals = Vec::with_capacity(def.portals.len());
    for portal in &def.portals {
        let Some(&to_map) = map_ids.get(portal.to_map.as_str()) else {
            errors.push(DataError::new(
                file,
                format!(
                    "portal at ({}, {}) leads to unknown map '{}'",
                    portal.x, portal.y, portal.to_map
                ),
            ));
            continue;
        };
        let (_, target, _) = &maps[&portal.to_map];
        if portal.to_x >= target.width || portal.to_y >= target.height {
            errors.push(DataError::new(
                file,
                format!(
                    "portal at ({}, {}) lands outside map '{}'",
                    portal.x, portal.y, portal.to_map
                ),
            ));
        }
        portals.push(ResolvedPortal {
            x: portal.x,
            y: portal.y,
            to_map,
            to_x: portal.to_x,
            to_y: portal.to_y,
            to_facing: portal.to_facing,
        });
    }
    portals
}
