//! What the client said on stderr before it could show a window, kept in a file. On Windows the
//! `omnis` binary has no console (`windows_subsystem = "windows"` in `main.rs`), so a start that
//! fails leaves its reason here instead (Windows CI, 2026-11-08).

use std::io;
use std::path::{Path, PathBuf};

/// What the client last said at its start, under the working directory beside the saves.
pub const LAST_ERROR: &str = ".omnis/last-error.txt";

/// Write `line` (the loader's words may span lines) as the client's last word, over the one
/// before, and return the file's path.
///
/// # Errors
/// The `.omnis` directory cannot be made or the file not written.
pub fn record(root: &Path, line: &str) -> io::Result<PathBuf> {
    let path = root.join(LAST_ERROR);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, format!("omnis: {line}\n"))?;
    Ok(path)
}
