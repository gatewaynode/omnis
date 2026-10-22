//! The editor's model (M5 Editor v1 from M5.4; ARCHITECTURE §8.2, `tasks/plans/m5-editor-v1.md`
//! §Client side): the document session over the pack being edited, the open map's level,
//! cursor and facing, the plan view, and the acts the layer's controls ask for. Bevy-free: the
//! shell (`shell/editor.rs`) draws it and routes the pointer and the keys to it.

pub mod plan;
pub mod view;

use omnis_sim::omnis_core::{Facing, Level};
use omnis_sim::omnis_data::edit::{EditError, EditorStatus, Grid, MapEntry, Session};
use omnis_sim::omnis_data::{Data, MapData, MapDef};
use std::path::PathBuf;

/// What a control of the editor's layer asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditAct {
    /// The map choice: steps through the pack's map files.
    Map,
    /// Open the chosen map, dropping any unsaved change to the open one.
    Open,
    /// The level choice: steps through the open map's levels.
    Level,
    /// Zoom the plan in.
    ZoomIn,
    /// Zoom the plan out.
    ZoomOut,
    /// Put the cursor at the plan's middle.
    Centre,
    /// Turn the cursor's facing a quarter left.
    FaceLeft,
    /// Turn the cursor's facing a quarter right.
    FaceRight,
    /// Open the editor's menu.
    Menu,
}

/// What the layer does after an act: nothing more, or open the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// Nothing beyond the model's change.
    Nothing,
    /// Open the editor's menu.
    Menu,
}

/// The editor as the layer reads and changes it.
#[derive(Debug)]
pub struct Editor {
    /// The document session: the pack, the open map, the notes.
    pub session: Session,
    /// The pack's map files, as listed when the editor opened.
    pub maps: Vec<MapEntry>,
    /// The map chosen in the tools column, an index into `maps`.
    pub chosen: usize,
    /// The level shown.
    pub level: Level,
    /// The cell the inspector describes and the preview looks from.
    pub cursor: (u16, u16),
    /// The way the preview looks.
    pub facing: Facing,
    /// Where the plan looks.
    pub view: plan::View,
    /// The pane the plan was last laid out for, in pixels.
    pub pane: (u32, u32),
    /// The last refusal or outcome to show.
    pub message: Option<String>,
    /// The map as the loader last took it; what is derived (cliffs, a stair's rise) is read
    /// from here, and it stays while the document does not resolve.
    pub resolved: Option<MapData>,
    /// Counts every change the layer should redraw for.
    pub revision: u64,
}

impl Editor {
    /// The editor over `roots` (the last is edited), listing its maps.
    ///
    /// # Errors
    /// The pack's manifest or notes do not read, or its maps cannot be listed.
    pub fn new(roots: Vec<PathBuf>) -> Result<Editor, EditError> {
        let session = Session::new(roots)?;
        let maps = session.maps()?;
        Ok(Editor {
            session,
            maps,
            chosen: 0,
            level: Level::GROUND,
            cursor: (0, 0),
            facing: Facing::North,
            view: plan::View::default(),
            pane: (800, 600),
            message: None,
            resolved: None,
            revision: 0,
        })
    }

    /// Open the map `id`, putting the cursor at its start and fitting the plan to the pane.
    ///
    /// # Errors
    /// The session's: no such map, or its layout does not read.
    pub fn open(&mut self, id: &str, data: &Data) -> Result<(), EditError> {
        self.session.open(id)?;
        if let Some(i) = self.maps.iter().position(|m| m.id == id) {
            self.chosen = i;
        }
        let (start, size) = self.def().map_or(((0, 0, Facing::North), (0, 0)), |d| {
            (d.start, (d.width, d.height))
        });
        self.level = Level::GROUND;
        self.cursor = (start.0, start.1);
        self.facing = start.2;
        self.view = plan::View::fitted(size, self.pane);
        self.message = None;
        self.resolved = None;
        self.changed(data);
        Ok(())
    }

    /// The document changed (by the layer or over the socket): resolve it again, keeping the
    /// last valid resolve and saying why when the loader refuses it, and ask for a redraw.
    pub fn changed(&mut self, data: &Data) {
        self.revision = self.revision.wrapping_add(1);
        let Some(def) = self.def() else {
            self.resolved = None;
            return;
        };
        match MapData::resolve(&def, data) {
            Ok(map) => self.resolved = Some(map),
            Err(errors) => {
                if let Some(first) = errors.first() {
                    self.message = Some(first.to_string());
                }
            }
        }
        if !self.levels().contains(&self.level) {
            self.level = Level::GROUND;
        }
        self.cursor = (
            self.cursor.0.min(def.width.saturating_sub(1)),
            self.cursor.1.min(def.height.saturating_sub(1)),
        );
    }

