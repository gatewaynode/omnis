//! Input limits and path rules for packs and saves (ARCHITECTURE.md §6.2). Everything that
//! reaches the loader is untrusted.

use std::path::{Component, Path};

/// Largest text file the loader reads.
pub const MAX_FILE_BYTES: u64 = 4 * 1024 * 1024;
/// Largest total of data files in one pack.
pub const MAX_PACK_BYTES: u64 = 256 * 1024 * 1024;
/// Largest map side in tiles.
pub const MAX_MAP_SIDE: u16 = 256;
/// Largest collection in any data file.
pub const MAX_COLLECTION: usize = 65_536;
/// Largest string in any data file, in bytes.
pub const MAX_STRING_BYTES: usize = 4 * 1024;
/// Nesting allowed when parsing RON.
pub const RON_RECURSION_LIMIT: usize = 64;
/// Largest tileset detail depth (PRD §7.2: roughly 4 to 6).
pub const MAX_DETAIL_DEPTH: u8 = 8;
/// Largest visibility depth a tile may declare (PRD §7.2: an open plain reaches 20).
pub const MAX_VISIBILITY_DEPTH: u8 = 32;
/// Stacks an encounter may hold: the combat screen lists four.
pub const MAX_STACKS: usize = 4;
/// Image extensions a pack may reference.
pub const IMAGE_EXTENSIONS: [&str; 1] = ["png"];

/// Why a pack-relative path was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathFault {
    /// Empty path.
    Empty,
    /// Starts with a root or a drive prefix.
    Absolute,
    /// Contains a `..` or `.` component.
    Traversal,
    /// Contains a byte outside `[A-Za-z0-9._/-]`.
    Character,
    /// The extension is not on the whitelist.
    Extension,
}

impl PathFault {
    /// A short reason for an error message.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            PathFault::Empty => "path is empty",
            PathFault::Absolute => "path must be relative to the pack",
            PathFault::Traversal => "path must not contain '.' or '..' components",
            PathFault::Character => "path may only use letters, digits, '.', '_', '-', and '/'",
            PathFault::Extension => "file type is not allowed",
        }
    }
}

/// Check a pack-relative asset path: relative, no traversal, plain characters, allowed
/// extension. Symlinks are refused separately when the file is opened.
pub fn check_asset_path(path: &str, extensions: &[&str]) -> Result<(), PathFault> {
    if path.is_empty() {
        return Err(PathFault::Empty);
    }
    if !path
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-' | b'/'))
    {
        return Err(PathFault::Character);
    }
    let p = Path::new(path);
    for component in p.components() {
        match component {
            Component::Normal(_) => {}
            Component::RootDir | Component::Prefix(_) => return Err(PathFault::Absolute),
            Component::CurDir | Component::ParentDir => return Err(PathFault::Traversal),
        }
    }
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !extensions.contains(&ext) {
        return Err(PathFault::Extension);
    }
    Ok(())
}

/// Whether `s` fits the per-string limit.
#[must_use]
pub fn string_fits(s: &str) -> bool {
    s.len() <= MAX_STRING_BYTES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_paths_are_confined_to_the_pack() {
        let png = &IMAGE_EXTENSIONS;
        assert_eq!(check_asset_path("assets/tilesets/a/wall.png", png), Ok(()));
        assert_eq!(check_asset_path("", png), Err(PathFault::Empty));
        assert_eq!(
            check_asset_path("/etc/passwd.png", png),
            Err(PathFault::Absolute)
        );
        assert_eq!(check_asset_path("../x.png", png), Err(PathFault::Traversal));
        assert_eq!(
            check_asset_path("a/../x.png", png),
            Err(PathFault::Traversal)
        );
        assert_eq!(check_asset_path("./x.png", png), Err(PathFault::Traversal));
        assert_eq!(check_asset_path("a/x.exe", png), Err(PathFault::Extension));
        assert_eq!(check_asset_path("a/x", png), Err(PathFault::Extension));
        assert_eq!(check_asset_path("a b.png", png), Err(PathFault::Character));
        assert_eq!(check_asset_path("a\\b.png", png), Err(PathFault::Character));
    }
}
