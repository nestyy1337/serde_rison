use std::num::FpCategory;

use crate::error::{Error, ErrorCode};
use serde::ser::Impossible;
use serde::{Serialize, ser};

type Result<T> = std::result::Result<T, Error>;

pub struct Serializer {
    output: String,
}

impl Default for Serializer {
    fn default() -> Self {
        Self::new()
    }
}

impl Serializer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            output: String::new(),
        }
    }
}

pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut serializer = Serializer {
        output: String::new(),
    };
    value.serialize(&mut serializer)?;
    Ok(serializer.output)
}

impl ser::Serializer for &mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.output += if v { "!t" } else { "!f" };
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        match v.classify() {
            FpCategory::Nan | FpCategory::Infinite => {
                self.output += "!n";
            }
            _ => {
                let mut buf = zmij::Buffer::new();
                let string = buf.format_finite(v);
                let pos = string.find('+');
                if let Some(pos) = pos {
                    let (left, right) = string.split_at(pos);
                    self.output += left;
                    self.output += right;
                } else {
                    self.output += string;
                }
            }
        }
        Ok(())
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        match v.classify() {
            FpCategory::Nan | FpCategory::Infinite => {
                self.output += "!n";
            }
            _ => {
                let mut buf = zmij::Buffer::new();
                let string = buf.format_finite(v);
                match string.split_once('+') {
                    Some((left, right)) => {
                        self.output += left;
                        self.output += right;
                    }
                    None => self.output += string,
                }
            }
        }
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        if v.is_empty()
            || v.starts_with(|c: char| c.is_ascii_digit() || c == '-')
            || !v
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '/' | '~'))
        {
            self.output += "'";
            for ch in v.chars() {
                match ch {
                    '!' => self.output += "!!",
                    '\'' => self.output += "!'",
                    _ => self.output.push(ch),
                }
            }
            self.output += "'";
        } else {
            self.output += v;
        }
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        use serde::ser::SerializeSeq;
        let mut seq = self.serialize_seq(Some(v.len()))?;
        for byte in v {
            seq.serialize_element(byte)?;
        }
        seq.end()
    }

    fn serialize_none(self) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        self.output += "!n";
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += "(";
        variant.serialize(&mut *self)?;
        self.output += ":";
        value.serialize(&mut *self)?;
        self.output += ")";
        Ok(())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.output += "!(";
        Ok(self)
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.output += "(";
        variant.serialize(&mut *self)?;
        self.output += ":!(";
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.output += "(";
        Ok(self)
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.output += "(";
        variant.serialize(&mut *self)?;
        self.output += ":(";
        Ok(self)
    }
}

impl ser::SerializeSeq for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if !self.output.ends_with("!(") {
            self.output += ",";
        }
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += ")";
        Ok(())
    }
}

impl ser::SerializeTuple for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if !self.output.ends_with("!(") {
            self.output += ",";
        }
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += ")";
        Ok(())
    }
}

impl ser::SerializeTupleStruct for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if !self.output.ends_with("!(") {
            self.output += ",";
        }
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += ")";
        Ok(())
    }
}

impl ser::SerializeTupleVariant for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if !self.output.ends_with("!(") {
            self.output += ",";
        }
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += "))";
        Ok(())
    }
}

impl ser::SerializeMap for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if !self.output.ends_with('(') {
            self.output += ",";
        }
        key.serialize(MapKeySerializer { ser: self })
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.output += ":";
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += ")";
        Ok(())
    }
}

impl ser::SerializeStruct for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if !self.output.ends_with('(') {
            self.output += ",";
        }
        key.serialize(&mut **self)?;
        self.output += ":";
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += ")";
        Ok(())
    }
}

impl ser::SerializeStructVariant for &mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        if !self.output.ends_with('(') {
            self.output += ",";
        }
        key.serialize(&mut **self)?;
        self.output += ":";
        value.serialize(&mut **self)
    }

    fn end(self) -> Result<()> {
        self.output += "))";
        Ok(())
    }
}

