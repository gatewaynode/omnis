//! The editor's requests and answers as one plain enum each, so every host (the CLI's
//! `Headless`, the game's socket) is three lines over a `Session`: the protocol's `editor.*` ops
//! become a `Request` in `omnis-sim`, which has no files, and come back as an `Answer`.

use crate::edit::kinds::{Brush, Edit, EditError, EditorStatus, MapEntry, NewMap, Place};
use crate::edit::session::Session;
use crate::loader::Data;
use omnis_core::Level;

/// What a client asks of the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// The session's state.
    Status,
    /// The pack's map files.
    Maps,
    /// Open a map by id.
    Open {
        /// Its id.
        map: String,
    },
    /// Start a new map.
    New {
        /// Its shape.
        map: NewMap,
    },
    /// Paint one tile or edge.
    Paint {
        /// Level.
        level: Level,
        /// Column.
        x: u16,
        /// Row.
        y: u16,
        /// With what.
        brush: Brush,
        /// Part of the open stroke.
        stroke: bool,
    },
    /// Put or remove a placement.
    Place {
        /// Which.
        place: Place,
    },
    /// Change the map's structure.
    Edit {
        /// Which.
        edit: Edit,
    },
    /// Back one step.
    Undo,
    /// Forward one step.
    Redo,
    /// Write the map and reseal the pack.
    Save,
    /// Every file the loader reads.
    Files,
    /// A file's text.
    Read {
        /// Root-relative.
        path: String,
    },
    /// What the loader would say of a text as a file.
    Check {
        /// Root-relative.
        path: String,
        /// The text.
        text: String,
    },
    /// Write a text as a file, as typed, once it checks clean.
    Write {
        /// Root-relative.
        path: String,
        /// The text.
        text: String,
    },
    /// The note on a key.
    Note {
        /// The key.
        key: String,
    },
    /// Write a note, or take it away.
    NoteSet {
        /// The key.
        key: String,
        /// The text, or none to take the note away.
        text: Option<String>,
    },
    /// The keys with notes, and the orphans among all of them.
    Notes {
        /// Only keys starting with this.
        prefix: Option<String>,
    },
}

/// What the editor answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// The session's state.
    Status(EditorStatus),
    /// The map files.
    Maps(Vec<MapEntry>),
    /// The files.
    Files(Vec<String>),
    /// A file's text.
    File {
        /// Root-relative.
        path: String,
        /// The text.
        text: String,
    },
    /// The loader's words, empty when clean.
    Checked(Vec<String>),
    /// A file written, and the pack's version when it was resealed.
    Written {
        /// Root-relative.
        path: String,
        /// The version the pack is sealed at, when the write resealed it.
        version: Option<String>,
    },
    /// A note.
    Note {
        /// The key.
        key: String,
        /// The text, if there is one.
        text: Option<String>,
    },
    /// Keys with notes, and the orphans.
    Notes {
        /// The keys, sorted.
        keys: Vec<String>,
        /// The keys whose target is nowhere, sorted.
        orphans: Vec<String>,
    },
}

/// Answer one request over the session, against the loaded `data`.
///
/// # Errors
/// The session's, by request.
pub fn answer(session: &mut Session, data: &Data, request: Request) -> Result<Answer, EditError> {
    Ok(match request {
        Request::Status => Answer::Status(session.status(data)),
        Request::Maps => Answer::Maps(session.maps()?),
        Request::Open { map } => {
            session.open(&map)?;
            Answer::Status(session.status(data))
        }
        Request::New { map } => {
            session.new_map(map)?;
            Answer::Status(session.status(data))
        }
        Request::Paint {
            level,
            x,
            y,
            brush,
            stroke,
        } => {
            session.document_mut()?.paint(level, x, y, &brush, stroke)?;
            Answer::Status(session.status(data))
        }
        Request::Place { place } => {
            session.document_mut()?.place(place)?;
            Answer::Status(session.status(data))
        }
        Request::Edit { edit } => {
            session.document_mut()?.edit(edit)?;
            Answer::Status(session.status(data))
        }
        Request::Undo => {
            session.document_mut()?.undo()?;
            Answer::Status(session.status(data))
        }
        Request::Redo => {
            session.document_mut()?.redo()?;
            Answer::Status(session.status(data))
        }
        Request::Save => {
            let file = session
                .document()
                .map(|d| d.file().to_owned())
                .ok_or(EditError::NoDocument)?;
            let sealed = session.save(data)?;
            Answer::Written {
                path: file,
                version: Some(sealed.version),
            }
        }
        Request::Files => Answer::Files(session.files()?),
        Request::Read { path } => Answer::File {
            text: session.read(&path)?,
            path,
        },
        Request::Check { path, text } => Answer::Checked(
            session
                .check(&path, &text, data)?
                .iter()
                .map(ToString::to_string)
                .collect(),
        ),
        Request::Write { path, text } => {
            let sealed = session.write(&path, &text, data)?;
            Answer::Written {
                path,
                version: sealed.map(|s| s.version),
            }
        }
        Request::Note { key } => Answer::Note {
            text: session.notes().get(&key).map(str::to_owned),
            key,
        },
        Request::NoteSet { key, text } => {
            session.set_note(&key, text.as_deref())?;
            Answer::Note { key, text }
        }
        Request::Notes { prefix } => Answer::Notes {
            keys: session
                .notes()
                .with_prefix(prefix.as_deref().unwrap_or(""))
                .into_iter()
                .map(|(k, _)| k.to_owned())
                .collect(),
            orphans: session.orphaned_notes(data),
        },
    })
}
