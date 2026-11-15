//! The author's notes of one pack as the editor holds them: every `notes/*.ron` file, read
//! once, written back one file at a time as a note changes. A note hangs on a key (`notes.rs`);
//! which file holds it is the store's business: a map's notes in `notes/<map name>.ron`, any
//! other kind's together in `notes/<kind>.ron`. Never sealed, so a note changes no fingerprint.

use crate::edit::document::Document;
use crate::edit::kinds::EditError;
use crate::limits::{MAX_COLLECTION, string_fits};
use crate::loader::Data;
use crate::map::MapDef;
use crate::notes::{NotesFile, is_note_key};
use crate::ron_io::{list_under, read_under, remove_under, to_string, write_files};
use omnis_core::Level;
use std::collections::BTreeMap;
use std::path::Path;

/// Every notes file of a pack, by root-relative path.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Notes {
    files: BTreeMap<String, NotesFile>,
}

impl Notes {
    /// The notes under `root/notes`.
    ///
    /// # Errors
    /// A file that cannot be read or does not parse (`Io`).
    pub fn read(root: &Path) -> Result<Notes, EditError> {
        let mut files = BTreeMap::new();
        for rel in list_under(root, "notes")? {
            let text = read_under(root, &rel)?;
            let file: NotesFile = crate::ron_io::from_str(&text, Path::new(&rel))?;
            files.insert(rel, file);
        }
        Ok(Notes { files })
    }

    /// The note on `key`, if any.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.files
            .values()
            .find_map(|f| f.entries.get(key))
            .map(String::as_str)
    }

    /// Every note whose key starts with `prefix` (a map id lists the map's), sorted by key.
    #[must_use]
    pub fn with_prefix(&self, prefix: &str) -> Vec<(&str, &str)> {
        let mut out: Vec<(&str, &str)> = self
            .files
            .values()
            .flat_map(|f| f.entries.iter())
            .filter(|(k, _)| k.starts_with(prefix))
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        out.sort_unstable();
        out
    }

    /// The notes written on cells of `map`: each key with its level and cell.
    #[must_use]
    pub fn on_cells(&self, map: &str) -> Vec<(String, i8, (u16, u16))> {
        let prefix = format!("{map}/");
        let notes = self.with_prefix(&prefix).into_iter();
        notes
            .filter_map(|(key, _)| {
                let (level, cell) = key.strip_prefix(&prefix)?.split_once('/')?;
                let (x, y) = cell.split_once(',')?;
                let at = (x.parse().ok()?, y.parse().ok()?);
                Some((key.to_owned(), level.parse().ok()?, at))
            })
            .collect()
    }

    /// Move the notes written on cells of `map` as its cells moved: `to` gives a cell's new
    /// place, or `None` for a cell that is gone, whose note goes with it. Notes on cells the
    /// map never had (`was` says which it had) are left as they are.
    ///
    /// # Errors
    /// The store's, as for [`Notes::set`].
    pub fn follow(
        &mut self,
        root: &Path,
        map: &str,
        was: impl Fn((u16, u16)) -> bool,
        to: impl Fn((u16, u16)) -> Option<(u16, u16)>,
    ) -> Result<(), EditError> {
        let moving: Vec<_> = self
            .on_cells(map)
            .into_iter()
            .filter(|(_, _, at)| was(*at) && to(*at) != Some(*at))
            .collect();
        let texts: Vec<Option<String>> = moving
            .iter()
            .map(|(key, _, _)| self.get(key).map(str::to_owned))
            .collect();
        for (key, _, _) in &moving {
            self.set(root, key, None)?;
        }
        for ((_, level, at), text) in moving.iter().zip(texts) {
            if let (Some((x, y)), Some(text)) = (to(*at), text) {
                self.set(root, &format!("{map}/{level}/{x},{y}"), Some(&text))?;
            }
        }
        Ok(())
    }

    /// How many notes there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.values().map(|f| f.entries.len()).sum()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The file a new note on `key` goes to.
    fn file_for(key: &str) -> String {
        let id = key.split('/').next().unwrap_or(key);
        let mut parts = id.split(':');
        let (_, kind, name) = (parts.next(), parts.next().unwrap_or("notes"), parts.next());
        match (kind, name) {
            ("map", Some(name)) => format!("notes/{name}.ron"),
            (kind, _) => format!("notes/{kind}.ron"),
        }
    }

    /// Write `text` as the note on `key`, or take the note away with `None`; the file that
    /// holds it is written at once.
    ///
    /// # Errors
    /// `BadId` for a key that names nothing; `OutOfRange` for a text over the string limit;
    /// `TooMany` when the file is full; `Io` when it cannot be written.
    pub fn set(&mut self, root: &Path, key: &str, text: Option<&str>) -> Result<(), EditError> {
        if !is_note_key(key) {
            return Err(EditError::BadId(key.to_owned()));
        }
        let rel = self
            .files
            .iter()
            .find(|(_, f)| f.entries.contains_key(key))
            .map(|(rel, _)| rel.clone())
            .unwrap_or_else(|| Self::file_for(key));
        let file = self.files.entry(rel.clone()).or_insert_with(|| NotesFile {
            schema: crate::SCHEMA,
            entries: BTreeMap::new(),
        });
        match text {
            Some(text) => {
                if !string_fits(text) {
                    return Err(EditError::OutOfRange);
                }
                if !file.entries.contains_key(key) && file.entries.len() >= MAX_COLLECTION {
                    return Err(EditError::TooMany("notes in one file"));
                }
                file.entries.insert(key.to_owned(), text.to_owned());
            }
            None => {
                file.entries.remove(key);
            }
        }
        if file.entries.is_empty() {
            self.files.remove(&rel);
            if root.join(&rel).is_file() {
                remove_under(root, &rel)?;
            }
        } else {
            let written = to_string(file)?;
            write_files(root, &[(&rel, &written, false)])?;
        }
        Ok(())
    }

    /// The keys whose target is not in the loaded packs, nor in the open document when the key
    /// is its map's: what a game master wrote about before it was built, or after it went.
    #[must_use]
    pub fn orphans(&self, data: &Data, document: Option<&Document>) -> Vec<String> {
        let open = document.map(Document::def);
        let mut out: Vec<String> = self
            .files
            .values()
            .flat_map(|f| f.entries.keys())
            .filter(|key| !target_exists(key, data, open.as_ref()))
            .cloned()
            .collect();
        out.sort_unstable();
        out
    }
}