    /// The open map as its file would hold it.
    #[must_use]
    pub fn def(&self) -> Option<MapDef> {
        self.session.document().map(|d| d.def())
    }

    /// The open map's id.
    #[must_use]
    pub fn map_id(&self) -> Option<String> {
        self.session.document().map(|d| d.def().id)
    }

    /// The open map's size.
    #[must_use]
    pub fn size(&self) -> (u16, u16) {
        self.def().map_or((0, 0), |d| (d.width, d.height))
    }

    /// The open map's levels, lowest first.
    #[must_use]
    pub fn levels(&self) -> Vec<Level> {
        self.session
            .document()
            .map(|d| d.levels())
            .unwrap_or_default()
    }

    /// The shown level's grid.
    #[must_use]
    pub fn grid(&self) -> Option<&Grid> {
        self.session.document().and_then(|d| d.grid(self.level))
    }

    /// The session's state against `data`.
    #[must_use]
    pub fn status(&self, data: &Data) -> EditorStatus {
        self.session.status(data)
    }

    /// Put the cursor on `cell` when it is on the map.
    pub fn point(&mut self, cell: (u16, u16)) {
        let (w, h) = self.size();
        if cell.0 < w && cell.1 < h && self.cursor != cell {
            self.cursor = cell;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Move the cursor one cell `facing`, when that stays on the map.
    pub fn step(&mut self, facing: Facing) {
        let (dx, dy) = facing.delta();
        let next = (
            u16::try_from(i32::from(self.cursor.0) + dx).ok(),
            u16::try_from(i32::from(self.cursor.1) + dy).ok(),
        );
        if let (Some(x), Some(y)) = next {
            self.point((x, y));
        }
    }

    /// Show `level`, when the map has it.
    pub fn show_level(&mut self, level: Level) {
        if self.levels().contains(&level) && self.level != level {
            self.level = level;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// The plan's pane changed size: a map just opened is fitted to it.
    pub fn laid_out(&mut self, pane: (u32, u32)) {
        if pane != self.pane && pane.0 > 0 && pane.1 > 0 {
            let fit = self.view == plan::View::fitted(self.size(), self.pane);
            self.pane = pane;
            if fit {
                self.view = plan::View::fitted(self.size(), pane);
            }
            self.revision = self.revision.wrapping_add(1);
        }
    }

    /// Use a control of the layer: a press (`step` 0) or a choice's arrow (`step` −1 or 1).
    pub fn apply(&mut self, data: &Data, act: EditAct, step: i8) -> Asked {
        self.revision = self.revision.wrapping_add(1);
        match act {
            EditAct::Map => {
                let n = self.maps.len().max(1);
                let at = i64::try_from(self.chosen).unwrap_or(0) + i64::from(step);
                self.chosen =
                    usize::try_from(at.rem_euclid(i64::try_from(n).unwrap_or(1))).unwrap_or(0);
            }
            EditAct::Open => {
                let id = self.maps.get(self.chosen).map(|m| m.id.clone());
                match id {
                    Some(id) if !id.is_empty() => {
                        if let Err(e) = self.open(&id, data) {
                            self.message = Some(e.to_string());
                        }
                    }
                    _ => self.message = Some("that file declares no map".to_owned()),
                }
            }
            EditAct::Level => {
                let levels = self.levels();
                if let Some(i) = levels.iter().position(|l| *l == self.level) {
                    let n = i64::try_from(levels.len()).unwrap_or(1);
                    let at = (i64::try_from(i).unwrap_or(0) + i64::from(step)).rem_euclid(n);
                    if let Some(level) = usize::try_from(at).ok().and_then(|i| levels.get(i)) {
                        self.level = *level;
                    }
                }
            }
            EditAct::ZoomIn => self.view = self.view.zoomed(1, self.pane),
            EditAct::ZoomOut => self.view = self.view.zoomed(-1, self.pane),
            EditAct::Centre => self.view = self.view.centred_on(self.cursor, self.pane),
            EditAct::FaceLeft => self.facing = self.facing.left(),
            EditAct::FaceRight => self.facing = self.facing.right(),
            EditAct::Menu => return Asked::Menu,
        }
        Asked::Nothing
    }
}