/// Accepts only keys that read back as the same key: strings, chars,
/// integers, bools and unit variants.
struct MapKeySerializer<'a> {
    ser: &'a mut Serializer,
}

fn key_must_be_a_string() -> Error {
    Error::data(ErrorCode::KeyMustBeAString)
}

macro_rules! serialize_integer_key {
    ($($method:ident($ty:ty),)*) => {
        $(
            fn $method(self, v: $ty) -> Result<()> {
                self.ser.output += &v.to_string();
                Ok(())
            }
        )*
    };
}

impl ser::Serializer for MapKeySerializer<'_> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_str(self, v: &str) -> Result<()> {
        ser::Serializer::serialize_str(self.ser, v)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(v.encode_utf8(&mut [0; 4]))
    }

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.serialize_str(if v { "true" } else { "false" })
    }

    serialize_integer_key! {
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_i128(i128),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_u128(u128),
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(key_must_be_a_string())
    }

    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(key_must_be_a_string())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(key_must_be_a_string())
    }

    fn serialize_none(self) -> Result<()> {
        Err(key_must_be_a_string())
    }

    fn serialize_some<T>(self, _value: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(key_must_be_a_string())
    }

    fn serialize_unit(self) -> Result<()> {
        Err(key_must_be_a_string())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(key_must_be_a_string())
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Err(key_must_be_a_string())
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(key_must_be_a_string())
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(key_must_be_a_string())
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(key_must_be_a_string())
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(key_must_be_a_string())
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(key_must_be_a_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_struct() {
        #[derive(Serialize)]
        struct Test {
            int: u32,
            seq: Vec<&'static str>,
            text: &'static str,
        }

        let test = Test {
            int: 1,
            seq: vec!["a", "b"],
            text: "test with space",
        };
        let expected = r"(int:1,seq:!(a,b),text:'test with space')";
        assert_eq!(to_string(&test).unwrap(), expected);
    }

    #[test]
    fn test_enum() {
        #[derive(Serialize)]
        enum E {
            Unit,
            Newtype(u32),
            Tuple(u32, u32),
            Struct { a: u32 },
        }

        let u = E::Unit;
        let expected = r"Unit";
        assert_eq!(to_string(&u).unwrap(), expected);

        let n = E::Newtype(1);
        let expected = r"(Newtype:1)";
        assert_eq!(to_string(&n).unwrap(), expected);

        let t = E::Tuple(1, 2);
        let expected = r"(Tuple:!(1,2))";
        assert_eq!(to_string(&t).unwrap(), expected);

        let s = E::Struct { a: 1 };
        let expected = r"(Struct:(a:1))";
        assert_eq!(to_string(&s).unwrap(), expected);
    }

    #[test]
    fn test_escape_bang() {
        assert_eq!(to_string(&"bang!").unwrap(), "'bang!!'");
    }

    #[test]
    fn test_escape_quote() {
        assert_eq!(to_string(&"it's").unwrap(), "'it!'s'");
    }

    #[test]
    fn test_escape_both() {
        assert_eq!(to_string(&"it's a bang!").unwrap(), "'it!'s a bang!!'");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(to_string(&"").unwrap(), "''");
    }

    #[test]
    fn test_unquoted_string() {
        assert_eq!(to_string(&"hello").unwrap(), "hello");
    }

    #[test]
    fn test_quote_string_with_asterisk() {
        assert_eq!(to_string(&".ds-logs*").unwrap(), "'.ds-logs*'");
    }

    #[test]
    fn test_numeric_looking_string() {
        assert_eq!(to_string(&"123abc").unwrap(), "'123abc'");
    }

    #[test]
    fn test_none() {
        assert_eq!(to_string(&None::<u32>).unwrap(), "!n");
    }

    #[test]
    fn test_some() {
        assert_eq!(to_string(&Some(42u32)).unwrap(), "42");
    }

    #[test]
    fn test_bool() {
        assert_eq!(to_string(&true).unwrap(), "!t");
        assert_eq!(to_string(&false).unwrap(), "!f");
    }

    #[test]
    fn test_float() {
        assert_eq!(to_string(&2.5f64).unwrap(), "2.5");
        assert_eq!(to_string(&-0.5f64).unwrap(), "-0.5");
    }

    #[test]
    fn test_null_unit() {
        assert_eq!(to_string(&()).unwrap(), "!n");
    }

    #[test]
    fn test_nested_struct() {
        #[derive(Serialize)]
        struct Inner {
            x: u32,
        }
        #[derive(Serialize)]
        struct Outer {
            a: Inner,
            b: Vec<u32>,
        }
        let val = Outer {
            a: Inner { x: 1 },
            b: vec![2, 3],
        };
        assert_eq!(to_string(&val).unwrap(), "(a:(x:1),b:!(2,3))");
    }

    #[test]
    fn test_map() {
        let mut map = std::collections::BTreeMap::new();
        map.insert("a", 1);
        map.insert("b", 2);
        assert_eq!(to_string(&map).unwrap(), "(a:1,b:2)");
    }

    #[test]
    fn test_empty_map() {
        let map: std::collections::BTreeMap<String, u32> = std::collections::BTreeMap::new();
        assert_eq!(to_string(&map).unwrap(), "()");
    }

    #[test]
    fn test_empty_vec() {
        let v: Vec<u32> = vec![];
        assert_eq!(to_string(&v).unwrap(), "!()");
    }

    #[test]
    fn test_map_keys() {
        use std::collections::BTreeMap;

        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        enum K {
            Alpha,
        }

        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        struct Id(u32);

        let ints = BTreeMap::from([(-1, 0), (2, 0)]);
        assert_eq!(to_string(&ints).unwrap(), "(-1:0,2:0)");
        let bools = BTreeMap::from([(false, 0), (true, 1)]);
        assert_eq!(to_string(&bools).unwrap(), "(false:0,true:1)");
        let chars = BTreeMap::from([('a', 0), ('!', 1)]);
        assert_eq!(to_string(&chars).unwrap(), "('!!':1,a:0)");
        let strings = BTreeMap::from([("a b", 0), ("12", 1)]);
        assert_eq!(to_string(&strings).unwrap(), "('12':1,'a b':0)");
        let variants = BTreeMap::from([(K::Alpha, 0)]);
        assert_eq!(to_string(&variants).unwrap(), "(Alpha:0)");
        let newtypes = BTreeMap::from([(Id(7), 0)]);
        assert_eq!(to_string(&newtypes).unwrap(), "(7:0)");

        let err = to_string(&BTreeMap::from([((1, 2), 3)])).unwrap_err();
        assert_eq!(
            err.to_string(),
            "map key must be a string, integer, char or bool"
        );
        assert!(to_string(&BTreeMap::from([(None::<u32>, 1)])).is_err());
        assert!(to_string(&BTreeMap::from([(Some(1), 1)])).is_err());
        assert!(to_string(&BTreeMap::from([((), 1)])).is_err());
    }

    #[test]
    fn test_map_keys_roundtrip() {
        use std::collections::BTreeMap;

        let ints = BTreeMap::from([(-1_i64, 0), (2, 0)]);
        let back: BTreeMap<i64, i32> = crate::from_str(&to_string(&ints).unwrap()).unwrap();
        assert_eq!(back, ints);

        let bools = BTreeMap::from([(false, 0), (true, 1)]);
        let back: BTreeMap<bool, i32> = crate::from_str(&to_string(&bools).unwrap()).unwrap();
        assert_eq!(back, bools);

        let value: crate::Value = crate::from_str(&to_string(&ints).unwrap()).unwrap();
        let back: BTreeMap<i64, i32> = crate::from_value(value).unwrap();
        assert_eq!(back, ints);
    }

    #[test]
    fn f64_ser() {
        let val = 1e20_f64;
        let string = to_string(&val).unwrap();
        assert_eq!(string, "1e20");
    }
}
