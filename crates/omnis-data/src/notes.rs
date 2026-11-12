//! An author's notes on a pack's content (M5.1; owner, 2026-10-21): files under `notes/`, each a
//! table from a key to a text, in the shape of `text/<lang>/`. A key names anything in the packs:
//! a content id (`base:item:mace`, `test:map:town`), or a content id followed by `/` and a path
//! inside it (`test:map:town/0/3,1` the cell (3, 1) of level 0, `test:map:town/level/-1`,
//! `test:map:town/chest.1` a placement, `test:map:town/terrain/r`). The editor reads and shows
//! them; the game never does, and the pack seal does not cover them, so writing a note changes
//! no fingerprint. The loader checks a notes file as it checks every file (schema, key shape,
//! limits) but whether a key's target exists is the editor's to show: a game master may write
//! about what is planned before it is built.

use crate::error::DataError;
use crate::limits::{MAX_COLLECTION, string_fits};
use crate::manifest::is_content_id;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// One `notes/*.ron` file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotesFile {
    /// Schema version of this file.
    pub schema: u32,
    /// Note key to the note's text, at most the string limit each.
    pub entries: BTreeMap<String, String>,
}

/// Whether `key` names something a note may hang on: a content id, or a content id followed by
/// `/` and a non-empty path of `[A-Za-z0-9_.,:/-]`.
#[must_use]
pub fn is_note_key(key: &str) -> bool {
    if !string_fits(key) {
        return false;
    }
    match key.split_once('/') {
        None => is_content_id(key),
        Some((id, path)) => {
            is_content_id(id)
                && !path.is_empty()
                && path.bytes().all(|b| {
                    b.is_ascii_alphanumeric()
                        || matches!(b, b'_' | b'.' | b',' | b':' | b'/' | b'-')
                })
        }
    }
}

impl NotesFile {
    /// Every key names something, every text fits, and the table is bounded.
    pub fn validate(&self, file: &Path, errors: &mut Vec<DataError>) {
        if self.entries.len() > MAX_COLLECTION {
            errors.push(DataError::new(file, "too many entries"));
            return;
        }
        for (key, text) in &self.entries {
            if !is_note_key(key) {
                errors.push(DataError::new(
                    file,
                    format!(
                        "note key '{key}' is not a content id, or one followed by '/' and a path"
                    ),
                ));
            }
            if !string_fits(text) {
                errors.push(DataError::new(
                    file,
                    format!("note '{key}' is over the size limit"),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_content_ids_with_an_optional_path() {
        for key in [
            "base:item:mace",
            "test:map:town",
            "test:map:town/0/3,1",
            "test:map:town/level/-1",
            "test:map:town/chest.1",
            "test:map:town/terrain/r",
        ] {
            assert!(is_note_key(key), "{key}");
        }
        for key in [
            "",
            "town",
            "Test:map:town",
            "test:map:town/",
            "/test:map:town",
            "test:map:town/0/3 1",
            "test:map:town/level/-1\n",
        ] {
            assert!(!is_note_key(key), "{key:?}");
        }
        assert!(!is_note_key(&format!("test:map:town/{}", "a".repeat(4096))));
    }

    #[test]
    fn a_file_reports_every_bad_key_and_oversized_text() {
        let mut notes = NotesFile::default();
        notes.entries.insert("bad key".into(), "fine".into());
        notes
            .entries
            .insert("test:map:town".into(), "n".repeat(4097));
        let mut errors = Vec::new();
        notes.validate(Path::new("notes/a.ron"), &mut errors);
        let messages: Vec<&str> = errors.iter().map(|e| e.message.as_str()).collect();
        assert_eq!(
            messages,
            [
                "note key 'bad key' is not a content id, or one followed by '/' and a path",
                "note 'test:map:town' is over the size limit",
            ]
        );
    }
}
