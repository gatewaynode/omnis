//! The files of a save (PRD D28, M8.5 7d), as text and relative paths; the host writes and reads
//! them. A world folder `.omnis/worlds/<id>/` holds the manifest and the bases, each base named
//! by the hash of its text; the character save names the folder and its bases, and holds the
//! party as it stands and the journal of the world's changes since. A load lays the changes over
//! the bases and runs no rule. A schema 1 to 7 save, a whole world, is converted on load: the
//! world becomes the base of an empty journal.

use super::character::{BaseNames, CharacterSave, is_world_id, world_id};
use super::{Archived, Bases, Manifest, RegionBase, Split, WorldBase, join, split};
use crate::journal::apply_changes;
use crate::ops::OpError;
use crate::save::JoinError;
use crate::world::{LoadError, SAVE_SCHEMA, World};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use omnis_core::fnv1a64;
use omnis_data::Data;
use omnis_data::limits::{MAX_FILE_BYTES, MAX_SAVE_BYTES};
use omnis_data::ron_io::{parse, to_compact, to_string};
use serde::Deserialize;

/// Where the world folders are, relative to the host's root.
pub const WORLDS: &str = ".omnis/worlds";

/// One file to write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOut {
    /// Relative to the host's root.
    pub path: String,
    /// Its text.
    pub text: String,
    /// A file already there may stay: its name is its content's hash, or its folder's.
    pub keep: bool,
}

/// What a load did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Restored {
    /// Interactions of the save's journal whose changes were laid over the bases.
    pub applied: usize,
    /// The schema of an older save made the base of a new journal.
    pub converted_from: Option<u32>,
}

/// What a new game on the world's seed and settings holds: base 0.
fn base_zero(data: &Data, manifest: &Manifest, world: &World) -> Result<Bases, String> {
    let mut fresh = World::new(data, manifest.seed, world.settings).map_err(|e| e.to_string())?;
    fresh.packs.clone_from(&manifest.packs);
    Ok(split(&fresh, data).into())
}

/// The path of a region's base: its string id's pack and name as folders.
pub(crate) fn region_path(id: &str, name: &str, hash: u64) -> Result<String, String> {
    let mut parts = name.split(':');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(pack), Some("region"), Some(region), None) => Ok(format!(
            "{WORLDS}/{id}/regions/{pack}/{region}/{hash:016x}.ron"
        )),
        _ => Err(format!("region '{name}' is not a region id")),
    }
}

/// The path of an archive segment in the world folder `id`.
#[must_use]
pub fn archive_path(id: &str, name: u64) -> String {
    format!("{WORLDS}/{id}/archive/{name:016x}.ron")
}

/// The files that save `world` at `save_path`, in the order to write them: the bases its journal
/// starts from, the archive segments folded in this session, the world's manifest, then the
/// character save, so a save never names a file that is not already on disk.
pub fn save_files(world: &World, data: &Data, save_path: &str) -> Result<Vec<FileOut>, OpError> {
    if !world.may_save() {
        return Err(OpError::failed("the save rule forbids saving here"));
    }
    let failed = |e: &dyn core::fmt::Display| OpError::failed(e);
    let manifest = Manifest {
        seed: world.seed,
        packs: world.packs.clone(),
    };
    let id = world_id(&manifest).map_err(|e| failed(&e))?;
    let bases = match &world.origin {
        Some(origin) => origin.clone(),
        None => base_zero(data, &manifest, world).map_err(|e| failed(&e))?,
    };
    let mut files = Vec::new();
    let mut names = BaseNames::default();
    if !bases.world.is_empty() {
        let text = to_string(&bases.world).map_err(|e| failed(&e))?;
        let hash = fnv1a64(text.as_bytes());
        names.world = Some(hash);
        files.push(FileOut {
            path: format!("{WORLDS}/{id}/world/{hash:016x}.ron"),
            text,
            keep: true,
        });
    }
    for (region, base) in &bases.regions {
        let name = data
            .registry
            .regions
            .name(*region)
            .ok_or_else(|| OpError::failed(format!("region {region:?} is not loaded")))?;
        let text = to_string(base).map_err(|e| failed(&e))?;
        let hash = fnv1a64(text.as_bytes());
        names.regions.insert(name.into(), hash);
        files.push(FileOut {
            path: region_path(&id, name, hash).map_err(|e| failed(&e))?,
            text,
            keep: true,
        });
    }
    let mut archive = Vec::new();
    for segment in &world.archive {
        match segment {
            Archived::Named(name) => archive.push(*name),
            Archived::Held(journal) => {
                let text = to_compact(journal).map_err(|e| failed(&e))?;
                let name = fnv1a64(text.as_bytes());
                archive.push(name);
                files.push(FileOut {
                    path: archive_path(&id, name),
                    text,
                    keep: true,
                });
            }
        }
    }
    files.push(FileOut {
        path: format!("{WORLDS}/{id}/manifest.ron"),
        text: to_string(&manifest).map_err(|e| failed(&e))?,
        keep: true,
    });
    let save = CharacterSave {
        schema: SAVE_SCHEMA,
        world: id,
        bases: names,
        checkpoint: split(world, data).checkpoint,
        journal: world.journal.clone(),
        fingerprint: world.fingerprint().map_err(|e| failed(&e))?,
        archive,
        log: world
            .log
            .iter()
            .filter(|l| crate::log::told(&l.event))
            .cloned()
            .collect(),
    };
    let text = to_compact(&save).map_err(|e| failed(&e))?;
    files.push(FileOut {
        path: save_path.into(),
        text,
        keep: false,
    });
    files.iter().try_for_each(fits)?;
    Ok(files)
}

