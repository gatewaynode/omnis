//! Saving and loading a game: the one path the pause screen, the title, the defeat screen and the
//! dev socket's `save.write` and `save.read` share (presentation-ARCHITECTURE §4.5, §6.2). The
//! rules are the engine's I/O-free ones (`ops::client_path`, the save rule in
//! `SimWorld::save_text`, the migration in `ops::load_text`); this module only reads and writes the
//! file, with the pack loader's limits on what it reads. Bevy-free.

use crate::sim::SimWorld;
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::ron_io::read_text;
use omnis_sim::ops::{client_path, load_text};
use omnis_sim::{OpError, World};
use std::path::{Path, PathBuf};

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

/// Save `world` to `path` under `root`, creating its directory.
///
/// # Errors
/// The save rule forbids saving here, the path is refused, or the file cannot be written.
pub fn save(world: &SimWorld, root: &Path, path: &str) -> Result<PathBuf, OpError> {
    let file = file(root, path)?;
    let text = world.save_text()?;
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(OpError::failed)?;
    }
    std::fs::write(&file, text).map_err(OpError::failed)?;
    Ok(file)
}

/// The world saved at `path` under `root`, migrated to the current schema and checked against
/// `data`; with `force`, a save made with other packs loads anyway.
///
/// # Errors
/// The path is refused, the file is missing, too large or a link, or it is not a save these
/// packs can load.
pub fn load(data: &Data, root: &Path, path: &str, force: bool) -> Result<World, OpError> {
    let file = file(root, path)?;
    let text = read_text(&file, Path::new(path)).map_err(OpError::failed)?;
    load_text(&text, data, force)
}
