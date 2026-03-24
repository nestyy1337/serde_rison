use crate::error::Error;
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

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.output += &v.to_string();
        Ok(())
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        if v.is_empty()
            || v.starts_with(|c: char| c.is_ascii_digit() || c == '-')
            || v.contains(&['(', ')', '!', ':', ',', '\'', ' '][..])
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
        key.serialize(&mut **self)
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
        let expected = r#"(int:1,seq:!(a,b),text:'test with space')"#;
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
        let expected = r#"Unit"#;
        assert_eq!(to_string(&u).unwrap(), expected);

        let n = E::Newtype(1);
        let expected = r#"(Newtype:1)"#;
        assert_eq!(to_string(&n).unwrap(), expected);

        let t = E::Tuple(1, 2);
        let expected = r#"(Tuple:!(1,2))"#;
        assert_eq!(to_string(&t).unwrap(), expected);

        let s = E::Struct { a: 1 };
        let expected = r#"(Struct:(a:1))"#;
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
        assert_eq!(to_string(&3.14f64).unwrap(), "3.14");
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
}
