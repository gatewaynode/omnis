//! Saving and loading a game: the one path the pause screen, the title, the defeat screen and the
//! dev socket's `save.write` and `save.read` share (presentation-ARCHITECTURE §4.5, §6.2). A save
//! is a character save beside a world folder (PRD D28, M8.5 7d). The rules are the engine's
//! I/O-free ones (`ops::client_path`, `save::save_files` with the save rule, `save::load_save`
//! with conversion and the replay); this module only writes and reads the files, each whole, with
//! the pack loader's limits, under `root`. Bevy-free.

use crate::sim::SimWorld;
use omnis_sim::api::{
    Examined, PackFit, Restored, SaveCheck, Verdict, examine as examine_save, load_save,
    restore as restore_save, tidy_list,
};
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::ron_io::{
    list_under, read_save_under, read_under, remove_under, size_under, write_files,
};
use omnis_sim::ops::client_path;
use omnis_sim::{OpError, World};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// The file `path` names under `root`, once `path` passes the client path rule (relative, no
/// `..`, a `.ron` file).
///
/// # Errors
/// The rule's refusal.
pub fn file(root: &Path, path: &str) -> Result<PathBuf, OpError> {
    Ok(root.join(client_path(path, &["ron"])?))
}

/// Whether there is a save at `path` under `root` to load.
#[must_use]
pub fn exists(root: &Path, path: &str) -> bool {
    file(root, path).is_ok_and(|f| f.is_file())
}

/// Save `world` to `path` under `root`: the world folder's bases and manifest first, then the
/// character save, each written whole.
///
/// # Errors
/// The save rule forbids saving here, the path is refused, or a file cannot be written.
pub fn save(world: &SimWorld, data: &Data, root: &Path, path: &str) -> Result<PathBuf, OpError> {
    let file = file(root, path)?;
    let files = world.save_files(data, path)?;
    let list: Vec<(&str, &str, bool)> = files
        .iter()
        .map(|f| (f.path.as_str(), f.text.as_str(), f.keep))
        .collect();
    write_files(root, &list).map_err(OpError::failed)?;
    Ok(file)
}

/// A world loaded from a save, and what loading it did, in the player's words.
#[derive(Debug)]
pub struct Loaded {
    /// The world.
    pub world: World,
    /// "12 interactions applied in 3 ms", or "converted from a schema 7 save".
    pub said: String,
    /// Each place the loaded maps no longer allowed, and what the load did with it (M8.5 7h4).
    pub warnings: Vec<String>,
}

impl Loaded {
    /// What the log says of it: "Loaded `path`: …", then each place warning.
    #[must_use]
    pub fn lines(&self, path: &str) -> Vec<String> {
        let mut lines = vec![format!("Loaded {path}: {}", self.said)];
        lines.extend(self.warnings.iter().cloned());
        lines
    }
}

/// A save read and examined against the loaded packs, not yet loaded (M8.5 7h4).
#[derive(Debug)]
pub struct Opened {
    examined: Examined,
    started: Instant,
}

impl Opened {
    /// What the loaded packs make of it.
    #[must_use]
    pub fn report(&self) -> &SaveCheck {
        self.examined.report()
    }

    /// The report, the save dropped.
    #[must_use]
    pub fn into_report(self) -> SaveCheck {
        self.examined.into_report()
    }

    /// Load it, unforced: refused when its verdict does not load.
    ///
    /// # Errors
    /// It names what the packs lack, keeps no names on other packs, or does not hold together.
    pub fn load(self, data: &Data) -> Result<Loaded, OpError> {
        let warnings = warnings(self.report());
        let (world, restored) =
            restore_save(self.examined, data, false).map_err(OpError::failed)?;
        Ok(Loaded {
            world,
            said: said(restored, self.started.elapsed().as_millis(), warnings.len()),
            warnings,
        })
    }
}

