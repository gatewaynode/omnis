//! The editor's steps through the document's history and its way to the file: Undo and Redo,
//! Save, Revert. Moved out of `editor/mod.rs` to keep that file under a thousand lines.

use super::{Asked, Editor};
use omnis_sim::omnis_data::Data;
use omnis_sim::omnis_data::edit::EditError;

impl Editor {
    /// Back or forward one step of the document's history.
    pub(super) fn walk(&mut self, data: &Data, back: bool) {
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
    pub(super) fn save(&mut self, data: &Data) -> Asked {
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
    pub(super) fn revert(&mut self, data: &Data) {
        let Some(id) = self.map_id() else {
            return;
        };
        self.reopen(data);
        if self.message.is_none() {
            self.message = Some(format!("Reverted {id} to its file"));
        }
    }
}
