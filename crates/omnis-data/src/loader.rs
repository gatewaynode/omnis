//! The pack loader: read every file under each pack root, validate it as untrusted input,
//! collect every error, then resolve cross-references and intern ids into a `Data` the
//! simulation can use without touching the file system again.

use crate::SCHEMA;
use crate::character::{Background, Class, Race};
use crate::condition::Condition;
use crate::content::{Content, Files, RawContent, resolve_content};
use crate::error::{DataError, LoadReport};
use crate::flag::{FlagDef, FlagScope};
use crate::item::Item;
use crate::levels::{self, LevelData};
use crate::limits::{MAX_COLLECTION, MAX_PACK_BYTES, string_fits};
use crate::loot::LootTable;
use crate::manifest::{PackManifest, is_content_id, is_pack_id};
use crate::map::MapDef;
use crate::map_data::{self, MapData, ResolvedPortal};
use crate::migration::{self, Migration, Migrations, PackMigration};
use crate::monster::Monster;
use crate::notes::NotesFile;
use crate::object::ObjectKind;
use crate::region::{self, Region, RegionDef};
use crate::registry::Registry;
use crate::ron_io::{from_str, read_text};
use crate::rules::RulesFile;
use crate::service::ServiceDef;
use crate::spell::Spell;
use crate::text::TextFile;
use crate::tileset::Tileset;
use omnis_core::{
    BackgroundId, Calendar, ClassId, ConditionId, FlagId, ItemId, Level, LootTableId, MapId,
    MonsterId, ObjectKindId, RaceId, RegionId, ServiceId, SpellId, TextKey, TilesetId, fnv1a64,
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
    /// Services by id.
    pub services: BTreeMap<ServiceId, ServiceDef>,
    /// The compiled rule set from every `data/rules` file.
    pub rules: Rules,
    /// Regions by id.
    pub regions: BTreeMap<RegionId, Region>,
    /// Object kinds by id.
    pub objects: BTreeMap<ObjectKindId, ObjectKind>,
    /// Loot tables by id.
    pub loot: BTreeMap<LootTableId, LootTable>,
    /// Flags a pack declared, by id; an undeclared flag is the world's.
    pub flags: BTreeMap<FlagId, FlagDef>,
    /// Each pack's migrations, by pack id, in file order (M8.5 7h6).
    pub migrations: Migrations,
    /// The packs that wrote each definition, by its id, in load order: the last supplied the one
    /// loaded, the others were replaced by it (the catalogue's `pack` and `replaces`).
    pub sources: BTreeMap<String, Vec<String>>,
}

impl Data {
    /// Whose `flag` is: as its file declares, else the world's (owner, 2026-10-17).
    #[must_use]
    pub fn flag_scope(&self, flag: FlagId) -> FlagScope {
        self.flags.get(&flag).map_or(FlagScope::World, |f| f.scope)
    }

    /// The calendar's shape from `rules/time.ron`, each value the default when it is missing.
    #[must_use]
    pub fn calendar(&self) -> Calendar {
        let base = Calendar::default();
        let value = |name: &str, default: u32| {
            self.rules
                .value(name)
                .and_then(|v| u32::try_from(v).ok())
                .unwrap_or(default)
        };
        Calendar {
            minutes_per_day: value("minutes_per_day", base.minutes_per_day),
            days_per_year: value("days_per_year", base.days_per_year),
            night_from: value("night_from", base.night_from),
            night_to: value("night_to", base.night_to),
        }
    }

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
    maps: BTreeMap<String, (PathBuf, MapDef, Vec<LevelData>)>,
    text: BTreeMap<String, BTreeMap<String, String>>,
    content: RawContent,
    /// Every pack's migrations, checked against the loaded packs once all are read.
    migrations: Vec<(PathBuf, String, PackMigration)>,
    /// The packs that wrote each definition, in load order.
    sources: BTreeMap<String, Vec<String>>,
}

