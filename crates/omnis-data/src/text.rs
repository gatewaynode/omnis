//! `text/<lang>/*.ron`: localized strings by key. The simulation only ever emits keys.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One text file: keys to strings with `{arg}` placeholders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextFile {
    /// Schema version of this file.
    pub schema: u32,
    /// Key (`pack:text:name`) to string.
    pub entries: BTreeMap<String, String>,
}

/// Fill each `{name}` in `text` with its value from `args`; a placeholder with no argument, and
/// any brace that does not close, stays as written. One pass: a value is never filled again.
#[must_use]
pub fn fill(text: &str, args: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let value = after.find('}').and_then(|close| {
            let name = &after[..close];
            args.iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| (*v, close))
        });
        match value {
            Some((v, close)) => {
                out.push_str(v);
                rest = &after[close + 1..];
            }
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::fill;

    #[test]
    fn placeholders_are_filled_once_and_unknown_ones_stay() {
        assert_eq!(
            fill(
                "Rats, {ago} ago, in {where}.",
                &[("ago", "3 days"), ("where", "{ago}")]
            ),
            "Rats, 3 days ago, in {ago}."
        );
        assert_eq!(fill("{who} and {", &[]), "{who} and {");
        assert_eq!(fill("no braces", &[("x", "y")]), "no braces");
        assert_eq!(fill("{a}{a}", &[("a", "é")]), "éé");
    }
}
