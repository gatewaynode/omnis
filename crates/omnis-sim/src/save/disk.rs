//! The files of a save (PRD D28, M8.5 7d), as text and relative paths; the host writes and reads
//! them. A world folder `.omnis/worlds/<id>/` holds the manifest and the bases, each base named
//! by the hash of its text; the character save names the folder and its bases, and holds the
//! party as it stands and the journal of the world's changes since. A load lays the changes over
//! the bases and runs no rule. A schema 1 to 7 save, a whole world, is converted on load: the
//! world becomes the base of an empty journal.

use super::character::{BaseNames, CharacterSave, is_world_id, world_id};
use super::check::{examine_with, restore};
use super::join::assemble;
use super::{Archived, Bases, Manifest, NameTable, RegionBase, Segment, Split, WorldBase, split};
use crate::journal::apply_changes;
use crate::ops::OpError;
use crate::save::JoinError;
use crate::world::{LoadError, SAVE_SCHEMA, World};
use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_core::RegionId;
use omnis_core::fnv1a64;
use omnis_data::limits::{MAX_FILE_BYTES, MAX_SAVE_BYTES};
use omnis_data::ron_io::{parse, to_compact, to_string};
use omnis_data::{Data, PackFingerprint};
use serde::Deserialize;

/// The first schema that is a character save; an older save is a whole world.
pub(crate) const CHARACTER_SCHEMA: u32 = 8;

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
    /// How the save's packs fit the loaded ones (M8.5 7h3).
    pub fit: PackFit,
}

