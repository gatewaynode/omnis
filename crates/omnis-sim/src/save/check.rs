//! What the loaded packs make of a save, before it loads (M8.5 7h4): `examine` rebuilds it and
//! reports, changing nothing; `restore` loads what was examined. A report answers every save
//! that holds together, whether it can load or not; only a save that cannot be read, or was
//! altered, is an error.

use super::disk::{Built, PackFit, Restored, build};
use super::places::{self, PlaceWarning};
use super::{Manifest, Missing, NameTable, fold_from, remap};
use crate::world::{LoadError, World};
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use omnis_data::{Data, PackFingerprint};
use serde::{Deserialize, Serialize};

/// What the loaded packs make of a save.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveCheck {
    /// The verdict.
    pub verdict: Verdict,
    /// Every pack the save was made with or that is loaded, by id: what it was, what it is.
    pub packs: Vec<PackChange>,
    /// Names the save holds whose number moved under the loaded packs.
    pub moved: usize,
    /// Names the save holds that the loaded packs lack, each where the world uses it.
    pub missing: Vec<Missing>,
    /// Places the loaded maps no longer allow, and what loading does with each.
    pub places: Vec<PlaceWarning>,
    /// Automap tiles outside their map, dropped.
    pub trimmed: usize,
    /// Interactions of the save's journal laid over its bases.
    pub applied: usize,
    /// The schema of an older save, converted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub converted_from: Option<u32>,
}

/// Whether a save loads on the loaded packs, and how.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The packs it was made with.
    #[default]
    Same,
    /// Other packs, every name the same: it loads as it is.
    Content,
    /// Other packs whose names moved: it loads by name, and the world moves on.
    Renamed,
    /// It names what the loaded packs lack: it does not load.
    Unresolved,
    /// Other packs, and a save that keeps no names (before 7h2): only a forced load reads it,
    /// numbers as written.
    Unnamed,
}

impl Verdict {
    /// Whether `restore` loads it unforced.
    #[must_use]
    pub const fn loads(self) -> bool {
        matches!(self, Verdict::Same | Verdict::Content | Verdict::Renamed)
    }
}

/// A pack as a save was made with it and as it is loaded; either side is absent when the pack
/// is new or gone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackChange {
    /// The pack's id.
    pub id: String,
    /// As the save was made with it.
    pub saved: Option<PackStamp>,
    /// As it is loaded.
    pub loaded: Option<PackStamp>,
}

/// A pack's version and content hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackStamp {
    /// Its version.
    pub version: String,
    /// Its content hash.
    pub hash: u64,
}

impl From<&PackFingerprint> for PackStamp {
    fn from(p: &PackFingerprint) -> Self {
        PackStamp {
            version: p.version.clone(),
            hash: p.hash,
        }
    }
}

/// A save examined against the loaded packs, ready for `restore`.
#[derive(Debug)]
pub struct Examined {
    report: SaveCheck,
    world: Option<Box<World>>,
    old: Option<Box<Manifest>>,
    against: Vec<PackFingerprint>,
    /// The places check changed the world: a load folds.
    fixed: bool,
}

impl Examined {
    /// What the loaded packs make of it.
    #[must_use]
    pub const fn report(&self) -> &SaveCheck {
        &self.report
    }

    /// The report, the save dropped.
    #[must_use]
    pub fn into_report(self) -> SaveCheck {
        self.report
    }
}

