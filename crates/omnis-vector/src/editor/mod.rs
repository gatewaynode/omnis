//! The editor's model (M5 Editor v1 from M5.4; ARCHITECTURE §8.2, `tasks/plans/m5-editor-v1.md`
//! §Client side): the document session over the pack being edited, the open map's level,
//! cursor and facing, the plan view, the brush (M5.5) and the acts the layer's controls ask
//! for. Bevy-free: the shell (`shell/editor.rs`) draws it and routes the pointer and the keys
//! to it.

pub mod brush;
pub mod plan;
pub mod view;

use brush::{Target, Tool};
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
    /// Choose a brush of the palette, by its position (M5.5).
    Brush(u8),
    /// The height choice: steps the height brush's eighths.
    Eighths,
    /// Back one step.
    Undo,
    /// Forward one step.
    Redo,
    /// Write the map to its file and reseal the pack.
    Save,
    /// Reopen the map's file, dropping every unsaved change.
    Revert,
    /// Open the editor's menu.
    Menu,
}

/// What the layer does after an act: nothing more, open the menu, or reload the packs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asked {
    /// Nothing beyond the model's change.
    Nothing,
    /// Open the editor's menu.
    Menu,
    /// The map was written: the packs are read from disk again, so a new game walks it.
    Saved,
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
    /// The way the preview looks, and the side an edge brush paints from the keys.
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
    /// The brush a click paints with.
    pub tool: Tool,
    /// The height brush's height, in eighths.
    pub eighths: i16,
    /// The last target painted in the stroke the held button keeps open; none between strokes.
    pub stroke: Option<Target>,
    /// The session's state against the packs, as of the last change (the loader's verdict is
    /// costly, so it is read here, never asked for each frame).
    pub status: EditorStatus,
}

