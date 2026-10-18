//! The seal (M8.5 7h1): `pack.ron` records the hash of the pack's content at its version, so a
//! save made on a version names exactly the content it was made on. The loader refuses a pack
//! whose content no longer has the recorded hash; `seal` records it, raising the version when
//! the content changed since the last seal.

use crate::error::DataError;
use crate::loader::content_hash;
use crate::ron_io::{read_text, write_atomic};
use std::path::Path;

/// A pack's seal as it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// The version `pack.ron` names.
    pub version: String,
    /// The hash `pack.ron` records, if any.
    pub recorded: Option<u64>,
    /// The hash of the content now.
    pub hash: u64,
}

/// What `seal` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sealed {
    /// The version the pack is sealed at.
    pub version: String,
    /// The hash recorded.
    pub hash: u64,
    /// Whether `pack.ron` was written.
    pub changed: bool,
}

/// The seal of the pack at `root`, reading only.
///
/// # Errors
/// `pack.ron` does not read.
pub fn status(root: &Path) -> Result<Status, DataError> {
    let (manifest, hash) = content_hash(root)?;
    Ok(Status {
        version: manifest.version,
        recorded: (manifest.hash != 0).then_some(manifest.hash),
        hash,
    })
}

/// Seal the pack at `root`: record the hash of its content in `pack.ron`, editing only the
/// `version` and `hash` lines so comments and layout stay. A first seal keeps the version; a
/// pack whose content changed since its seal goes up a patch number, or to `version` when given.
///
/// # Errors
/// `pack.ron` does not read or cannot be written, or its version is not `x.y.z` when it must
/// go up.
pub fn seal(root: &Path, version: Option<&str>) -> Result<Sealed, DataError> {
    let file = root.join("pack.ron");
    let now = status(root)?;
    let target = match (version, now.recorded) {
        (Some(v), _) => v.to_owned(),
        (None, Some(recorded)) if recorded != now.hash => {
            next_patch(&now.version).ok_or_else(|| {
                DataError::new(&file, format!("version '{}' is not x.y.z", now.version))
            })?
        }
        (None, _) => now.version.clone(),
    };
    if target == now.version && now.recorded == Some(now.hash) {
        return Ok(Sealed {
            version: target,
            hash: now.hash,
            changed: false,
        });
    }
    let text = read_text(&file, &file)?;
    let text = with_field(&text, "version", &format!("{target:?}"))
        .ok_or_else(|| DataError::new(&file, "no `version:` line to seal"))?;
    write_atomic(&file, &text)?;
    // The version is content: hash again once it is written.
    let hash = status(root)?.hash;
    let text = read_text(&file, &file)?;
    let sealed = match with_field(&text, "hash", &hash.to_string()) {
        Some(sealed) => sealed,
        None => after_version(&text, hash)
            .ok_or_else(|| DataError::new(&file, "no `version:` line to seal"))?,
    };
    write_atomic(&file, &sealed)?;
    Ok(Sealed {
        version: target,
        hash,
        changed: true,
    })
}

/// `x.y.z` with `z` one higher.
fn next_patch(version: &str) -> Option<String> {
    let mut parts = version.split('.');
    let (major, minor, patch) = (parts.next()?, parts.next()?, parts.next()?);
    if parts.next().is_some() || [major, minor].iter().any(|p| p.parse::<u64>().is_err()) {
        return None;
    }
    Some(format!(
        "{major}.{minor}.{}",
        patch.parse::<u64>().ok()?.checked_add(1)?
    ))
}

/// Where the value of `field` lies in `text`: the bytes after `field:` up to the next `,`, `)`
/// or line end outside a string, skipping comments.
fn value_span(text: &str, field: &str) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let (mut at, mut in_string, mut line_start) = (0, false, true);
    while at < bytes.len() {
        let rest = &text[at..];
        match bytes[at] {
            b'"' if !in_string => in_string = true,
            b'"' if in_string && bytes[at - 1] != b'\\' => in_string = false,
            b'/' if !in_string && rest.starts_with("//") => {
                at += rest.find('\n').unwrap_or(rest.len());
                continue;
            }
            _ if in_string => {}
            _ => {
                let after = at.checked_sub(1).map(|p| bytes[p]);
                let boundary =
                    line_start || matches!(after, Some(b'(' | b',' | b' ' | b'\t' | b'\n'));
                if boundary && rest.starts_with(field) {
                    let tail = rest[field.len()..].trim_start();
                    if let Some(value) = tail.strip_prefix(':') {
                        let start = text.len() - value.trim_start().len();
                        let end = end_of_value(text, start);
                        return Some((start, end));
                    }
                }
            }
        }
        line_start = bytes[at] == b'\n';
        at += 1;
    }
    None
}

/// The end of a value starting at `start`: the next `,`, `)` or line end outside a string.
fn end_of_value(text: &str, start: usize) -> usize {
    let mut in_string = false;
    for (offset, b) in text.as_bytes()[start..].iter().enumerate() {
        match b {
            b'"' => in_string = !in_string,
            b',' | b')' | b'\n' if !in_string => return start + offset,
            _ => {}
        }
    }
    text.len()
}

/// `text` with `field`'s value replaced by `value`; `None` when the field is not there.
fn with_field(text: &str, field: &str, value: &str) -> Option<String> {
    let (start, end) = value_span(text, field)?;
    Some(format!("{}{value}{}", &text[..start], &text[end..]))
}

/// `text` with `hash: <hash>` added after the `version` field: on a line of its own when the
/// version has one, else on the same line.
fn after_version(text: &str, hash: u64) -> Option<String> {
    let (start, end) = value_span(text, "version")?;
    let line = text[..start].rfind('\n').map_or(0, |n| n + 1);
    let indent: String = text[line..]
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let own_line = text[end..].starts_with(",\n") || text[end..].starts_with(",\r\n");
    Some(if own_line {
        let comma = end + 1;
        format!(
            "{}\n{indent}hash: {hash},{}",
            &text[..comma],
            &text[comma..]
        )
    } else {
        format!("{}, hash: {hash}{}", &text[..end], &text[end..])
    })
}
