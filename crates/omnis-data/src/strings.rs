//! Every string a value holds, whatever its shape, collected by a `serde::Serializer` that keeps
//! the strings and drops the rest. The catalogue finds a definition's references and text keys
//! among them ([`crate::catalog::links`]), so a field or a kind the packs gain later is covered
//! with no code of its own. Field and variant names are not values and are never among them.

use core::fmt::{self, Display};
use serde::Serialize;
use serde::ser::{
    self, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant,
};

/// Every string value and string map key in `value`, in the order it writes them. A value whose
/// own `Serialize` fails gives the strings written before it did.
#[must_use]
pub fn strings<T: Serialize + ?Sized>(value: &T) -> Vec<String> {
    let mut found = Vec::new();
    // The collector itself never fails; an error is the value's own, and ends the walk.
    let _ = value.serialize(&mut Collector { found: &mut found });
    found
}

// serde's `Serializer` trait has float methods; ours drop them, and no definition carries a
// float. The aliases keep the banned spelling on two allowed lines (ARCHITECTURE §11).
type Float32 = f32; // lint-sim: allow
type Float64 = f64; // lint-sim: allow

/// The error a value's own `Serialize` may raise; the collector raises none.
#[derive(Debug)]
struct Stop;

impl Display for Stop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("stop")
    }
}

impl core::error::Error for Stop {}

impl ser::Error for Stop {
    fn custom<T: Display>(_msg: T) -> Self {
        Stop
    }
}

struct Collector<'a> {
    found: &'a mut Vec<String>,
}

impl Collector<'_> {
    fn walk<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        value.serialize(&mut Collector { found: self.found })
    }
}

macro_rules! dropped {
    ($($method:ident: $ty:ty),* $(,)?) => {$(
        fn $method(self, _: $ty) -> Result<(), Stop> {
            Ok(())
        }
    )*};
}

impl<'a, 'b> ser::Serializer for &'a mut Collector<'b> {
    type Ok = ();
    type Error = Stop;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    dropped! {
        serialize_bool: bool, serialize_i8: i8, serialize_i16: i16, serialize_i32: i32,
        serialize_i64: i64, serialize_u8: u8, serialize_u16: u16, serialize_u32: u32,
        serialize_u64: u64, serialize_f32: Float32, serialize_f64: Float64,
        serialize_char: char, serialize_bytes: &[u8],
    }

    fn serialize_str(self, v: &str) -> Result<(), Stop> {
        self.found.push(v.to_owned());
        Ok(())
    }

    fn serialize_none(self) -> Result<(), Stop> {
        Ok(())
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<(), Stop> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<(), Stop> {
        Ok(())
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Stop> {
        Ok(())
    }

    fn serialize_unit_variant(self, _: &'static str, _: u32, _: &'static str) -> Result<(), Stop> {
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
        _: &'static str,
        value: &T,
    ) -> Result<(), Stop> {
        value.serialize(self)
    }

    fn serialize_seq(self, _: Option<usize>) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_tuple(self, _: usize) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_map(self, _: Option<usize>) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, Stop> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, Stop> {
        Ok(self)
    }
}

/// Every compound shape walks each value it holds; a struct's field names are not values.
macro_rules! compound {
    ($($shape:ident: $method:ident($($key:ty)?)),* $(,)?) => {$(
        impl $shape for &mut Collector<'_> {
            type Ok = ();
            type Error = Stop;
            fn $method<T: Serialize + ?Sized>(
                &mut self,
                $(_: $key,)?
                value: &T,
            ) -> Result<(), Stop> {
                self.walk(value)
            }
            fn end(self) -> Result<(), Stop> {
                Ok(())
            }
        }
    )*};
}

compound! {
    SerializeSeq: serialize_element(),
    SerializeTuple: serialize_element(),
    SerializeTupleStruct: serialize_field(),
    SerializeTupleVariant: serialize_field(),
    SerializeStruct: serialize_field(&'static str),
    SerializeStructVariant: serialize_field(&'static str),
}

impl SerializeMap for &mut Collector<'_> {
    type Ok = ();
    type Error = Stop;
    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Stop> {
        self.walk(key)
    }
    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Stop> {
        self.walk(value)
    }
    fn end(self) -> Result<(), Stop> {
        Ok(())
    }
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
                "she said \"a:item:two\" and left",
                "a:item:three",
                "a:text:key",
                "a:item:four",
                "a:item:five",
            ]
        );
    }
}
