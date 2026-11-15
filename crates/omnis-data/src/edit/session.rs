//! The editor's session over a pack: the roots loaded (the last is edited), the open map, the
//! notes, and the files. A save writes the map and reseals the pack, raising its patch version
//! (owner, 2026-10-21), so the game loads what the editor wrote; a save is refused while the
//! loader would refuse the map, and nothing is written then.

use crate::edit::document::Document;
use crate::edit::extend::Losses;
use crate::edit::files::{DIRECTORIES, check_text};
use crate::edit::kinds::{Edit, EditError, EditorStatus, MapEntry, NewMap};
use crate::edit::notes_store::Notes;
use crate::error::DataError;
use crate::loader::{self, Data};
use crate::manifest::{PackManifest, is_content_id};
use crate::map::MapDef;
use crate::ron_io::{list_under, read_under, to_string, write_files};
use crate::seal::{self, Sealed};
use std::path::{Path, PathBuf};

/// A pack being edited among the packs it loads with.
#[derive(Debug, Clone)]
pub struct Session {
    roots: Vec<PathBuf>,
    pack: String,
    document: Option<Document>,
    notes: Notes,
}

impl Session {
    /// A session over `roots`, in load order; the last is the pack edited.
    ///
    /// # Errors
    /// No root, or the edited pack's manifest or notes do not read (`Io`).
    pub fn new(roots: Vec<PathBuf>) -> Result<Session, EditError> {
        let root = roots
            .last()
            .ok_or_else(|| EditError::Io(DataError::new("packs", "no pack root to edit")))?;
        let text = read_under(root, "pack.ron")?;
        let manifest: PackManifest = crate::ron_io::from_str(&text, Path::new("pack.ron"))?;
        let notes = Notes::read(root)?;
        Ok(Session {
            pack: manifest.id,
            document: None,
            notes,
            roots,
        })
    }

    /// The pack root being edited.
    #[must_use]
    pub fn root(&self) -> &Path {
        self.roots.last().map_or(Path::new(""), PathBuf::as_path)
    }

    /// Every root, in load order.
    #[must_use]
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// The edited pack's id.
    #[must_use]
    pub fn pack(&self) -> &str {
        &self.pack
    }

    /// The open map, if one is.
    #[must_use]
    pub fn document(&self) -> Option<&Document> {
        self.document.as_ref()
    }

    /// The open map, to change.
    ///
    /// # Errors
    /// `NoDocument`.
    pub fn document_mut(&mut self) -> Result<&mut Document, EditError> {
        self.document.as_mut().ok_or(EditError::NoDocument)
    }

    /// Close the open map, dropping unsaved changes.
    pub fn close(&mut self) {
        self.document = None;
    }

    /// The author's notes.
    #[must_use]
    pub fn notes(&self) -> &Notes {
        &self.notes
    }

    /// Write a note, or take it away with `None`; written at once, never sealed.
    ///
    /// # Errors
    /// The store's: `BadId`, `OutOfRange`, `TooMany`, `Io`.
    pub fn set_note(&mut self, key: &str, text: Option<&str>) -> Result<(), EditError> {
        let root = self.root().to_path_buf();
        self.notes.set(&root, key, text)
    }

    /// What `edit` would destroy in the open map ([`Document::losses`]), with the notes written
    /// on cells it cuts.
    ///
    /// # Errors
    /// `NoDocument`; `OutOfRange` for a size the map may not have.
    pub fn losses(&self, edit: &Edit) -> Result<Losses, EditError> {
        let doc = self.document.as_ref().ok_or(EditError::NoDocument)?;
        let mut lost = doc.losses(edit)?;
        let def = doc.def();
        let mut after = doc.clone();
        after.edit(edit.clone())?;
        let (by, was) = doc.moved_since_saved();
        let (then, size) = (after.moved_since_saved().0, {
            let d = after.def();
            (d.width, d.height)
        });
        let now = (def.width, def.height);
        let cells = self.notes.on_cells(&def.id);
        let cut = cells.iter().filter(|(_, _, at)| {
            let had = at.0 < was.0 && at.1 < was.1;
            had && moved(*at, by, now).is_some() && moved(*at, then, size).is_none()
        });
        lost.notes = u32::try_from(cut.count()).unwrap_or(u32::MAX);
        Ok(lost)
    }

    /// The notes whose target is nowhere in `data` nor in the open map.
    #[must_use]
    pub fn orphaned_notes(&self, data: &Data) -> Vec<String> {
        self.notes.orphans(data, self.document.as_ref())
    }

    /// The pack's map files, with the id each declares or why it does not read.
    ///
    /// # Errors
    /// The directory cannot be listed (`Io`).
    pub fn maps(&self) -> Result<Vec<MapEntry>, EditError> {
        let root = self.root();
        let mut out = Vec::new();
        for file in list_under(root, "data/maps")? {
            let mut errors = Vec::new();
            let id = read_under(root, &file)
                .ok()
                .and_then(|text| {
                    loader::parse_checked::<MapDef>(&text, Path::new(&file), &mut errors)
                })
                .map(|def| def.id);
            out.push(MapEntry {
                id: id.clone().unwrap_or_default(),
                file,
                error: match id {
                    Some(_) => None,
                    None => Some(
                        errors
                            .first()
                            .map_or_else(|| "cannot read".to_owned(), |e| e.message.clone()),
                    ),
                },
            });
        }
        Ok(out)
    }

