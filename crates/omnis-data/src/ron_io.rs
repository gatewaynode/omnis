//! RON read and write. The loader and the editor share this path so anything the editor writes
//! the game loads (PRD §10). Reads are bounded: file size, recursion depth.

use crate::error::DataError;
use crate::limits::{MAX_FILE_BYTES, RON_RECURSION_LIMIT};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::path::Path;

fn options() -> ron::Options {
    ron::Options::default().with_recursion_limit(RON_RECURSION_LIMIT)
}

/// Parse a RON document. Errors carry the line.
pub fn from_str<T: DeserializeOwned>(text: &str, file: &Path) -> Result<T, DataError> {
    options()
        .from_str(text)
        .map_err(|e| DataError::at(file, e.span.start.line, format!("parse error: {}", e.code)))
}

/// Parse a RON document held in memory (a save, a socket message).
pub fn parse<T: DeserializeOwned>(text: &str) -> Result<T, DataError> {
    from_str(text, Path::new("<memory>"))
}

/// Render a value as pretty RON, the way the editor writes it.
pub fn to_string<T: Serialize>(value: &T) -> Result<String, DataError> {
    let config = ron::ser::PrettyConfig::new()
        .depth_limit(RON_RECURSION_LIMIT)
        .struct_names(false);
    options()
        .to_string_pretty(value, config)
        .map_err(|e| DataError::new("<memory>", format!("serialize error: {e}")))
}

/// Read a file's text with the size cap, refusing symlinks so a pack cannot point outside
/// its root.
pub fn read_text(path: &Path, display: &Path) -> Result<String, DataError> {
    let meta = std::fs::symlink_metadata(path)
        .map_err(|e| DataError::new(display, format!("cannot read: {e}")))?;
    if meta.file_type().is_symlink() {
        return Err(DataError::new(display, "symlinks are not allowed in packs"));
    }
    if meta.len() > MAX_FILE_BYTES {
        return Err(DataError::new(
            display,
            format!("file is {} bytes; limit is {MAX_FILE_BYTES}", meta.len()),
        ));
    }
    std::fs::read_to_string(path).map_err(|e| DataError::new(display, format!("cannot read: {e}")))
}

/// Read and parse one RON file.
pub fn read_ron<T: DeserializeOwned>(path: &Path, display: &Path) -> Result<T, DataError> {
    let text = read_text(path, display)?;
    from_str(&text, display)
}

/// Write a value as pretty RON, creating parent directories.
pub fn write_ron<T: Serialize>(path: &Path, value: &T) -> Result<(), DataError> {
    let text = to_string(value)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| DataError::new(path, format!("cannot create directory: {e}")))?;
    }
    std::fs::write(path, text).map_err(|e| DataError::new(path, format!("cannot write: {e}")))
}
