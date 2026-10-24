//! The SVG subset the Lucide set uses: one `<svg>` root with fixed attributes and flat,
//! self-closing children carrying only attributes. Anything else is refused, so the set is checked
//! whole by test.

use crate::Error;

/// A child element of the root: its name and its attributes in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// `path`, `circle`, `rect`, `line`, `ellipse`, `polyline` or `polygon`.
    pub name: String,
    /// Its attributes.
    pub attrs: Vec<(String, String)>,
}

impl Element {
    /// The attribute `name`, if given.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// The attribute `name` as a number, `default` when absent.
    ///
    /// # Errors
    /// The attribute is not a number.
    pub fn number(&self, name: &str, default: f32) -> Result<f32, Error> {
        match self.attr(name) {
            None => Ok(default),
            Some(v) => v.trim().parse::<f32>().map_err(|_| {
                Error::Malformed(format!("<{}> {name}=\"{v}\" is not a number", self.name))
            }),
        }
    }
}

/// The root's attributes the set uses; their values are not read (every icon is the same box).
const ROOT_ATTRS: [&str; 10] = [
    "xmlns",
    "width",
    "height",
    "viewBox",
    "fill",
    "stroke",
    "stroke-width",
    "stroke-linecap",
    "stroke-linejoin",
    "class",
];

/// The child elements and the attributes each may carry. A child's `fill` is the set's dot: a
/// filled circle no wider than the stroke's half width, whose outline under the stroke is the same
/// disc, so the fill is read and not kept (the set's tests pin that nothing larger is filled).
const CHILDREN: [(&str, &[&str]); 7] = [
    ("path", &["d", "fill"]),
    ("circle", &["cx", "cy", "r", "fill"]),
    ("rect", &["x", "y", "width", "height", "rx", "ry"]),
    ("line", &["x1", "y1", "x2", "y2"]),
    ("ellipse", &["cx", "cy", "rx", "ry"]),
    ("polyline", &["points"]),
    ("polygon", &["points"]),
];

/// Read `text`: the root checked, its children returned in order.
///
/// # Errors
/// An unknown element or attribute, or text that is not this shape.
pub fn read(text: &str) -> Result<Vec<Element>, Error> {
    let mut reader = Reader {
        s: text.as_bytes(),
        at: 0,
    };
    reader.skip_space_and_comments();
    let (root, attrs, closed) = reader.tag()?;
    if root != "svg" {
        return Err(Error::UnknownElement(root));
    }
    for (k, _) in &attrs {
        if !ROOT_ATTRS.contains(&k.as_str()) {
            return Err(Error::UnknownAttribute {
                element: root,
                attribute: k.clone(),
            });
        }
    }
    let mut out = Vec::new();
    if closed {
        return Ok(out);
    }
    loop {
        reader.skip_space_and_comments();
        if reader.starts_with(b"</") {
            reader.at += 2;
            let name = reader.name()?;
            reader.skip_space();
            reader.expect(b'>')?;
            if name != "svg" {
                return Err(Error::Malformed(format!("closing </{name}> for <svg>")));
            }
            reader.skip_space_and_comments();
            if reader.at != reader.s.len() {
                return Err(Error::Malformed("text after </svg>".to_owned()));
            }
            return Ok(out);
        }
        let (name, attrs, closed) = reader.tag()?;
        let Some((_, allowed)) = CHILDREN.iter().find(|(n, _)| *n == name) else {
            return Err(Error::UnknownElement(name));
        };
        if !closed {
            return Err(Error::Malformed(format!("<{name}> is not self-closing")));
        }
        for (k, _) in &attrs {
            if !allowed.contains(&k.as_str()) {
                return Err(Error::UnknownAttribute {
                    element: name,
                    attribute: k.clone(),
                });
            }
        }
        out.push(Element { name, attrs });
    }
}

/// A tag as read: its name, its attributes, whether it closed itself.
type Tag = (String, Vec<(String, String)>, bool);

