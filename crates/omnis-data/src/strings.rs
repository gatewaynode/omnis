//! Every string a value holds, whatever its shape, read from the value's compact RON: the words
//! between its quotes. The catalogue finds a definition's references and text keys among them
//! ([`crate::catalog::links`]), so a field or a kind the packs gain later is covered with no code
//! of its own. Field and variant names are bare words in RON and are never among them.
//!
//! Read from the text and not through a `serde::Serializer` of its own: that trait's signatures
//! name the float types, which no simulation crate writes (ARCHITECTURE §11).

use crate::ron_io;
use serde::Serialize;

/// Every string value and string map key in `value`, in the order it writes them, each as RON
/// escapes it (an id or a text key has nothing to escape). Empty when the value does not write.
#[must_use]
pub fn strings<T: Serialize>(value: &T) -> Vec<String> {
    ron_io::to_compact(value)
        .map(|text| quoted(&text).into_iter().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// The contents of every string literal in a RON `text`, escapes left as written. A character
/// literal is stepped over, so a quote written as a character opens no string.
#[must_use]
pub fn quoted(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut chars = text.char_indices();
    while let Some((at, c)) = chars.next() {
        match c {
            '"' => {
                let mut end = text.len();
                while let Some((i, c)) = chars.next() {
                    match c {
                        '\\' => {
                            chars.next();
                        }
                        '"' => {
                            end = i;
                            break;
                        }
                        _ => {}
                    }
                }
                out.push(&text[at + 1..end]);
            }
            '\'' => {
                // One character or one escape, then the closing quote.
                let mut first = true;
                while let Some((_, c)) = chars.next() {
                    match c {
                        '\\' => {
                            chars.next();
                        }
                        '\'' if !first => break,
                        _ => {}
                    }
                    first = false;
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[derive(Serialize)]
    enum Shape {
        Plain,
        Holding { id: String },
    }

    #[derive(Serialize)]
    struct Sample {
        id: String,
        glyphs: Vec<char>,
        said: String,
        pairs: Vec<(String, u8)>,
        table: BTreeMap<String, Option<String>>,
        shapes: Vec<Shape>,
    }

    #[test]
    fn every_string_is_found_and_no_name_of_the_format() {
        let sample = Sample {
            id: "a:item:one".into(),
            glyphs: vec!['"', '\'', '\\', 'x'],
            said: "she said \"a:item:two\" and left".into(),
            pairs: vec![("a:item:three".into(), 2)],
            table: BTreeMap::from([("a:text:key".into(), Some("a:item:four".into()))]),
            shapes: vec![
                Shape::Plain,
                Shape::Holding {
                    id: "a:item:five".into(),
                },
            ],
        };
        let found = strings(&sample);
        assert_eq!(
            found,
            [
                "a:item:one",
                "she said \\\"a:item:two\\\" and left",
                "a:item:three",
                "a:text:key",
                "a:item:four",
                "a:item:five",
            ]
        );
    }

    #[test]
    fn an_unclosed_string_runs_to_the_end() {
        assert_eq!(quoted("(a: \"open"), ["open"]);
        assert_eq!(quoted("('\"', \"x\")"), ["x"]);
    }
}