/// How a save's packs fit the loaded ones (M8.5 7h3).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PackFit {
    /// The same packs.
    #[default]
    Same,
    /// Other packs with the same names: content changed, no number moved.
    Content,
    /// Other packs whose names moved: the world was renumbered by name and folded, its
    /// history kept in its old world's folder.
    Renumbered,
    /// Other packs, and a save with no name table (before 7h2), loaded forced with its
    /// numbers as written.
    Forced,
    /// Other packs whose migrations changed it, applied with the player's consent (M8.5 7h6):
    /// renumbered by name through them and folded.
    Migrated,
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
    let manifest = Manifest::of(world, data);
    let id = world_id(&manifest).map_err(|e| failed(&e))?;
    // Base 0, the packs as written, holds nothing: a new world's parts are all the party's.
    let bases = world.origin.clone().unwrap_or_default();
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
    let mut others = BTreeMap::new();
    for segment in &world.archive {
        match segment {
            Archived::Named(segment) => archive.push(segment.clone()),
            Archived::Held { journal, from } => {
                let text = to_compact(journal).map_err(|e| failed(&e))?;
                let name = fnv1a64(text.as_bytes());
                let folder = match from {
                    Some(old) => {
                        let folder = world_id(old).map_err(|e| failed(&e))?;
                        others.insert(folder.clone(), old);
                        Some(folder)
                    }
                    None => None,
                };
                files.push(FileOut {
                    path: archive_path(folder.as_deref().unwrap_or(&id), name),
                    text,
                    keep: true,
                });
                archive.push(Segment {
                    world: folder,
                    name,
                });
            }
        }
    }
    // A segment in this world's own folder is named by its hash alone.
    for segment in &mut archive {
        if segment.world.as_deref() == Some(id.as_str()) {
            segment.world = None;
        }
    }
    for (folder, old) in others {
        files.push(FileOut {
            path: format!("{WORLDS}/{folder}/manifest.ron"),
            text: to_string(old).map_err(|e| failed(&e))?,
            keep: true,
        });
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
        migrated: world.migrated.clone(),
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
/// journal's changes laid over them, in order, to the fingerprint it recorded. No rule runs.
/// `force` passes other packs for a save with no name table, never a save that does not hold
/// together or names what the packs lack. It is `examine` then `restore` (M8.5 7h4).
pub fn load_save(
    text: &str,
    data: &Data,
    force: bool,
    read: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Result<(World, Restored), LoadError> {
    load_save_with(text, data, force, false, read)
}

/// `load_save`, with `migrate` applying the packs' migrations when they change the save
/// (M8.5 7h6): the player's consent, which `save.read migrate` gives.
pub fn load_save_with(
    text: &str,
    data: &Data,
    force: bool,
    migrate: bool,
    read: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Result<(World, Restored), LoadError> {
    restore(examine_with(text, data, force, read)?, data, force, migrate)
}

/// A save rebuilt in its own numbering, not yet fitted to the loaded packs.
pub(crate) enum Built {
    /// Other packs and no name table (a save before 7h2, or a whole world), not forced: its
    /// numbers cannot be read.
    Unnamed {
        /// The packs it was made with.
        saved: Vec<PackFingerprint>,
    },
    /// The world it recorded.
    World {
        /// It, in the save's numbering.
        world: Box<World>,
        /// Its world folder's manifest; none for a whole-world save.
        manifest: Option<Box<Manifest>>,
        /// Interactions laid over the bases.
        applied: usize,
        /// The schema of an older save, converted.
        converted_from: Option<u32>,
    },
}

/// Read a save and rebuild its world in its own numbering, checked against its fingerprint.
pub(crate) fn build(
    text: &str,
    data: &Data,
    force: bool,
    read: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Result<Built, LoadError> {
    #[derive(Deserialize)]
    struct Header {
        schema: u32,
    }
    let header: Header = parse(text).map_err(LoadError::Parse)?;
    if header.schema < CHARACTER_SCHEMA {
        #[derive(Deserialize)]
        struct Packs {
            #[serde(default)]
            packs: Vec<PackFingerprint>,
        }
        let mut world = match World::from_ron(text, data, force) {
            Err(LoadError::PackMismatch) if !force => {
                let saved = parse::<Packs>(text).map_err(LoadError::Parse)?.packs;
                return Ok(Built::Unnamed { saved });
            }
            other => other?,
        };
        world.origin = Some(split(&world, data).into());
        return Ok(Built::World {
            world: Box::new(world),
            manifest: None,
            applied: 0,
            converted_from: Some(header.schema),
        });
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
    if let Some(bad) = save
        .archive
        .iter()
        .filter_map(|s| s.world.as_deref())
        .find(|w| !is_world_id(w))
    {
        return Err(LoadError::BadSave(format!(
            "archive world '{bad}' is not a world's name"
        )));
    }
    let id = save.world.clone();
    let path = format!("{WORLDS}/{id}/manifest.ron");
    let manifest_text = read(&path).map_err(|e| LoadError::BadSave(format!("{path}: {e}")))?;
    let manifest: Manifest = parse(&manifest_text).map_err(LoadError::Parse)?;
    if world_id(&manifest).map_err(LoadError::Parse)? != id {
        return Err(LoadError::BadSave(format!("{path} is not the world {id}")));
    }
    // A save with no name table reads its numbers as the loaded packs': on other packs, only
    // forced (owner, 2026-10-18).
    let same_packs = manifest.packs == data.fingerprints;
    if !same_packs && manifest.names.is_empty() && !force {
        return Ok(Built::Unnamed {
            saved: manifest.packs,
        });
    }
    let world_base: WorldBase = match save.bases.world {
        Some(hash) => base(&format!("{WORLDS}/{id}/world/{hash:016x}.ron"), hash, read)?,
        None => WorldBase::default(),
    };
    let mut regions = BTreeMap::new();
    for (name, hash) in &save.bases.regions {
        let region = region_number(&manifest.names, data, name)
            .ok_or_else(|| LoadError::BadSave(format!("region '{name}' is not named")))?;
        let path = region_path(&id, name, *hash).map_err(LoadError::BadSave)?;
        let base: RegionBase = base(&path, *hash, read)?;
        regions.insert(region, base);
    }
    let mut parts = Split {
        manifest,
        world: world_base,
        regions,
        checkpoint: save.checkpoint,
    };
    let mut journal = save.journal;
    super::rows::parts(data, &mut parts, &mut journal)?;
    let manifest = parts.manifest.clone();
    let mut world = assemble(parts.clone()).map_err(|e| match e {
        JoinError::Load(e) => e,
        JoinError::Duplicate(what) => LoadError::BadSave(format!("two parts hold {what}")),
    })?;
    let bases = Bases::from(parts);
    world.origin = (bases != Bases::default()).then_some(bases);
    for entry in journal.entries() {
        apply_changes(&mut world, &entry.changes);
    }
    world.journal = journal;
    world.archive = save
        .archive
        .into_iter()
        .map(|s| {
            Archived::Named(Segment {
                world: Some(s.world.unwrap_or_else(|| id.clone())),
                name: s.name,
            })
        })
        .collect();
    world.migrated = save.migrated;
    world.log = save.log;
    super::rows::log(data, &mut world.log);
    crate::log::bound(&mut world.log);
    // Rebuilt in its own numbering, the world is the one the save recorded, on any packs; a
    // schema 8 save's was fingerprinted with its encounters' rows, which are ids now.
    let converted = header.schema < SAVE_SCHEMA;
    let actual = world.fingerprint().map_err(LoadError::Parse)?;
    if !converted && actual != save.fingerprint {
        return Err(LoadError::Diverged {
            expected: save.fingerprint,
            actual,
        });
    }
    Ok(Built::World {
        applied: world.journal.len(),
        world: Box::new(world),
        manifest: Some(Box::new(manifest)),
        converted_from: converted.then_some(header.schema),
    })
}

/// The number of the region `name` in a save's world folder: by its name table, or, before the
/// table, the loaded registry's.
fn region_number(names: &NameTable, data: &Data, name: &str) -> Option<RegionId> {
    if names.is_empty() {
        return data.registry.regions.get(name);
    }
    let index = names.regions.iter().position(|r| r == name)?;
    u32::try_from(index).ok().map(RegionId)
}