struct Reader<'a> {
    s: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn starts_with(&self, what: &[u8]) -> bool {
        self.s[self.at..].starts_with(what)
    }

    fn skip_space(&mut self) {
        while self.at < self.s.len() && self.s[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
    }

    fn skip_space_and_comments(&mut self) {
        loop {
            self.skip_space();
            if self.starts_with(b"<!--") {
                match self.s[self.at..].windows(3).position(|w| w == b"-->") {
                    Some(end) => self.at += end + 3,
                    None => self.at = self.s.len(),
                }
            } else if self.starts_with(b"<?") {
                match self.s[self.at..].windows(2).position(|w| w == b"?>") {
                    Some(end) => self.at += end + 2,
                    None => self.at = self.s.len(),
                }
            } else {
                return;
            }
        }
    }

    fn expect(&mut self, b: u8) -> Result<(), Error> {
        if self.s.get(self.at) == Some(&b) {
            self.at += 1;
            Ok(())
        } else {
            Err(Error::Malformed(format!(
                "expected '{}' at byte {}",
                b as char, self.at
            )))
        }
    }

    /// An element or attribute name: letters, digits, `-`, `_`, `:`.
    fn name(&mut self) -> Result<String, Error> {
        let start = self.at;
        while self.at < self.s.len()
            && (self.s[self.at].is_ascii_alphanumeric() || b"-_:".contains(&self.s[self.at]))
        {
            self.at += 1;
        }
        if start == self.at {
            return Err(Error::Malformed(format!("a name at byte {start}")));
        }
        Ok(String::from_utf8_lossy(&self.s[start..self.at]).into_owned())
    }

    /// `<name attr="value" ...>` or `<name ... />`: the name, the attributes, whether it closed
    /// itself.
    fn tag(&mut self) -> Result<Tag, Error> {
        self.expect(b'<')?;
        let name = self.name()?;
        let mut attrs = Vec::new();
        loop {
            self.skip_space();
            if self.starts_with(b"/>") {
                self.at += 2;
                return Ok((name, attrs, true));
            }
            if self.starts_with(b">") {
                self.at += 1;
                return Ok((name, attrs, false));
            }
            let key = self.name()?;
            self.skip_space();
            self.expect(b'=')?;
            self.skip_space();
            let quote = *self
                .s
                .get(self.at)
                .filter(|q| **q == b'"' || **q == b'\'')
                .ok_or_else(|| Error::Malformed(format!("a quoted value for {key}")))?;
            self.at += 1;
            let start = self.at;
            while self.at < self.s.len() && self.s[self.at] != quote {
                self.at += 1;
            }
            if self.at >= self.s.len() {
                return Err(Error::Malformed(format!("an unclosed value for {key}")));
            }
            let value = String::from_utf8_lossy(&self.s[start..self.at]).into_owned();
            self.at += 1;
            attrs.push((key, value));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_root_with_flat_children_reads_and_anything_else_is_refused() {
        let text = r#"<svg xmlns="x" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
  <!-- a note -->
  <path d="M1 1h2" />
  <circle cx='12' cy="12" r="3"/>
</svg>"#;
        let elements = read(text).unwrap();
        assert_eq!(elements.len(), 2);
        assert_eq!(elements[0].attr("d"), Some("M1 1h2"));
        assert_eq!(elements[1].number("cx", 0.0).unwrap(), 12.0);
        assert_eq!(
            read("<svg><g /></svg>"),
            Err(Error::UnknownElement("g".into()))
        );
        assert_eq!(
            read("<svg><path d=\"M0 0\" stroke=\"red\" /></svg>"),
            Err(Error::UnknownAttribute {
                element: "path".into(),
                attribute: "stroke".into()
            })
        );
        assert!(matches!(
            read("<svg><path d=\"M0 0\"></path></svg>"),
            Err(Error::Malformed(_))
        ));
        assert!(matches!(read("<svg>text</svg>"), Err(Error::Malformed(_))));
    }
}
