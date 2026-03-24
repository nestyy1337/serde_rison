use serde::Deserialize;
use serde::de::{DeserializeSeed, EnumAccess, MapAccess, SeqAccess, VariantAccess, Visitor};

use crate::Error;
use crate::Result;

pub fn from_str<'de, T: Deserialize<'de>>(input: &'de str) -> Result<T> {
    let mut deserializer = Deserializer::new(input);
    let t = T::deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(t)
}

pub struct Deserializer<'de> {
    input: &'de str,
    index: usize,
}

impl<'de> Deserializer<'de> {
    #[must_use]
    pub fn new(input: &'de str) -> Self {
        Self { input, index: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.index..].chars().next()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.index += ch.len_utf8();
        Some(ch)
    }

    fn consume(&mut self, expected: char) -> Result<()> {
        match self.next() {
            Some(c) if c == expected => Ok(()),
            Some(c) => Err(Error::UnexpectedCharacter(c)),
            None => Err(Error::UnexpectedEndOfInput),
        }
    }

    fn end(&self) -> Result<()> {
        if self.index == self.input.len() {
            Ok(())
        } else {
            Err(Error::UnexpectedCharacter(self.peek().unwrap()))
        }
    }

    fn parse_number<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let start = self.index;

        if self.peek() == Some('-') {
            self.index += 1;
        }

        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.index += 1;
        }

        if self.peek() == Some('.') {
            self.index += 1;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.index += 1;
            }
            let f: f64 = self.input[start..self.index]
                .parse()
                .map_err(|_| Error::InvalidNumber(self.input[start..self.index].to_string()))?;
            return visitor.visit_f64(f);
        }

        let s = &self.input[start..self.index];
        if let Ok(v) = s.parse::<u64>() {
            visitor.visit_u64(v)
        } else {
            let v: i64 = s.parse().map_err(|_| Error::InvalidNumber(s.to_string()))?;
            visitor.visit_i64(v)
        }
    }

    fn parse_object<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.consume('(')?;
        let value = visitor.visit_map(RisonMapAccess {
            de: self,
            first: true,
        })?;
        self.consume(')')?;
        Ok(value)
    }

    fn parse_array<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.consume('!')?;
        self.consume('(')?;
        let value = visitor.visit_seq(RisonSeqAccess {
            de: self,
            first: true,
        })?;
        self.consume(')')?;
        Ok(value)
    }

    fn parse_quoted_string<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.consume('\'')?;
        let start = self.index;
        let mut has_escapes = false;

        let mut i = self.index;
        while i < self.input.len() {
            match self.input.as_bytes()[i] {
                b'\'' => break,
                b'!' => {
                    has_escapes = true;
                    i += 2;
                }
                _ => i += 1,
            }
        }

        if has_escapes {
            let mut buf = String::new();
            loop {
                match self.peek() {
                    Some('\'') => break,
                    Some('!') => {
                        self.index += 1; // skip '!'
                        match self.next() {
                            Some(c @ ('!' | '\'')) => buf.push(c),
                            Some(c) => return Err(Error::UnexpectedCharacter(c)),
                            None => return Err(Error::UnexpectedEndOfInput),
                        }
                    }
                    Some(c) => {
                        buf.push(c);
                        self.index += c.len_utf8();
                    }
                    None => return Err(Error::UnexpectedEndOfInput),
                }
            }
            self.consume('\'')?;
            visitor.visit_string(buf)
        } else {
            self.index = i;
            let s = &self.input[start..self.index];
            self.consume('\'')?;
            visitor.visit_borrowed_str(s)
        }
    }

    fn parse_unquoted_string<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let start = self.index;
        while self
            .peek()
            .is_some_and(|c| !matches!(c, '(' | ')' | ',' | ':' | '!' | '\''))
        {
            self.index += self.peek().unwrap().len_utf8();
        }
        if self.index == start {
            return Err(Error::UnexpectedEndOfInput);
        }
        visitor.visit_borrowed_str(&self.input[start..self.index])
    }

    fn parse_bang<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // peek past the '!'
        let next = self.input[self.index + 1..].chars().next();
        match next {
            Some('(') => self.parse_array(visitor),
            Some('t') => {
                self.index += 2;
                visitor.visit_bool(true)
            }
            Some('f') => {
                self.index += 2;
                visitor.visit_bool(false)
            }
            Some('n') => {
                self.index += 2;
                visitor.visit_unit()
            }
            Some(c) => Err(Error::UnexpectedCharacter(c)),
            None => Err(Error::UnexpectedEndOfInput),
        }
    }

    fn deserialize_number<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_number(visitor)
    }
}

macro_rules! deserialize_number {
    ($method:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            self.deserialize_number(visitor)
        }
    };
}

