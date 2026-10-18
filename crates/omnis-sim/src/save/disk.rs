//! The files of a save (PRD D28, M8.5 7d), as text and relative paths; the host writes and reads
//! them. A world folder `.omnis/worlds/<id>/` holds the manifest and the bases, each base named
//! by the hash of its text; the character save names the folder and its bases. A schema 1 to 7
//! save, a whole world, is converted on load: the world becomes the base its journal starts from.

use super::character::{BaseNames, CharacterSave, expand, is_world_id, runs, world_id};
use super::{Manifest, RegionBase, Split, WorldBase, join, split};
use crate::apply::apply;
use crate::ops::OpError;
use crate::save::JoinError;
use crate::world::{LoadError, SAVE_SCHEMA, World};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use omnis_core::fnv1a64;
use omnis_data::Data;
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
    /// Commands replayed from the save's journal.
    pub replayed: usize,
    /// The schema of an older save made the base of a new journal.
    pub converted_from: Option<u32>,
}

/// The parts a new game on the world's seed and settings starts from: base 0.
fn base_zero(data: &Data, manifest: &Manifest, world: &World) -> Result<Split, String> {
    let mut fresh = World::new(data, manifest.seed, world.settings).map_err(|e| e.to_string())?;
    fresh.packs.clone_from(&manifest.packs);
    Ok(split(&fresh, data))
}

/// The path of a region's base: its string id's pack and name as folders.
fn region_path(id: &str, name: &str, hash: u64) -> Result<String, String> {
    let mut parts = name.split(':');
    match (parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some(pack), Some("region"), Some(region), None) => Ok(format!(
            "{WORLDS}/{id}/regions/{pack}/{region}/{hash:016x}.ron"
        )),
        _ => Err(format!("region '{name}' is not a region id")),
    }
}

/// The files that save `world` at `save_path`, in the order to write them: the bases its journal
/// starts from, the world's manifest, then the character save, so a save never names a base
/// that is not already on disk.
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
    let origin = match &world.origin {
        Some(origin) => origin.clone(),
        None => base_zero(data, &manifest, world).map_err(|e| failed(&e))?,
    };
    let mut files = Vec::new();
    let mut bases = BaseNames::default();
    if !origin.world.is_empty() {
        let text = to_string(&origin.world).map_err(|e| failed(&e))?;
        let hash = fnv1a64(text.as_bytes());
        bases.world = Some(hash);
        files.push(FileOut {
            path: format!("{WORLDS}/{id}/world/{hash:016x}.ron"),
            text,
            keep: true,
        });
    }
    for (region, base) in &origin.regions {
        let name = data
            .registry
            .regions
            .name(*region)
            .ok_or_else(|| OpError::failed(format!("region {region:?} is not loaded")))?;
        let text = to_string(base).map_err(|e| failed(&e))?;
        let hash = fnv1a64(text.as_bytes());
        bases.regions.insert(name.into(), hash);
        files.push(FileOut {
            path: region_path(&id, name, hash).map_err(|e| failed(&e))?,
            text,
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
        bases,
        checkpoint: origin.checkpoint,
        journal: runs(&world.journal),
        fingerprint: world.fingerprint().map_err(|e| failed(&e))?,
    };
    files.push(FileOut {
        path: save_path.into(),
        text: to_compact(&save).map_err(|e| failed(&e))?,
        keep: false,
    });
    Ok(files)
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
/// base of an empty journal. A character save's bases are joined and its journal replayed to the
/// fingerprint it recorded (unless `force` loaded it on other packs). `force` passes other packs,
/// never a save that does not hold together.
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
        world.origin = Some(split(&world, data));
        let report = Restored {
            replayed: 0,
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
    // Other packs are refused by `join`'s checks unless forced; forced, the replay cannot be
    // held to the fingerprint the old packs reached.
    let packs_differ = manifest.packs != data.fingerprints;
    let commands = expand(&save.journal)?;
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
    world.origin = (parts != zero).then_some(parts);
    for (index, command) in commands.iter().enumerate() {
        apply(&mut world, data, command.clone())
            .map_err(|rejection| LoadError::Replay { index, rejection })?;
    }
    world.log.clear();
    let actual = world.fingerprint().map_err(LoadError::Parse)?;
    if !packs_differ && actual != save.fingerprint {
        return Err(LoadError::Diverged {
            expected: save.fingerprint,
            actual,
        });
    }
    let report = Restored {
        replayed: commands.len(),
        converted_from: None,
    };
    Ok((world, report))
}