/// Every pack of either side, by id.
fn changes(saved: &[PackFingerprint], loaded: &[PackFingerprint]) -> Vec<PackChange> {
    let mut ids: Vec<&str> = saved.iter().chain(loaded).map(|p| p.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .map(|id| PackChange {
            id: id.into(),
            saved: saved.iter().find(|p| p.id == id).map(PackStamp::from),
            loaded: loaded.iter().find(|p| p.id == id).map(PackStamp::from),
        })
        .collect()
}

/// What the loaded packs make of a save, changing nothing. The save is rebuilt in its own
/// numbering and checked against its fingerprint (an error when it cannot be read or was
/// altered); then, on other packs, fitted to them: renumbered by name, its places checked and
/// fixed, each fix a warning.
pub fn examine(
    text: &str,
    data: &Data,
    read: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Result<Examined, LoadError> {
    examine_with(text, data, false, read)
}

/// `examine`, with `force` building a save with no name table on other packs.
pub(crate) fn examine_with(
    text: &str,
    data: &Data,
    force: bool,
    read: &mut dyn FnMut(&str) -> Result<String, String>,
) -> Result<Examined, LoadError> {
    let against = data.fingerprints.clone();
    let (mut world, old, applied, converted_from) = match build(text, data, force, read)? {
        Built::Unnamed { saved } => {
            let report = SaveCheck {
                verdict: Verdict::Unnamed,
                packs: changes(&saved, &against),
                ..SaveCheck::default()
            };
            return Ok(Examined {
                report,
                world: None,
                old: None,
                against,
                fixed: false,
            });
        }
        Built::World {
            world,
            manifest,
            applied,
            converted_from,
        } => (world, manifest, applied, converted_from),
    };
    let mut report = SaveCheck {
        packs: changes(&world.packs, &against),
        applied,
        converted_from,
        ..SaveCheck::default()
    };
    let names = old.as_ref().map(|m| &m.names).filter(|n| !n.is_empty());
    let fixed = match names {
        _ if world.packs == against => false,
        None => {
            report.verdict = Verdict::Unnamed;
            false
        }
        Some(names) => refit(&mut world, names, data, &mut report),
    };
    Ok(Examined {
        report,
        world: Some(world),
        old,
        against,
        fixed,
    })
}

/// The running world, `old` the packs it runs on, examined against freshly loaded packs
/// (`pack.reload`, M8.5 7h5): fitted by name as a save made on `old` would be, changing
/// nothing. `restore` takes it, folding a renumbered or fixed world with its journal held for
/// `old`'s folder.
#[must_use]
pub fn examine_running(world: &World, old: &Data, data: &Data) -> Examined {
    let against = data.fingerprints.clone();
    let mut report = SaveCheck {
        packs: changes(&world.packs, &against),
        ..SaveCheck::default()
    };
    let mut next = world.clone();
    let fixed = world.packs != against && refit(&mut next, &NameTable::of(old), data, &mut report);
    Examined {
        report,
        world: Some(Box::new(next)),
        old: Some(Box::new(Manifest::of(world, old))),
        against,
        fixed,
    }
}

/// Fit `world`, numbered by `names`, to other packs: `Content` when every name is the same,
/// else renumbered by name (`Renamed`) or `Unresolved` with what is missing; a world that fits
/// has its places checked and fixed. Whether the fix changed the world.
fn refit(world: &mut World, names: &NameTable, data: &Data, report: &mut SaveCheck) -> bool {
    let now = NameTable::of(data);
    report.verdict = if *names == now {
        Verdict::Content
    } else {
        match remap::world(world, names, data) {
            Ok(()) => {
                remap::log(&mut world.log, names, data);
                report.moved = names.moved(&now);
                Verdict::Renamed
            }
            Err(missing) => {
                report.missing = missing;
                Verdict::Unresolved
            }
        }
    };
    if report.verdict == Verdict::Unresolved {
        return false;
    }
    let checked = places::fix(world, data);
    report.places = checked.places;
    report.trimmed = checked.trimmed;
    checked.changed
}

/// Load what was examined: refused when it names what the packs lack, when it keeps no names
/// unless `force`, and when the packs are not the ones it was examined against. On other packs
/// the world records them; renumbered, or changed by the places check, it folds, its journal held for
/// its old world's folder.
pub fn restore(
    examined: Examined,
    data: &Data,
    force: bool,
) -> Result<(World, Restored), LoadError> {
    let Examined {
        report,
        world,
        old,
        against,
        fixed,
    } = examined;
    if against != data.fingerprints {
        return Err(LoadError::PackMismatch);
    }
    let fit = match report.verdict {
        Verdict::Same => PackFit::Same,
        Verdict::Content => PackFit::Content,
        Verdict::Renamed => PackFit::Renumbered,
        Verdict::Unresolved => return Err(LoadError::Unresolved(report.missing)),
        Verdict::Unnamed if force => PackFit::Forced,
        Verdict::Unnamed => return Err(LoadError::PackMismatch),
    };
    let Some(mut world) = world.map(|w| *w) else {
        return Err(LoadError::PackMismatch);
    };
    if matches!(fit, PackFit::Content | PackFit::Renumbered) {
        world.packs.clone_from(&data.fingerprints);
    }
    let mut world = world.checked(data, fit == PackFit::Forced)?;
    if fit == PackFit::Renumbered || fixed {
        fold_from(&mut world, data, old);
    }
    let restored = Restored {
        applied: report.applied,
        converted_from: report.converted_from,
        fit,
    };
    Ok((world, restored))
}