/// A report's place warnings, each a sentence.
fn warnings(report: &SaveCheck) -> Vec<String> {
    report
        .places
        .iter()
        .map(|p| {
            let text = p.to_string();
            let mut chars = text.chars();
            chars.next().map_or_else(String::new, |c| {
                c.to_uppercase().chain(chars).collect::<String>() + "."
            })
        })
        .collect()
}

/// The save at `path` under `root`, examined against `data` (M8.5 7h4): the window loads it
/// at once when its verdict loads, and shows the report when it does not.
///
/// # Errors
/// The path is refused, a file is missing, too large or a link, or the save cannot be read or
/// was altered.
pub fn examine(data: &Data, root: &Path, path: &str) -> Result<Opened, OpError> {
    let path = client_path(path, &["ron"])?;
    let text = read_save_under(root, path).map_err(OpError::failed)?;
    let started = Instant::now();
    let examined = examine_save(&text, data, &mut |p| {
        read_under(root, p).map_err(|e| e.to_string())
    })
    .map_err(OpError::failed)?;
    Ok(Opened { examined, started })
}

/// What a load did, in words.
fn said(restored: Restored, millis: u128, places: usize) -> String {
    let done = match restored.converted_from {
        Some(schema) => format!("converted from a schema {schema} save"),
        None => {
            let n = restored.applied;
            let interactions = if n == 1 {
                "interaction"
            } else {
                "interactions"
            };
            format!("{n} {interactions} applied in {millis} ms")
        }
    };
    let done = match places {
        0 => done,
        1 => format!("{done}; 1 place changed, see the log"),
        n => format!("{done}; {n} places changed, see the log"),
    };
    match restored.fit {
        PackFit::Same => done,
        PackFit::Content => format!("{done}; made with other packs, every name the same"),
        PackFit::Renumbered => {
            format!("{done}; made with other packs, loaded by name. The world has moved on")
        }
        PackFit::Forced => format!("{done}; made with other packs, loaded as written"),
    }
}

/// The world saved at `path` under `root`: an older save converted, a character save joined from
/// its world folder with its journal's changes laid over, checked against `data`, examined
/// first; with `force`, a save that keeps no names loads on other packs, numbers as written.
///
/// # Errors
/// The path is refused, a file is missing, too large or a link, or it is not a save these packs
/// can load.
pub fn load(data: &Data, root: &Path, path: &str, force: bool) -> Result<Loaded, OpError> {
    let opened = examine(data, root, path)?;
    if !(force && opened.report().verdict == Verdict::Unnamed) {
        return opened.load(data);
    }
    let path = client_path(path, &["ron"])?;
    let text = read_save_under(root, path).map_err(OpError::failed)?;
    let started = Instant::now();
    let (world, restored) = load_save(&text, data, force, &mut |p| {
        read_under(root, p).map_err(|e| e.to_string())
    })
    .map_err(OpError::failed)?;
    Ok(Loaded {
        world,
        said: said(restored, started.elapsed().as_millis(), 0),
        warnings: Vec::new(),
    })
}

/// What a tidy found, or deleted: the world-folder files no save under `.omnis/` names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tidied {
    /// Their paths, relative to the root.
    pub files: Vec<String>,
    /// Their size together.
    pub bytes: u64,
}

/// The world-folder files no character save under `.omnis/` names (M8.5 7e,
/// `save::tidy_list`), deleted when `delete`; the CLI's `world tidy` does the same.
///
/// # Errors
/// A folder cannot be listed, a save under `.omnis/` does not read, or a file cannot be deleted.
pub fn tidy(root: &Path, delete: bool) -> Result<Tidied, String> {
    let all = list_under(root, ".omnis").map_err(|e| e.to_string())?;
    let files = tidy_list(&all, &mut |p| {
        read_save_under(root, p).map_err(|e| e.to_string())
    })?;
    let mut bytes = 0;
    for file in &files {
        bytes += size_under(root, file).map_err(|e| e.to_string())?;
    }
    if delete {
        for file in &files {
            remove_under(root, file).map_err(|e| e.to_string())?;
        }
    }
    Ok(Tidied { files, bytes })
}
