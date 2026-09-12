//! The headless game: packs loaded, a world running, every protocol op answered in-process
//! (ARCHITECTURE.md §9.2, §10). The CLI's `play` and `map text` use it directly; the MCP bridge
//! uses it in `--headless` mode. Host ops (files, packs) are handled here; the rest go to
//! `omnis_sim::dispatch`.

use omnis_data::ron_io::read_text;
use omnis_data::{Data, LoadReport, load_packs};
use omnis_sim::ops::client_path;
use omnis_sim::{NewGameError, Op, OpError, Reply, Settings, World, dispatch, ops};
use std::fmt;
use std::path::{Path, PathBuf};

/// A running headless game.
#[derive(Debug)]
pub struct Headless {
    /// The pack directories, in load order; `pack.reload` reads them again.
    pub packs: Vec<PathBuf>,
    /// The loaded packs.
    pub data: Data,
    /// The world.
    pub world: World,
}

/// Why a headless game could not start.
#[derive(Debug)]
pub enum HeadlessError {
    /// The packs did not load.
    Packs(LoadReport),
    /// The packs name no entry map.
    NewGame(NewGameError),
}

impl fmt::Display for HeadlessError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HeadlessError::Packs(report) => write!(f, "{report}"),
            HeadlessError::NewGame(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for HeadlessError {}

impl Headless {
    /// Load the packs and start a new game with `seed`.
    pub fn new(packs: Vec<PathBuf>, seed: u64) -> Result<Headless, HeadlessError> {
        let data = load(&packs).map_err(HeadlessError::Packs)?;
        let world = World::new(&data, seed, Settings::default()).map_err(HeadlessError::NewGame)?;
        Ok(Headless { packs, data, world })
    }

    /// Answer one op.
    pub fn handle(&mut self, op: &Op) -> Result<Reply, OpError> {
        match op {
            Op::SaveWrite { path } => {
                let file = PathBuf::from(client_path(path, &["ron"])?);
                let text = self.world.to_ron().map_err(OpError::failed)?;
                if let Some(parent) = file.parent().filter(|p| !p.as_os_str().is_empty()) {
                    std::fs::create_dir_all(parent).map_err(OpError::failed)?;
                }
                std::fs::write(&file, text).map_err(OpError::failed)?;
                Ok(Reply::Written { path: path.clone() })
            }
            Op::SaveRead { path, force } => {
                let file = PathBuf::from(client_path(path, &["ron"])?);
                let text = read_text(&file, &file).map_err(OpError::failed)?;
                self.world = World::from_ron(&text, &self.data, *force).map_err(OpError::failed)?;
                ops::status(&self.world, &self.data).map(Reply::Status)
            }
            Op::PackReload => {
                let data = load(&self.packs).map_err(OpError::failed)?;
                let p = self.world.position;
                if data
                    .maps
                    .get(&p.map)
                    .and_then(|m| m.cell(p.x, p.y))
                    .is_none()
                {
                    return Err(OpError::failed(format!(
                        "the party's tile {p} is not in the reloaded packs; nothing changed"
                    )));
                }
                self.data = data;
                Ok(Reply::Done {})
            }
            Op::Screenshot { .. } => Err(OpError::failed(
                "a screenshot needs the game window; this is headless",
            )),
            Op::RulesSet { slot, source } => {
                ops::bounded(source)?;
                self.data
                    .rules
                    .set_slot(slot, source)
                    .map_err(OpError::bad_request)?;
                ops::slot_view(&self.data, slot).map(|rule| Reply::Rule { rule })
            }
            other => dispatch(&mut self.world, &self.data, other),
        }
    }
}

fn load(packs: &[PathBuf]) -> Result<Data, LoadReport> {
    let roots: Vec<&Path> = packs.iter().map(PathBuf::as_path).collect();
    load_packs(&roots)
}
