//! A serde `Serializer` that walks a value looking for one dotted path and renders the leaf.
//! It is the generic inspector behind MCP `world.query`; it needs no reflection and no extra
//! dependency, and it works on anything that is `Serialize`.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt::Display;
use serde::Serialize;
use serde::ser::{
    self, Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};

/// Look `path` up in `value`.
pub fn find<T: Serialize>(value: &T, path: &str) -> Option<String> {
    let segments: Vec<&str> = if path.is_empty() {
        Vec::new()
    } else {
        path.split('.').collect()
    };
    let mut finder = Finder {
        segments: &segments,
        depth: 0,
        found: None,
    };
    // Every outcome is signalled through `found`; the error only stops the walk early.
    let _ = value.serialize(&mut finder);
    finder.found
}

/// Render a map key the way paths name it: primitives as themselves, unit variants by name,
/// newtype variants as `Name(inner)`, tuples joined by commas, newtype structs as their inner.
pub fn key_string<T: Serialize + ?Sized>(key: &T) -> String {
    let mut out = String::new();
    let _ = key.serialize(&mut KeyString { out: &mut out });
    out
}

// serde's `Serializer` trait has float methods; ours refuse them, and nothing in the world
// carries a float. The aliases keep the banned spelling on two allowed lines.
type Float32 = f32; // lint-sim: allow
type Float64 = f64; // lint-sim: allow

#[derive(Debug)]
struct Stop;

impl Display for Stop {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("stop")
    }
}

impl core::error::Error for Stop {}

impl ser::Error for Stop {
    fn custom<T: Display>(_msg: T) -> Self {
        Stop
    }
}

struct Finder<'a> {
    segments: &'a [&'a str],
    depth: usize,
    found: Option<String>,
}

impl Finder<'_> {
    fn at_leaf(&self) -> bool {
        self.depth == self.segments.len()
    }

    fn want(&self) -> Option<&str> {
        self.segments.get(self.depth).copied()
    }

    fn leaf(&mut self, text: impl Display) -> Result<(), Stop> {
        if self.at_leaf() {
            self.found = Some(text.to_string());
        }
        Err(Stop)
    }

    fn descend<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        let mut inner = Finder {
            segments: self.segments,
            depth: self.depth + 1,
            found: None,
        };
        let _ = value.serialize(&mut inner);
        self.found = inner.found;
        Err(Stop)
    }
}

macro_rules! leaves {
    ($($method:ident: $ty:ty),* $(,)?) => {$(
        fn $method(self, v: $ty) -> Result<(), Stop> {
            self.leaf(v)
        }
    )*};
}

impl<'a, 'b> ser::Serializer for &'a mut Finder<'b> {
    type Ok = ();
    type Error = Stop;
    type SerializeSeq = Seq<'a, 'b>;
    type SerializeTuple = Seq<'a, 'b>;
    type SerializeTupleStruct = Seq<'a, 'b>;
    type SerializeTupleVariant = Seq<'a, 'b>;
    type SerializeMap = Map<'a, 'b>;
    type SerializeStruct = Fields<'a, 'b>;
    type SerializeStructVariant = Fields<'a, 'b>;

    leaves! {
        serialize_bool: bool, serialize_i8: i8, serialize_i16: i16, serialize_i32: i32,
        serialize_i64: i64, serialize_u8: u8, serialize_u16: u16, serialize_u32: u32,
        serialize_u64: u64, serialize_char: char, serialize_str: &str,
    }