/// Refuse a file its reader would refuse: the character save over `MAX_SAVE_BYTES`, a world
/// file over `MAX_FILE_BYTES`, so a save is never written that cannot be loaded.
fn fits(file: &FileOut) -> Result<(), OpError> {
    let (limit, what) = if file.keep {
        (MAX_FILE_BYTES, "a world file")
    } else {
        (MAX_SAVE_BYTES, "a save")
    };
    if file.text.len() as u64 > limit {
        return Err(OpError::failed(format!(
            "{} is {} bytes; {what} may hold {limit}",
            file.path,
            file.text.len()
        )));
    }
    Ok(())
}

/// Read a base the save names and check that its text is what its name says.
fn base<T: serde::de::DeserializeOwned>(
    path: &str,
    hash: u64,
    read: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Result<T, LoadError> {
    let text = read(path).map_err(|e| LoadError::BadBase(format!("{path}: {e}")))?;
    if fnv1a64(text.as_bytes()) != hash {
        return Err(LoadError::BadBase(format!(
            "{path}: its text is not its name"
        )));
    }
    parse(&text).map_err(|e| LoadError::BadBase(format!("{path}: {e}")))
}

/// A save's world, from its text and, for a character save, the world folder's files through
/// `read` (a relative path to its text). An older whole-world save is converted: it becomes the
/// base of an empty journal. A character save's bases are joined with its party, and its
/// journal's changes laid over them, in order, to the fingerprint it recorded (unless `force`
/// loaded it on other packs). No rule runs. `force` passes other packs, never a save that does
/// not hold together.
pub fn load_save(
    text: &str,
    data: &Data,
    force: bool,
    read: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Result<(World, Restored), LoadError> {
    #[derive(Deserialize)]
    struct Header {
        schema: u32,
    }
    let header: Header = parse(text).map_err(LoadError::Parse)?;
    if header.schema < SAVE_SCHEMA {
        let mut world = World::from_ron(text, data, force)?;
        world.origin = Some(split(&world, data).into());
        let report = Restored {
            applied: 0,
            converted_from: Some(header.schema),
        };
        return Ok((world, report));
    }
    if header.schema > SAVE_SCHEMA {
        return Err(LoadError::Schema(header.schema));
    }
    let save: CharacterSave = parse(text).map_err(LoadError::Parse)?;
    if !is_world_id(&save.world) {
        return Err(LoadError::BadSave(format!(
            "world '{}' is not a world's name",
            save.world
        )));
    }
    let id = save.world.clone();
    let path = format!("{WORLDS}/{id}/manifest.ron");
    let manifest_text = read(&path).map_err(|e| LoadError::BadSave(format!("{path}: {e}")))?;
    let manifest: Manifest = parse(&manifest_text).map_err(LoadError::Parse)?;
    if world_id(&manifest).map_err(LoadError::Parse)? != id {
        return Err(LoadError::BadSave(format!("{path} is not the world {id}")));
    }
    // Other packs are refused by `join`'s checks unless forced; forced, the world cannot be
    // held to the fingerprint the old packs gave it.
    let packs_differ = manifest.packs != data.fingerprints;
    let world_base: WorldBase = match save.bases.world {
        Some(hash) => base(&format!("{WORLDS}/{id}/world/{hash:016x}.ron"), hash, read)?,
        None => WorldBase::default(),
    };
    let mut regions = BTreeMap::new();
    for (name, hash) in &save.bases.regions {
        let region = data
            .registry
            .regions
            .get(name)
            .ok_or_else(|| LoadError::BadSave(format!("region '{name}' is not loaded")))?;
        let path = region_path(&id, name, *hash).map_err(LoadError::BadSave)?;
        let base: RegionBase = base(&path, *hash, read)?;
        regions.insert(region, base);
    }
    let parts = Split {
        manifest,
        world: world_base,
        regions,
        checkpoint: save.checkpoint,
    };
    let mut world = join(parts.clone(), data, force).map_err(|e| match e {
        JoinError::Load(e) => e,
        JoinError::Duplicate(what) => LoadError::BadSave(format!("two parts hold {what}")),
    })?;
    let zero = base_zero(data, &parts.manifest, &world).map_err(LoadError::BadSave)?;
    let bases = Bases::from(parts);
    world.origin = (bases != zero).then_some(bases);
    for entry in save.journal.entries() {
        apply_changes(&mut world, &entry.changes);
    }
    world.journal = save.journal;
    world.archive = save.archive.into_iter().map(Archived::Named).collect();
    world.log = save.log;
    crate::log::bound(&mut world.log);
    let actual = world.fingerprint().map_err(LoadError::Parse)?;
    if !packs_differ && actual != save.fingerprint {
        return Err(LoadError::Diverged {
            expected: save.fingerprint,
            actual,
        });
    }
    let report = Restored {
        applied: world.journal.len(),
        converted_from: None,
    };
    Ok((world, report))
}