/// Load packs from `roots`, in order. A pack's dependencies must appear earlier in the list.
pub fn load_packs(roots: &[&Path]) -> Result<Data, LoadReport> {
    let mut report = LoadReport::default();
    let mut raw = Raw::default();
    let mut data = Data::default();
    for root in roots {
        load_one(root, &mut raw, &mut data, &mut report.errors, true);
    }
    resolve(raw, &mut data, &mut report.errors);
    if report.is_empty() {
        Ok(data)
    } else {
        Err(report)
    }
}

/// The manifest of the pack at `root` and the hash of its content, whatever else is wrong with
/// it: what `seal` records. Only a manifest that does not read is an error.
pub(crate) fn content_hash(root: &Path) -> Result<(PackManifest, u64), DataError> {
    let mut errors = Vec::new();
    let mut data = Data::default();
    load_one(root, &mut Raw::default(), &mut data, &mut errors, false);
    match (data.packs.pop(), data.fingerprints.pop()) {
        (Some(manifest), Some(print)) => Ok((manifest, print.hash)),
        _ => Err(errors
            .into_iter()
            .next()
            .unwrap_or_else(|| DataError::new(root.join("pack.ron"), "no manifest"))),
    }
}

/// Load one pack into `raw` and `data`. `sealed`: refuse it unless its manifest records the
/// hash of its content (`content_hash` passes false, to compute it).
fn load_one(
    root: &Path,
    raw: &mut Raw,
    data: &mut Data,
    errors: &mut Vec<DataError>,
    sealed: bool,
) {
    let mut hasher = PackHasher::default();
    let manifest_rel = Path::new("pack.ron");
    let Some(manifest) =
        read_checked::<PackManifest>(root, manifest_rel, &mut PackHasher::default(), errors)
    else {
        return;
    };
    // The manifest counts as content by its fields, not its layout, and without its own hash.
    let unsealed = PackManifest {
        hash: 0,
        ..manifest.clone()
    };
    match crate::ron_io::to_string(&unsealed) {
        Ok(text) => hasher.add(manifest_rel, &text),
        Err(e) => errors.push(e),
    }
    check_manifest(&manifest, &root.join(manifest_rel), &data.packs, errors);
    let pack = manifest.id.clone();
    let mut supplied = Vec::new();

    gather(
        root,
        "data/tiles",
        "tileset",
        &mut hasher,
        errors,
        &mut raw.tilesets,
        &mut supplied,
    );
    for rel in list_ron(root, Path::new("data/maps"), errors) {
        let Some(map) = read_checked::<MapDef>(root, &rel, &mut hasher, errors) else {
            continue;
        };
        let file = root.join(&rel);
        check_id(&map.id, "map", &file, errors);
        map.validate(&file, errors);
        let levels = levels::parse(&map, &file, errors).unwrap_or_default();
        supplied.push(map.id.clone());
        raw.maps.insert(map.id.clone(), (file, map, levels));
    }
    gather_content(root, &mut hasher, errors, &mut raw.content, &mut supplied);
    for id in supplied {
        let packs = raw.sources.entry(id).or_default();
        if packs.last() != Some(&pack) {
            packs.push(pack.clone());
        }
    }
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
            let table = raw.text.entry(lang_name.clone()).or_default();
            check_text_file(text, &file, table, errors);
        }
    }
    // Author notes are checked like any file but never hashed: they never change play (M5.1).
    for rel in list_ron(root, Path::new("notes"), errors) {
        if let Some(notes) =
            read_checked::<NotesFile>(root, &rel, &mut PackHasher::default(), errors)
        {
            notes.validate(&root.join(&rel), errors);
        }
    }
    let mut migrations = Vec::new();
    for rel in list_ron(root, Path::new("migrations"), errors) {
        if let Some(m) = read_checked::<Migration>(root, &rel, &mut hasher, errors) {
            let file = migration::file_name(&rel);
            let loaded = PackMigration {
                file,
                to: m.to,
                steps: m.steps,
            };
            migrations.push((root.join(&rel), loaded));
        }
    }
    migration::check_pack(&pack, &manifest.version, &migrations, errors);
    raw.migrations.extend(
        migrations
            .into_iter()
            .map(|(path, m)| (path, pack.clone(), m)),
    );
    if hasher.bytes > MAX_PACK_BYTES {
        errors.push(DataError::new(
            root.join(manifest_rel),
            format!("pack is {} bytes; limit is {MAX_PACK_BYTES}", hasher.bytes),
        ));
    }
    let hash = hasher.finish();
    if sealed {
        let file = root.join(manifest_rel);
        match manifest.hash {
            0 => errors.push(DataError::new(
                file,
                "the pack is not sealed: record its hash with `omnis-cli pack seal`",
            )),
            recorded if recorded != hash => errors.push(DataError::new(
                file,
                format!(
                    "the content changed since version {} was sealed: raise the version and \
                     reseal with `omnis-cli pack seal`",
                    manifest.version
                ),
            )),
            _ => {}
        }
    }
    data.fingerprints.push(PackFingerprint {
        id: pack,
        version: manifest.version.clone(),
        hash,
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

/// The manifest's own fields: the id's shape, the strings that must be there and fit, and the
/// dependencies, each a pack loaded before this one.
pub(crate) fn check_manifest(
    manifest: &PackManifest,
    file: &Path,
    loaded: &[PackManifest],
    errors: &mut Vec<DataError>,
) {
    if !is_pack_id(&manifest.id) {
        errors.push(DataError::new(
            file,
            format!("pack id '{}' is not [a-z0-9_-]+", manifest.id),
        ));
    }
    for field in [&manifest.version, &manifest.name, &manifest.license] {
        if field.is_empty() || !string_fits(field) {
            errors.push(DataError::new(
                file,
                "version, name, and license must be non-empty and within the size limit",
            ));
            break;
        }
    }
    for dep in &manifest.depends {
        if !loaded.iter().any(|p| &p.id == dep) {
            errors.push(DataError::new(
                file,
                format!("depends on '{dep}', which is not loaded before this pack"),
            ));
        }
    }
}

/// One text file's entries into `table`: their count, each key's shape, each value's size.
pub(crate) fn check_text_file(
    text: TextFile,
    file: &Path,
    table: &mut BTreeMap<String, String>,
    errors: &mut Vec<DataError>,
) {
    if text.entries.len() > MAX_COLLECTION {
        errors.push(DataError::new(file, "too many entries"));
        return;
    }
    for (key, value) in text.entries {
        check_id(&key, "text", file, errors);
        if !string_fits(&value) {
            errors.push(DataError::new(
                file,
                format!("text '{key}' is over the size limit"),
            ));
        }
        table.insert(key, value);
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
    let text = match read_text(&display, &display) {
        Ok(t) => t,
        Err(e) => {
            errors.push(e);
            return None;
        }
    };
    hasher.add(rel, &text);
    parse_checked(&text, &display, errors)
}

/// Parse `text` as `T` and check its schema; `display` names the file in errors. `None` means
/// an error was recorded. The loader's files and the editor's typed text both pass here.
pub(crate) fn parse_checked<T: DeserializeOwned + HasSchema>(
    text: &str,
    display: &Path,
    errors: &mut Vec<DataError>,
) -> Option<T> {
    let value: T = match from_str(text, display) {
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
/// by id so a later pack overrides an earlier one. `supplied` gains each id this pack wrote.
fn gather<T: DeserializeOwned + HasSchema + Content>(
    root: &Path,
    dir: &str,
    kind: &str,
    hasher: &mut PackHasher,
    errors: &mut Vec<DataError>,
    out: &mut Files<T>,
    supplied: &mut Vec<String>,
) {
    for rel in list_ron(root, Path::new(dir), errors) {
        let Some(value) = read_checked::<T>(root, &rel, hasher, errors) else {
            continue;
        };
        let file = root.join(&rel);
        check_id(value.id(), kind, &file, errors);
        value.validate(&file, errors);
        supplied.push(value.id().to_owned());
        out.insert(value.id().to_owned(), (file, value));
    }
}

/// Every content type besides tiles, maps, and text, one directory each.
fn gather_content(
    root: &Path,
    hasher: &mut PackHasher,
    errors: &mut Vec<DataError>,
    content: &mut RawContent,
    supplied: &mut Vec<String>,
) {
    gather(
        root,
        "data/races",
        "race",
        hasher,
        errors,
        &mut content.races,
        supplied,
    );
    gather(
        root,
        "data/classes",
        "class",
        hasher,
        errors,
        &mut content.classes,
        supplied,
    );
    gather(
        root,
        "data/backgrounds",
        "background",
        hasher,
        errors,
        &mut content.backgrounds,
        supplied,
    );
    gather(
        root,
        "data/items",
        "item",
        hasher,
        errors,
        &mut content.items,
        supplied,
    );
    gather(
        root,
        "data/conditions",
        "condition",
        hasher,
        errors,
        &mut content.conditions,
        supplied,
    );
    gather(
        root,
        "data/spells",
        "spell",
        hasher,
        errors,
        &mut content.spells,
        supplied,
    );
    gather(
        root,
        "data/monsters",
        "monster",
        hasher,
        errors,
        &mut content.monsters,
        supplied,
    );
    gather(
        root,
        "data/rules",
        "rules",
        hasher,
        errors,
        &mut content.rules,
        supplied,
    );
    gather(
        root,
        "data/services",
        "service",
        hasher,
        errors,
        &mut content.services,
        supplied,
    );
    gather(
        root,
        "data/regions",
        "region",
        hasher,
        errors,
        &mut content.regions,
        supplied,
    );
    gather(
        root,
        "data/objects",
        "object",
        hasher,
        errors,
        &mut content.objects,
        supplied,
    );
    gather(
        root,
        "data/loot",
        "loot",
        hasher,
        errors,
        &mut content.loot,
        supplied,
    );
    gather(
        root,
        "data/flags",
        "flag",
        hasher,
        errors,
        &mut content.flags,
        supplied,
    );
}

pub(crate) trait HasSchema {
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
    NotesFile,
    Race,
    Class,
    Background,
    Item,
    Condition,
    Spell,
    Monster,
    RulesFile,
    ServiceDef,
    RegionDef,
    ObjectKind,
    LootTable,
    FlagDef,
    Migration,
);

pub(crate) fn check_id(id: &str, kind: &str, file: &Path, errors: &mut Vec<DataError>) {
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
fn resolve(mut raw: Raw, data: &mut Data, errors: &mut Vec<DataError>) {
    let before = errors.len();
    data.sources = std::mem::take(&mut raw.sources);
    let region_files = std::mem::take(&mut raw.content.regions);
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
    // Content first, so encounters can name the monsters it interns.
    resolve_content(raw.content, data, errors);
    // Two passes so portals can refer to maps defined later in the order.
    let map_ids: BTreeMap<&str, MapId> = raw
        .maps
        .keys()
        .map(|id| (id.as_str(), data.registry.maps.intern(id)))
        .collect();
    let (regions, owners) = region::resolve(
        region_files,
        &map_ids,
        &mut data.registry.regions,
        &data.registry.text,
        &data.rules,
        errors,
    );
    data.regions = regions;
    check_calendar(&data.rules, errors);
    let targets = RawTargets {
        ids: &map_ids,
        maps: &raw.maps,
        owners: &owners,
    };
    let mut centres = BTreeMap::new();
    let mut landings = Vec::new();
    for (id, (file, def, levels)) in &raw.maps {
        let map_id = map_ids[id.as_str()];
        let Some(resolved) =
            map_data::resolve_one(map_id, file, def, levels, data, &targets, errors)
        else {
            continue;
        };
        centres.insert(map_id, resolved.centres);
        landings.push((file.clone(), resolved.map.portals.clone()));
        if errors.len() == before {
            data.maps.insert(map_id, resolved.map);
        }
    }
    check_landings(&landings, &centres, errors);
    if let Some(pack) = data.packs.iter().rev().find(|p| p.entry.is_some()) {
        let entry = pack.entry.as_deref().unwrap_or_default();
        match data.registry.maps.get(entry) {
            Some(id) => data.entry = Some(id),
            None => errors.push(DataError::new(
                "pack.ron",
                format!("entry map '{entry}' is not defined by any loaded pack"),
            )),
        }
        // The entrances and exits enhancement (W1): the world pack's maps are all reached, asked
        // only of a world whose links are sound (an erring link would cut off every map beyond).
        let maps = map_data::MapTargets::maps(&targets);
        let sound = errors.len() == before;
        for missing in map_data::unreached(&maps, entry, &pack.id)
            .into_iter()
            .filter(|_| sound)
        {
            let file = raw
                .maps
                .get(&missing)
                .map(|(f, _, _)| f.clone())
                .unwrap_or_default();
            errors.push(DataError::new(
                file,
                map_data::unreached_message(&missing, entry),
            ));
        }
    }
    if errors.len() == before {
        migration::check_loaded(&raw.migrations, data, errors);
    }
    for (_, pack, m) in raw.migrations {
        data.migrations.entry(pack).or_default().push(m);
    }
    if errors.len() > before {
        data.maps.clear();
    }
}

/// What the files read answer about the other maps while they resolve.
struct RawTargets<'a> {
    ids: &'a BTreeMap<&'a str, MapId>,
    maps: &'a BTreeMap<String, (PathBuf, MapDef, Vec<LevelData>)>,
    owners: &'a BTreeMap<MapId, RegionId>,
}

impl map_data::MapTargets for RawTargets<'_> {
    fn maps(&self) -> Vec<map_data::Target<'_>> {
        self.maps
            .iter()
            .filter_map(|(id, (_, def, levels))| {
                Some(map_data::Target {
                    id: *self.ids.get(id.as_str())?,
                    def,
                    levels,
                })
            })
            .collect()
    }

    fn owner(&self, map: MapId) -> Option<RegionId> {
        self.owners.get(&map).copied()
    }
}

/// No portal lands the party where an object stands across the centre of the cell.
fn check_landings(
    portals: &[(PathBuf, Vec<ResolvedPortal>)],
    centres: &BTreeMap<MapId, BTreeMap<(Level, u16, u16), String>>,
    errors: &mut Vec<DataError>,
) {
    for (file, list) in portals {
        for portal in list {
            if let Some(map) = centres.get(&portal.to_map)
                && let Some(message) = map_data::landing_message(portal, map)
            {
                errors.push(DataError::new(file, message));
            }
        }
    }
}

/// The calendar's values, when given, must make a calendar: a day and a year of at least one,
/// night inside the day, and a reconciliation cap that is not negative.
fn check_calendar(rules: &Rules, errors: &mut Vec<DataError>) {
    let day = rules.value("minutes_per_day").unwrap_or(1440);
    let checks = [
        ("minutes_per_day", 1, i64::from(u16::MAX)),
        ("days_per_year", 1, i64::from(u16::MAX)),
        ("night_from", 0, day - 1),
        ("night_to", 0, day - 1),
        ("time_cap_minutes", 0, i64::MAX),
    ];
    for (name, low, high) in checks {
        if let Some(v) = rules.value(name)
            && !(low..=high).contains(&v)
        {
            errors.push(DataError::new(
                "data/rules",
                format!("value '{name}' is {v}; it must be {low}..={high}"),
            ));
        }
    }
}