impl Editor {
    /// The editor over `roots` (the last is edited), listing its maps.
    ///
    /// # Errors
    /// The pack's manifest or notes do not read, or its maps cannot be listed.
    pub fn new(roots: Vec<PathBuf>) -> Result<Editor, EditError> {
        let session = Session::new(roots)?;
        let maps = session.maps()?;
        let status = EditorStatus {
            root: session.root().display().to_string(),
            ..EditorStatus::default()
        };
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
            tool: Tool::Clear,
            eighths: 0,
            stroke: None,
            status,
        })
    }

    /// Open the map `id`, putting the cursor at its start, fitting the plan to the pane and
    /// taking its first terrain as the brush.
    ///
    /// # Errors
    /// The session's: no such map, or its layout does not read.
    pub fn open(&mut self, id: &str, data: &Data) -> Result<(), EditError> {
        self.session.open(id)?;
        if let Some(i) = self.maps.iter().position(|m| m.id == id) {
            self.chosen = i;
        }
        let def = self.def();
        let (start, size) = def.as_ref().map_or(((0, 0, Facing::North), (0, 0)), |d| {
            (d.start, (d.width, d.height))
        });
        self.level = Level::GROUND;
        self.cursor = (start.0, start.1);
        self.facing = start.2;
        self.view = plan::View::fitted(size, self.pane);
        self.tool = def.as_ref().map_or(Tool::Clear, Tool::first);
        self.stroke = None;
        self.message = None;
        self.resolved = None;
        self.changed(data);
        Ok(())
    }

    /// The document changed (by the layer or over the socket): read its status, resolve it
    /// again, keeping the last valid resolve and saying why when the loader refuses it, and
    /// ask for a redraw.
    pub fn changed(&mut self, data: &Data) {
        self.revision = self.revision.wrapping_add(1);
        self.status = self.session.status(data);
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

    /// Paint the brush at `target` on the shown level and put the cursor on its cell. With
    /// `stroke` the paint joins the stroke the held button keeps open, so a drag undoes as
    /// one. A refusal is said and changes nothing. Whether the map changed.
    pub fn paint(&mut self, data: &Data, target: Target, stroke: bool) -> bool {
        let Some(def) = self.def() else {
            self.message = Some("open a map first".to_owned());
            return false;
        };
        let Some(brush) = self.tool.brush(&def, target, self.facing, self.eighths) else {
            return false;
        };
        let (level, (x, y)) = (self.level, target.cell);
        let joined = stroke && self.stroke.is_some();
        let result = self
            .session
            .document_mut()
            .and_then(|d| d.paint(level, x, y, &brush, joined));
        // The same target is not painted again while the button is held, even one refused.
        self.stroke = Some(target);
        match result {
            Ok(()) => {
                self.message = None;
                self.cursor = target.cell;
                self.changed(data);
                true
            }
            Err(e) => {
                self.message = Some(e.to_string());
                false
            }
        }
    }

    /// The button is up: the next paint is a new stroke.
    pub fn end_stroke(&mut self) {
        self.stroke = None;
    }

    /// Take what is under `target` as the brush: the edge's kind, the cell's terrain or no
    /// ground, and the cell's height; the cursor goes there. Never changes the map.
    pub fn pick(&mut self, target: Target) {
        let Some(grid) = self.grid() else {
            return;
        };
        let (x, y) = target.cell;
        let tool = match target.edge {
            Some(side) => Tool::Edge(grid.edge(x, y, side).unwrap_or_default()),
            None => match grid.tile(x, y).flatten() {
                Some(t) => Tool::Terrain(t),
                None => Tool::Clear,
            },
        };
        if let Some(height) = grid.height_at(x, y) {
            self.eighths = brush::held(height);
        }
        self.tool = tool;
        self.message = None;
        self.point(target.cell);
        self.revision = self.revision.wrapping_add(1);
    }

    /// Back or forward one step of the document's history.
    fn walk(&mut self, data: &Data, back: bool) {
        let result = self
            .session
            .document_mut()
            .and_then(|d| if back { d.undo() } else { d.redo() });
        self.stroke = None;
        match result {
            Ok(()) => {
                self.message = None;
                self.changed(data);
            }
            Err(e) => self.message = Some(e.to_string()),
        }
    }

    /// Write the map and reseal the pack, or say why not.
    fn save(&mut self, data: &Data) -> Asked {
        let file = self
            .session
            .document()
            .map(|d| d.file().to_owned())
            .unwrap_or_default();
        match self.session.save(data) {
            Ok(sealed) => {
                self.message = Some(format!(
                    "Saved {file}; {} sealed at {}",
                    self.session.pack(),
                    sealed.version
                ));
                self.stroke = None;
                self.changed(data);
                Asked::Saved
            }
            Err(EditError::Invalid(errors)) => {
                let first = errors.first().map(ToString::to_string).unwrap_or_default();
                self.message = Some(format!(
                    "Not saved: {} error{}; the first: {first}",
                    errors.len(),
                    if errors.len() == 1 { "" } else { "s" }
                ));
                Asked::Nothing
            }
            Err(e) => {
                self.message = Some(format!("Not saved: {e}"));
                Asked::Nothing
            }
        }
    }

    /// Reopen the map's file as it is on disk, keeping the cursor, the level and the view.
    fn revert(&mut self, data: &Data) {
        let Some(id) = self.map_id() else {
            return;
        };
        let (cursor, level, facing, view, tool, eighths) = (
            self.cursor,
            self.level,
            self.facing,
            self.view,
            self.tool,
            self.eighths,
        );
        match self.open(&id, data) {
            Ok(()) => {
                self.cursor = cursor;
                self.facing = facing;
                self.view = view;
                self.tool = tool;
                self.eighths = eighths;
                self.show_level(level);
                self.changed(data);
                self.message = Some(format!("Reverted {id} to its file"));
            }
            Err(e) => self.message = Some(format!("Not reverted: {e}")),
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
            EditAct::Brush(i) => {
                if let Some(tool) = self
                    .def()
                    .and_then(|def| brush::palette(&def).get(usize::from(i)).copied())
                {
                    self.tool = tool;
                }
            }
            EditAct::Eighths => {
                // A press steps right, as a choice does.
                let by = if step == 0 { 1 } else { i16::from(step) };
                self.eighths = brush::held(self.eighths.saturating_add(by));
            }
            EditAct::Undo => self.walk(data, true),
            EditAct::Redo => self.walk(data, false),
            EditAct::Save => return self.save(data),
            EditAct::Revert => self.revert(data),
            EditAct::Menu => return Asked::Menu,
        }
        Asked::Nothing
    }
}
