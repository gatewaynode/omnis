//! `pack.ron`: what a pack is, who made it, and what it needs.

use serde::{Deserialize, Serialize};

/// One credit line: `(source, license, text)` (ARCHITECTURE.md §6.3).
pub type Attribution = (String, String, String);

/// The pack manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackManifest {
    /// Schema version of this file.
    pub schema: u32,
    /// The pack id, the first segment of every id the pack defines.
    pub id: String,
    /// Semantic version string.
    pub version: String,
    /// Display name.
    pub name: String,
    /// SPDX licence expression for the pack's own content.
    pub license: String,
    /// Credits rendered on the credits screen.
    #[serde(default)]
    pub attribution: Vec<Attribution>,
    /// Ids of packs that must load first.
    #[serde(default)]
    pub depends: Vec<String>,
}

/// Whether `id` is a well-formed pack id: `[a-z0-9_-]+`.
#[must_use]
pub fn is_pack_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
}

/// Whether `id` is a well-formed content id: `pack:type:name`, each segment `[a-z0-9_.-]+`.
#[must_use]
pub fn is_content_id(id: &str) -> bool {
    let mut segments = 0;
    for segment in id.split(':') {
        segments += 1;
        if segment.is_empty()
            || !segment.bytes().all(|b| {
                b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')
            })
        {
            return false;
        }
    }
    segments == 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_shapes() {
        assert!(is_pack_id("test"));
        assert!(is_pack_id("my-mod_2"));
        assert!(!is_pack_id(""));
        assert!(!is_pack_id("Test"));
        assert!(!is_pack_id("a:b"));
        assert!(is_content_id("test:map:dungeon"));
        assert!(is_content_id("base:text:map.dungeon.name"));
        assert!(!is_content_id("test:map"));
        assert!(!is_content_id("test:map:"));
        assert!(!is_content_id("test:Map:x"));
        assert!(!is_content_id("a:b:c:d"));
    }
}
