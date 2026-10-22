//! The editor's ops on the wire (M5.3): each `editor.*` op becomes an `omnis_data::edit::Request`
//! with its arguments bounded, and an `Answer` or an `EditError` comes back as a `Reply` or an
//! `OpError`. The simulation has no files, so every editor op is a host op; a host is three
//! lines over an `edit::Session` (`omnis-cli`'s `Headless`, the game's socket).

use crate::ops::{Op, OpError, Reply, bounded, bounded_file, client_path};
use alloc::borrow::ToOwned;
use alloc::string::ToString;
use alloc::vec;
use omnis_data::edit::{Answer, EditError, Request};

impl Op {
    /// The editor's request this op makes, its strings and paths bounded; `None` for every
    /// other op. The strings inside a `Place`, an `Edit` or a `NewMap` are the loader's to
    /// bound, in `validate`.
    pub fn editor(&self) -> Option<Result<Request, OpError>> {
        let path_text = |path: &str, text: &str| -> Result<(), OpError> {
            client_path(path, &["ron"])?;
            bounded_file(text)
        };
        Some(match self {
            Op::EditorStatus => Ok(Request::Status),
            Op::EditorMaps => Ok(Request::Maps),
            Op::EditorOpen { map } => bounded(map).map(|()| Request::Open { map: map.clone() }),
            Op::EditorNew { map } => bounded(&map.id).map(|()| Request::New { map: map.clone() }),
            Op::EditorPaint {
                level,
                x,
                y,
                brush,
                stroke,
            } => Ok(Request::Paint {
                level: *level,
                x: *x,
                y: *y,
                brush: brush.clone(),
                stroke: *stroke,
            }),
            Op::EditorPlace { place } => Ok(Request::Place {
                place: place.clone(),
            }),
            Op::EditorEdit { edit } => Ok(Request::Edit { edit: edit.clone() }),
            Op::EditorUndo => Ok(Request::Undo),
            Op::EditorRedo => Ok(Request::Redo),
            Op::EditorSave => Ok(Request::Save),
            Op::EditorFiles => Ok(Request::Files),
            Op::EditorRead { path } => {
                client_path(path, &["ron"]).map(|p| Request::Read { path: p.to_owned() })
            }
            Op::EditorCheck { path, text } => path_text(path, text).map(|()| Request::Check {
                path: path.clone(),
                text: text.clone(),
            }),
            Op::EditorWrite { path, text } => path_text(path, text).map(|()| Request::Write {
                path: path.clone(),
                text: text.clone(),
            }),
            Op::EditorNote { key } => bounded(key).map(|()| Request::Note { key: key.clone() }),
            Op::EditorNoteSet { key, text } => bounded(key)
                .and_then(|()| text.as_deref().map_or(Ok(()), bounded))
                .map(|()| Request::NoteSet {
                    key: key.clone(),
                    text: text.clone(),
                }),
            Op::EditorNotes { prefix } => {
                prefix
                    .as_deref()
                    .map_or(Ok(()), bounded)
                    .map(|()| Request::Notes {
                        prefix: prefix.clone(),
                    })
            }
            _ => return None,
        })
    }
}

impl From<Answer> for Reply {
    fn from(answer: Answer) -> Self {
        match answer {
            Answer::Status(editor) => Reply::Editor { editor },
            Answer::Maps(maps) => Reply::Maps { maps },
            Answer::Files(files) => Reply::Files { files },
            Answer::File { path, text } => Reply::File { path, text },
            Answer::Checked(errors) => Reply::Checked { errors },
            Answer::Written { path, version } => Reply::Written { path, version },
            Answer::Note { key, text } => Reply::Note { key, text },
            Answer::Notes { keys, orphans } => Reply::Notes { keys, orphans },
        }
    }
}

impl From<EditError> for OpError {
    fn from(error: EditError) -> Self {
        match error {
            EditError::Io(e) => OpError::Failed {
                message: e.to_string(),
            },
            EditError::Invalid(errors) => OpError::Invalid {
                errors: errors.iter().map(ToString::to_string).collect(),
            },
            other => OpError::Invalid {
                errors: vec![other.to_string()],
            },
        }
    }
}