/// Whether `key` names something that exists.
fn target_exists(key: &str, data: &Data, open: Option<&MapDef>) -> bool {
    let (id, path) = match key.split_once('/') {
        Some((id, path)) => (id, Some(path)),
        None => (key, None),
    };
    let kind = id.split(':').nth(1).unwrap_or_default();
    if kind == "map" {
        let def = match open {
            Some(def) if def.id == id => Some(def.clone()),
            _ => data
                .registry
                .maps
                .get(id)
                .and_then(|m| data.maps.get(&m))
                .map(|m| m.def.clone()),
        };
        return def.is_some_and(|def| path.is_none_or(|p| in_map(&def, p)));
    }
    if path.is_some() {
        return false;
    }
    let r = &data.registry;
    match kind {
        "tileset" => r.tilesets.get(id).is_some(),
        "text" => r.text.get(id).is_some(),
        "flag" => r.flags.get(id).is_some(),
        "biosphere" => data.biospheres.contains_key(id),
        "race" => r.races.get(id).is_some(),
        "class" => r.classes.get(id).is_some(),
        "background" => r.backgrounds.get(id).is_some(),
        "item" => r.items.get(id).is_some(),
        "condition" => r.conditions.get(id).is_some(),
        "spell" => r.spells.get(id).is_some(),
        "monster" => r.monsters.get(id).is_some(),
        "service" => r.services.get(id).is_some(),
        "region" => r.regions.get(id).is_some(),
        "object" => r.objects.get(id).is_some(),
        "loot" => r.loot.get(id).is_some(),
        _ => false,
    }
}

/// Whether a path inside a map names something in it: `level/N`, `N/x,y`, `terrain/g`, or a
/// placement id.
fn in_map(def: &MapDef, path: &str) -> bool {
    let has_level = |l: i8| l == 0 || def.levels.iter().any(|d| d.level == l);
    if let Some(level) = path.strip_prefix("level/") {
        return level.parse::<i8>().is_ok_and(has_level);
    }
    if let Some(glyph) = path.strip_prefix("terrain/") {
        let mut chars = glyph.chars();
        return matches!((chars.next(), chars.next()), (Some(g), None) if def.terrains.iter().any(|t| t.glyph == g));
    }
    if let Some((level, cell)) = path.split_once('/')
        && let Some((x, y)) = cell.split_once(',')
        && let (Ok(level), Ok(x), Ok(y)) = (level.parse::<i8>(), x.parse::<u16>(), y.parse::<u16>())
    {
        let _ = Level(level);
        return has_level(level) && x < def.width && y < def.height;
    }
    def.objects.iter().any(|o| o.id == path)
        || def.encounters.iter().any(|e| e.id == path)
        || def.lore.iter().any(|l| l.id == path)
}