impl<'de> serde::de::Deserializer<'de> for &mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek() {
            Some('(') => self.parse_object(visitor),
            Some('!') => self.parse_bang(visitor),
            Some('\'') => self.parse_quoted_string(visitor),
            Some(c) if c.is_ascii_digit() || c == '-' => self.parse_number(visitor),
            Some(_) => self.parse_unquoted_string(visitor),
            None => Err(Error::UnexpectedEndOfInput),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    deserialize_number!(deserialize_i8);
    deserialize_number!(deserialize_i16);
    deserialize_number!(deserialize_i32);
    deserialize_number!(deserialize_i64);
    deserialize_number!(deserialize_u8);
    deserialize_number!(deserialize_u16);
    deserialize_number!(deserialize_u32);
    deserialize_number!(deserialize_u64);
    deserialize_number!(deserialize_f32);
    deserialize_number!(deserialize_f64);

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_bytes(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek() {
            Some('!') if self.input[self.index + 1..].starts_with('n') => {
                self.index += 2;
                visitor.visit_none()
            }
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_array(visitor)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_object(visitor)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.parse_object(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.peek() {
            Some('(') => {
                self.consume('(')?;
                let value = visitor.visit_enum(RisonEnumAccess { de: self })?;
                self.consume(')')?;
                Ok(value)
            }
            _ => visitor.visit_enum(RisonEnumAccess { de: self }),
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

struct RisonSeqAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'de> SeqAccess<'de> for RisonSeqAccess<'_, 'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.de.peek() == Some(')') {
            return Ok(None);
        }
        if !self.first {
            self.de.consume(',')?;
        }
        self.first = false;
        seed.deserialize(&mut *self.de).map(Some)
    }
}

struct RisonMapAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
    first: bool,
}

impl<'de> MapAccess<'de> for RisonMapAccess<'_, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.de.peek() == Some(')') {
            return Ok(None);
        }
        if !self.first {
            self.de.consume(',')?;
        }
        self.first = false;
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        self.de.consume(':')?;
        seed.deserialize(&mut *self.de)
    }
}

struct RisonEnumAccess<'a, 'de> {
    de: &'a mut Deserializer<'de>,
}

impl<'de> EnumAccess<'de> for RisonEnumAccess<'_, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let val = seed.deserialize(&mut *self.de)?;
        Ok((val, self))
    }
}

