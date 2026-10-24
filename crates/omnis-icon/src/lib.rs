//! Omnis icon: the Lucide icon set (ISC, `assets/svg-icons/`) compiled into the binary and read
//! back as strokes, with no dependencies (owner, 2026-10-24: "a self contained library").
//!
//! Every Lucide icon is a 24 by 24 drawing of strokes two units wide with round caps and joins,
//! made of paths, circles, rectangles, lines, ellipses and polygons. This crate reads exactly that
//! subset of SVG (`svg`), turns path data into polylines (`path`, arcs included), the other shapes
//! likewise (`shape`), and offers each icon as polylines in its 24-unit space (`Icon`) for a
//! client to draw at any size: as pixels, or as lines in a scene. The catalogue (`catalogue`)
//! holds the whole set by name, built in by `build.rs`, and the set's licence text.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod catalogue;
pub mod icon;
pub mod path;
pub mod shape;
pub mod svg;

pub use catalogue::{LICENSE, get, names, text};
pub use icon::{Icon, Stroke, WIDTH};

/// Why an icon's text does not read: the set is read whole by a test, so a Lucide release that
/// adds a feature fails loudly rather than drawing wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// An element this reader does not know.
    UnknownElement(String),
    /// An attribute this reader does not know on an element it does.
    UnknownAttribute {
        /// The element.
        element: String,
        /// The attribute.
        attribute: String,
    },
    /// The text is not the shape this reader expects, with where and why.
    Malformed(String),
    /// A number or a path command that does not read.
    Path(String),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Error::UnknownElement(e) => write!(f, "unknown element <{e}>"),
            Error::UnknownAttribute { element, attribute } => {
                write!(f, "unknown attribute {attribute} on <{element}>")
            }
            Error::Malformed(why) => write!(f, "malformed: {why}"),
            Error::Path(why) => write!(f, "path data: {why}"),
        }
    }
}

impl std::error::Error for Error {}
