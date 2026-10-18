//! Tidying the world folders (M8.5 7e, owner 2026-10-18): the bases and archive segments no save
//! names any more, and the folders of worlds no save names at all. Pure: the host lists the
//! files and the saves, asks before deleting, and deletes.

use super::WORLDS;
use super::character::{CharacterSave, is_world_id};
use super::disk::{archive_path, region_path};
use crate::world::SAVE_SCHEMA;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_data::ron_io::parse;
use serde::Deserialize;

/// Every file in the world folders a save names: its manifest, its bases, its archive.
fn named(saves: &[CharacterSave]) -> BTreeSet<String> {
    let mut named = BTreeSet::new();
    for save in saves.iter().filter(|s| is_world_id(&s.world)) {
        let id = &save.world;
        named.insert(format!("{WORLDS}/{id}/manifest.ron"));
        if let Some(hash) = save.bases.world {
            named.insert(format!("{WORLDS}/{id}/world/{hash:016x}.ron"));
        }
        for (region, hash) in &save.bases.regions {
            if let Ok(path) = region_path(id, region, *hash) {
                named.insert(path);
            }
        }
        named.extend(save.archive.iter().map(|name| archive_path(id, *name)));
    }
    named
}

/// Whether `path` has the shape of a file the world folders hold: a world's manifest, a world
/// base, a region base or an archive segment, under a world's own folder.
fn ours(path: &str) -> bool {
    let Some(rest) = path.strip_prefix(WORLDS).and_then(|r| r.strip_prefix('/')) else {
        return false;
    };
    let parts: Vec<&str> = rest.split('/').collect();
    let hashed = |file: &str| {
        file.strip_suffix(".ron")
            .is_some_and(|h| h.len() == 16 && h.bytes().all(|b| b.is_ascii_hexdigit()))
    };
    match parts.as_slice() {
        [id, "manifest.ron"] => is_world_id(id),
        [id, "world" | "archive", file] => is_world_id(id) && hashed(file),
        [id, "regions", pack, region, file] => {
            is_world_id(id) && !pack.is_empty() && !region.is_empty() && hashed(file)
        }
        _ => false,
    }
}

/// Of `files` (paths relative to the host's root), the world-folder files no save in `saves`
/// names, in the order given. Anything else, however it is named, is never listed.
#[must_use]
pub fn unnamed(saves: &[CharacterSave], files: &[String]) -> Vec<String> {
    let named = named(saves);
    files
        .iter()
        .filter(|f| ours(f) && !named.contains(f.as_str()))
        .cloned()
        .collect()
}

/// Of the files under `.omnis/` (`all`, relative paths), the world-folder files no character save
/// among them names; `read` gives a file's text. A file outside the world folders that is not a
/// save of this schema is passed over, but one that says it is and does not read as one stops
/// the tidy: the files it names might be the ones listed.
///
/// # Errors
/// A file cannot be read, or a save does not read.
pub fn tidy_list(
    all: &[String],
    read: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Result<Vec<String>, String> {
    #[derive(Deserialize)]
    struct Header {
        schema: u32,
    }
    let prefix = format!("{WORLDS}/");
    let (worlds, others): (Vec<String>, Vec<String>) =
        all.iter().cloned().partition(|p| p.starts_with(&prefix));
    let mut saves = Vec::new();
    for path in &others {
        let text = read(path)?;
        if parse::<Header>(&text).is_ok_and(|h| h.schema == SAVE_SCHEMA) {
            let save = parse(&text).map_err(|e| format!("{path} does not read as a save: {e}"))?;
            saves.push(save);
        }
    }
    Ok(unnamed(&saves, &worlds))
}