impl<'de> VariantAccess<'de> for RisonEnumAccess<'_, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        self.de.consume(':')?;
        seed.deserialize(&mut *self.de)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.consume(':')?;
        serde::de::Deserializer::deserialize_seq(&mut *self.de, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.de.consume(':')?;
        serde::de::Deserializer::deserialize_map(&mut *self.de, visitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Test {
            int: u32,
            seq: Vec<String>,
            text: String,
        }

        let input = "(int:1,seq:!(a,b),text:'test with space')";
        let expected = Test {
            int: 1,
            seq: vec!["a".to_string(), "b".to_string()],
            text: "test with space".to_string(),
        };
        assert_eq!(from_str::<Test>(input).unwrap(), expected);
    }

    #[test]
    fn test_enum() {
        #[derive(Deserialize, Debug, PartialEq)]
        enum E {
            Unit,
            Newtype(u32),
            Tuple(u32, u32),
            Struct { a: u32 },
        }

        assert_eq!(from_str::<E>("Unit").unwrap(), E::Unit);
        assert_eq!(from_str::<E>("(Newtype:1)").unwrap(), E::Newtype(1));
        assert_eq!(from_str::<E>("(Tuple:!(1,2))").unwrap(), E::Tuple(1, 2));
        assert_eq!(from_str::<E>("(Struct:(a:1))").unwrap(), E::Struct { a: 1 });
    }

    #[test]
    fn test_primitives() {
        assert_eq!(from_str::<bool>("!t").unwrap(), true);
        assert_eq!(from_str::<bool>("!f").unwrap(), false);
        assert_eq!(from_str::<u64>("42").unwrap(), 42);
        assert_eq!(from_str::<i64>("-7").unwrap(), -7);
        assert_eq!(from_str::<f64>("3.14").unwrap(), 3.14);
        assert_eq!(from_str::<String>("hello").unwrap(), "hello");
        assert_eq!(from_str::<String>("'hello world'").unwrap(), "hello world");
        assert_eq!(from_str::<Option<u32>>("!n").unwrap(), None);
        assert_eq!(from_str::<Option<u32>>("5").unwrap(), Some(5));
    }

    #[test]
    fn test_vec() {
        assert_eq!(from_str::<Vec<u32>>("!(1,2,3)").unwrap(), vec![1, 2, 3]);
    }

    #[test]
    fn test_escaped_bang() {
        assert_eq!(from_str::<String>("'bang!!'").unwrap(), "bang!");
    }

    #[test]
    fn test_escaped_quote() {
        assert_eq!(from_str::<String>("'it!'s here'").unwrap(), "it's here");
    }

    #[test]
    fn test_escaped_both() {
        assert_eq!(
            from_str::<String>("'it!'s a bang!!'").unwrap(),
            "it's a bang!"
        );
    }

    #[test]
    fn test_no_escapes_in_quoted() {
        assert_eq!(from_str::<String>("'hello world'").unwrap(), "hello world");
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(from_str::<String>("''").unwrap(), "");
    }

    #[test]
    fn test_empty_object() {
        assert_eq!(
            from_str::<std::collections::BTreeMap<String, u32>>("()").unwrap(),
            std::collections::BTreeMap::new()
        );
    }

    #[test]
    fn test_empty_array() {
        let empty: Vec<u32> = vec![];
        assert_eq!(from_str::<Vec<u32>>("!()").unwrap(), empty);
    }

    #[test]
    fn test_nested_object() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Inner {
            b: u32,
        }
        #[derive(Deserialize, Debug, PartialEq)]
        struct Outer {
            a: Inner,
        }
        assert_eq!(
            from_str::<Outer>("(a:(b:1))").unwrap(),
            Outer { a: Inner { b: 1 } }
        );
    }

    #[test]
    fn test_nested_array() {
        assert_eq!(
            from_str::<Vec<Vec<u32>>>("!(!(1,2),!(3,4))").unwrap(),
            vec![vec![1, 2], vec![3, 4]]
        );
    }

    #[test]
    fn test_mixed_nesting() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Mixed {
            a: Vec<u32>,
            b: Inner,
        }
        #[derive(Deserialize, Debug, PartialEq)]
        struct Inner {
            c: u32,
        }
        assert_eq!(
            from_str::<Mixed>("(a:!(1,2),b:(c:3))").unwrap(),
            Mixed {
                a: vec![1, 2],
                b: Inner { c: 3 },
            }
        );
    }

    #[test]
    fn test_negative_float() {
        assert_eq!(from_str::<f64>("-3.14").unwrap(), -3.14);
    }

    #[test]
    fn test_zero() {
        assert_eq!(from_str::<u64>("0").unwrap(), 0);
        assert_eq!(from_str::<f64>("0.0").unwrap(), 0.0);
    }

    #[test]
    fn test_single_char_string() {
        assert_eq!(from_str::<String>("a").unwrap(), "a");
    }

    #[test]
    fn test_bool_in_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Flags {
            a: bool,
            b: bool,
        }
        assert_eq!(
            from_str::<Flags>("(a:!t,b:!f)").unwrap(),
            Flags { a: true, b: false }
        );
    }

    #[test]
    fn test_null_option_in_struct() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Opt {
            a: Option<u32>,
            b: Option<u32>,
        }
        assert_eq!(
            from_str::<Opt>("(a:!n,b:5)").unwrap(),
            Opt {
                a: None,
                b: Some(5)
            }
        );
    }

    #[test]
    fn test_trailing_garbage() {
        assert!(from_str::<u32>("42abc").is_err());
    }

    #[test]
    fn test_unclosed_paren() {
        assert!(from_str::<std::collections::BTreeMap<String, u32>>("(a:1").is_err());
    }

    #[test]
    fn test_unclosed_quote() {
        assert!(from_str::<String>("'hello").is_err());
    }

    #[test]
    fn test_invalid_bang() {
        assert!(from_str::<bool>("!x").is_err());
    }

    #[test]
    fn test_empty_input() {
        assert!(from_str::<u32>("").is_err());
    }

    #[test]
    fn test_roundtrip_struct() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Test {
            int: u32,
            seq: Vec<String>,
            text: String,
        }
        let original = Test {
            int: 42,
            seq: vec!["hello".into(), "world".into()],
            text: "it's a test!".into(),
        };
        let rison = crate::to_string(&original).unwrap();
        let back: Test = from_str(&rison).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn test_roundtrip_enum() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        enum E {
            Unit,
            Newtype(u32),
            Tuple(u32, u32),
            Struct { a: u32 },
        }
        for val in [E::Unit, E::Newtype(7), E::Tuple(1, 2), E::Struct { a: 99 }] {
            let rison = crate::to_string(&val).unwrap();
            let back: E = from_str(&rison).unwrap();
            assert_eq!(val, back);
        }
    }

    #[test]
    fn test_roundtrip_nested() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Inner {
            x: Vec<i32>,
        }
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct Outer {
            name: String,
            inner: Inner,
            flag: bool,
        }
        let original = Outer {
            name: "it's complex!".into(),
            inner: Inner { x: vec![-1, 0, 1] },
            flag: true,
        };
        let rison = crate::to_string(&original).unwrap();
        let back: Outer = from_str(&rison).unwrap();
        assert_eq!(original, back);
    }
}