    /// Open the map with this id, by its file.
    ///
    /// # Errors
    /// `NoSuchTarget` when no file declares the id; the loader's errors when its layout does not
    /// read (`Invalid`).
    pub fn open(&mut self, id: &str) -> Result<(), EditError> {
        let entry = self
            .maps()?
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| EditError::NoSuchTarget(format!("map '{id}'")))?;
        let text = read_under(self.root(), &entry.file)?;
        let mut errors = Vec::new();
        let def = loader::parse_checked::<MapDef>(&text, Path::new(&entry.file), &mut errors)
            .ok_or_else(|| EditError::Invalid(errors.clone()))?;
        self.document = Some(Document::from_def(&entry.file, def).map_err(EditError::Invalid)?);
        Ok(())
    }

    /// Start a new map in this pack: its id is `<pack>:map:<name>` and its file
    /// `data/maps/<name>.ron`, which must not exist yet. Open but unsaved.
    ///
    /// # Errors
    /// `BadId` for an id of another shape or another pack; `Exists` when the file is there.
    pub fn new_map(&mut self, new: NewMap) -> Result<(), EditError> {
        let mut parts = new.id.split(':');
        let (pack, kind, name) = (parts.next(), parts.next(), parts.next());
        if !is_content_id(&new.id) || pack != Some(self.pack.as_str()) || kind != Some("map") {
            return Err(EditError::BadId(new.id.clone()));
        }
        let file = format!("data/maps/{}.ron", name.unwrap_or_default());
        if self.root().join(&file).exists() {
            return Err(EditError::Exists(file));
        }
        self.document = Some(Document::blank(&file, new));
        Ok(())
    }

    /// The session as a client reads it: the open map's shape and whether the loader would
    /// take it against `data`.
    #[must_use]
    pub fn status(&self, data: &Data) -> EditorStatus {
        let mut status = EditorStatus {
            root: self.root().display().to_string(),
            ..EditorStatus::default()
        };
        if let Some(doc) = &self.document {
            let def = doc.def();
            status.map = Some(def.id.clone());
            status.file = Some(doc.file().to_owned());
            status.width = def.width;
            status.height = def.height;
            status.levels = doc.levels().iter().map(|l| l.0).collect();
            status.terrains = def
                .terrains
                .iter()
                .map(|t| (t.glyph, t.name.clone()))
                .collect();
            status.dirty = doc.dirty();
            status.undo = doc.undo_depth();
            status.redo = doc.redo_depth();
            status.errors = doc
                .validate(Some(data))
                .into_iter()
                .map(|e| e.to_string())
                .collect();
        }
        status
    }

    /// Write the open map to its file and reseal the pack, raising its patch version.
    ///
    /// # Errors
    /// `NoDocument`; `Invalid` with the loader's words when the map would not load, and nothing
    /// is written; `Io` when the file or the seal cannot be written.
    pub fn save(&mut self, data: &Data) -> Result<Sealed, EditError> {
        let root = self.root().to_path_buf();
        let doc = self.document.as_mut().ok_or(EditError::NoDocument)?;
        let errors = doc.validate(Some(data));
        if !errors.is_empty() {
            return Err(EditError::Invalid(errors));
        }
        let def = doc.def();
        let text = to_string(&def)?;
        write_files(&root, &[(doc.file(), &text, false)])?;
        // Notes written on cells follow them: by the cells added or cut on the west and north
        // since the file was last written, and gone with a cell that is.
        let (by, was) = doc.moved_since_saved();
        let size = (def.width, def.height);
        doc.mark_saved();
        self.notes.follow(
            &root,
            &def.id,
            |(x, y)| x < was.0 && y < was.1,
            |cell| moved(cell, by, size),
        )?;
        Ok(seal::seal(&root, None)?)
    }

    /// Every file the loader reads, root-relative: `pack.ron` and the `.ron` files under
    /// `data/`, `text/`, `notes/` and `migrations/`.
    ///
    /// # Errors
    /// A directory cannot be listed (`Io`).
    pub fn files(&self) -> Result<Vec<String>, EditError> {
        let mut out = vec!["pack.ron".to_owned()];
        for dir in DIRECTORIES {
            out.extend(list_under(self.root(), dir)?);
        }
        Ok(out)
    }

    /// A file's text, root-relative.
    ///
    /// # Errors
    /// Not a `.ron` file under the root, or unreadable (`Io`).
    pub fn read(&self, rel: &str) -> Result<String, EditError> {
        Ok(read_under(self.root(), rel)?)
    }

    /// What the loader would say of `text` as the file at `rel`.
    ///
    /// # Errors
    /// `UnknownDirectory`.
    pub fn check(&self, rel: &str, text: &str, data: &Data) -> Result<Vec<DataError>, EditError> {
        check_text(rel, text, data)
    }

    /// Write `text` as the file at `rel`, exactly as typed, once the loader's check passes;
    /// then reseal the pack, unless the file is a note, which the seal does not cover.
    ///
    /// # Errors
    /// `UnknownDirectory`; `Invalid` with the loader's words, and nothing is written; `Io`.
    pub fn write(
        &mut self,
        rel: &str,
        text: &str,
        data: &Data,
    ) -> Result<Option<Sealed>, EditError> {
        let errors = self.check(rel, text, data)?;
        if !errors.is_empty() {
            return Err(EditError::Invalid(errors));
        }
        let root = self.root().to_path_buf();
        write_files(&root, &[(rel, text, false)])?;
        if rel.starts_with("notes/") {
            self.notes = Notes::read(&root)?;
            return Ok(None);
        }
        Ok(Some(seal::seal(&root, None)?))
    }
}

/// Where a cell lies after every cell moved `by` (east, south) on a map of `size`, when it is
/// still on it.
fn moved((x, y): (u16, u16), by: (i32, i32), size: (u16, u16)) -> Option<(u16, u16)> {
    let x = u16::try_from(i32::from(x) + by.0).ok()?;
    let y = u16::try_from(i32::from(y) + by.1).ok()?;
    (x < size.0 && y < size.1).then_some((x, y))
}