    fn serialize_f32(self, _: Float32) -> Result<(), Stop> {
        Err(Stop)
    }
    fn serialize_f64(self, _: Float64) -> Result<(), Stop> {
        Err(Stop)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<(), Stop> {
        self.leaf(format!("<{} bytes>", v.len()))
    }

    fn serialize_none(self) -> Result<(), Stop> {
        self.leaf("None")
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Stop> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), Stop> {
        self.leaf("()")
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<(), Stop> {
        self.leaf(name)
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), Stop> {
        self.leaf(variant)
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        if self.at_leaf() {
            return self.leaf(format!("{variant}({})", key_string(value)));
        }
        if self.want() == Some(variant) {
            return self.descend(value);
        }
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Stop> {
        if self.at_leaf() {
            self.found = Some(len.map_or_else(|| "<seq>".into(), |n| format!("<seq of {n}>")));
            return Err(Stop);
        }
        Ok(Seq {
            finder: self,
            index: 0,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Stop> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Stop> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant, Stop> {
        self.serialize_seq(Some(len))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Stop> {
        if self.at_leaf() {
            self.found = Some(len.map_or_else(|| "<map>".into(), |n| format!("<map of {n}>")));
            return Err(Stop);
        }
        Ok(Map {
            finder: self,
            key_matches: false,
        })
    }

    fn serialize_struct(self, name: &'static str, _: usize) -> Result<Self::SerializeStruct, Stop> {
        if self.at_leaf() {
            self.found = Some(format!("<struct {name}>"));
            return Err(Stop);
        }
        Ok(Fields { finder: self })
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant, Stop> {
        self.serialize_struct(variant, len)
    }
}

struct Seq<'a, 'b> {
    finder: &'a mut Finder<'b>,
    index: usize,
}

impl Seq<'_, '_> {
    fn element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        let index = self.index;
        self.index += 1;
        if self.finder.want().and_then(|w| w.parse::<usize>().ok()) == Some(index) {
            return self.finder.descend(value);
        }
        Ok(())
    }
}

impl SerializeSeq for Seq<'_, '_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        self.element(value)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

impl SerializeTuple for Seq<'_, '_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        self.element(value)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

impl SerializeTupleStruct for Seq<'_, '_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        self.element(value)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

impl SerializeTupleVariant for Seq<'_, '_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        self.element(value)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

struct Map<'a, 'b> {
    finder: &'a mut Finder<'b>,
    key_matches: bool,
}

impl SerializeMap for Map<'_, '_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Stop> {
        self.key_matches = self.finder.want() == Some(key_string(key).as_str());
        Ok(())
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        if self.key_matches {
            return self.finder.descend(value);
        }
        Ok(())
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

struct Fields<'a, 'b> {
    finder: &'a mut Finder<'b>,
}

impl SerializeStruct for Fields<'_, '_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        if self.finder.want() == Some(key) {
            return self.finder.descend(value);
        }
        Ok(())
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

impl SerializeStructVariant for Fields<'_, '_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        SerializeStruct::serialize_field(self, key, value)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
}

/// Renders a key to text; see `key_string`.
struct KeyString<'a> {
    out: &'a mut String,
}

macro_rules! key_leaves {
    ($($method:ident: $ty:ty),* $(,)?) => {$(
        fn $method(self, v: $ty) -> Result<(), Stop> {
            self.out.push_str(&v.to_string());
            Ok(())
        }
    )*};
}

impl<'a> ser::Serializer for &'a mut KeyString<'_> {
    type Ok = ();
    type Error = Stop;
    type SerializeSeq = KeyTuple<'a>;
    type SerializeTuple = KeyTuple<'a>;
    type SerializeTupleStruct = KeyTuple<'a>;
    type SerializeTupleVariant = KeyTuple<'a>;
    type SerializeMap = Impossible<(), Stop>;
    type SerializeStruct = Impossible<(), Stop>;
    type SerializeStructVariant = Impossible<(), Stop>;

    key_leaves! {
        serialize_bool: bool, serialize_i8: i8, serialize_i16: i16, serialize_i32: i32,
        serialize_i64: i64, serialize_u8: u8, serialize_u16: u16, serialize_u32: u32,
        serialize_u64: u64, serialize_char: char, serialize_str: &str,
    }

    fn serialize_f32(self, _: Float32) -> Result<(), Stop> {
        Err(Stop)
    }
    fn serialize_f64(self, _: Float64) -> Result<(), Stop> {
        Err(Stop)
    }

    fn serialize_bytes(self, _: &[u8]) -> Result<(), Stop> {
        Err(Stop)
    }
    fn serialize_none(self) -> Result<(), Stop> {
        self.out.push_str("None");
        Ok(())
    }
    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Stop> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Stop> {
        self.out.push_str("()");
        Ok(())
    }
    fn serialize_unit_struct(self, name: &'static str) -> Result<(), Stop> {
        self.out.push_str(name);
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<(), Stop> {
        self.out.push_str(variant);
        Ok(())
    }
    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        self.out.push_str(variant);
        self.out.push('(');
        value.serialize(&mut *self)?;
        self.out.push(')');
        Ok(())
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Stop> {
        Ok(KeyTuple {
            out: self.out,
            first: true,
            close: false,
        })
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Stop> {
        Ok(KeyTuple {
            out: self.out,
            first: true,
            close: false,
        })
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Stop> {
        Ok(KeyTuple {
            out: self.out,
            first: true,
            close: false,
        })
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Stop> {
        self.out.push_str(variant);
        self.out.push('(');
        Ok(KeyTuple {
            out: self.out,
            first: true,
            close: true,
        })
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Stop> {
        Err(Stop)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self::SerializeStruct, Stop> {
        Err(Stop)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Stop> {
        Err(Stop)
    }
}

struct KeyTuple<'a> {
    out: &'a mut String,
    first: bool,
    close: bool,
}

impl KeyTuple<'_> {
    fn element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        if !self.first {
            self.out.push(',');
        }
        self.first = false;
        value.serialize(&mut KeyString { out: self.out })
    }

    fn finish(self) -> Result<(), Stop> {
        if self.close {
            self.out.push(')');
        }
        Ok(())
    }
}

impl SerializeSeq for KeyTuple<'_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        self.element(value)
    }
    fn end(self) -> Result<(), Stop> {
        self.finish()
    }
}

impl SerializeTuple for KeyTuple<'_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        self.element(value)
    }
    fn end(self) -> Result<(), Stop> {
        self.finish()
    }
}

impl SerializeTupleStruct for KeyTuple<'_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        self.element(value)
    }
    fn end(self) -> Result<(), Stop> {
        self.finish()
    }
}

impl SerializeTupleVariant for KeyTuple<'_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        self.element(value)
    }
    fn end(self) -> Result<(), Stop> {
        self.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeMap;
    use alloc::vec;

    #[derive(Serialize)]
    enum Holder {
        Party(u32),
        Region(u32),
    }

    #[derive(Serialize)]
    struct Inner {
        elapsed: i64,
        era: u32,
    }

    #[derive(Serialize)]
    enum Facing {
        North,
    }

    #[derive(Serialize)]
    struct Outer {
        x: u16,
        facing: Facing,
        clocks: BTreeMap<String, Inner>,
        pairs: BTreeMap<(u16, u16), u8>,
        list: Vec<Holder>,
        maybe: Option<u8>,
    }

    fn sample() -> Outer {
        let mut clocks = BTreeMap::new();
        clocks.insert(
            key_string(&Holder::Party(0)),
            Inner {
                elapsed: 42,
                era: 0,
            },
        );
        let mut pairs = BTreeMap::new();
        pairs.insert((4, 7), 5);
        Outer {
            x: 16,
            facing: Facing::North,
            clocks,
            pairs,
            list: vec![Holder::Region(3), Holder::Party(1)],
            maybe: None,
        }
    }

    #[test]
    fn finds_fields_keys_indices_and_variants() {
        let v = sample();
        assert_eq!(find(&v, "x").as_deref(), Some("16"));
        assert_eq!(find(&v, "facing").as_deref(), Some("North"));
        assert_eq!(find(&v, "clocks.Party(0).elapsed").as_deref(), Some("42"));
        assert_eq!(
            find(&v, "clocks.Party(0)").as_deref(),
            Some("<struct Inner>")
        );
        assert_eq!(find(&v, "pairs.4,7").as_deref(), Some("5"));
        assert_eq!(find(&v, "list.1").as_deref(), Some("Party(1)"));
        assert_eq!(find(&v, "list.1.Party").as_deref(), Some("1"));
        assert_eq!(find(&v, "list").as_deref(), Some("<seq of 2>"));
        assert_eq!(find(&v, "maybe").as_deref(), Some("None"));
        assert_eq!(find(&v, "").as_deref(), Some("<struct Outer>"));
        assert_eq!(find(&v, "nope"), None);
        assert_eq!(find(&v, "x.deeper"), None);
        assert_eq!(find(&v, "list.9"), None);
        assert_eq!(key_string(&Holder::Region(3)), "Region(3)");
        assert_eq!(key_string(&(1u8, 2u8)), "1,2");
    }
}
