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

    fn scan_number(&mut self) -> Result<NumberToken<'de>> {
        let start = self.index;

        if self.peek() == Some('-') {
            self.index += 1;
        }

        let integer_start = self.index;

        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.index += 1;
        }

        if self.index == integer_start {
            return Err(Error::InvalidNumber(
                self.input[start..self.index].to_string(),
            ));
        }

        let mut is_float = false;
        if self.peek() == Some('.') {
            is_float = true;
            self.index += 1;
            let fraction_start = self.index;

            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.index += 1;
            }

            if self.index == fraction_start {
                return Err(Error::InvalidNumber(
                    self.input[start..self.index].to_string(),
                ));
            }
        }

        if matches!(self.peek(), Some('E' | 'e')) {
            is_float = true;
            self.index += 1;

            if self.peek() == Some('-') {
                self.index += 1;
            }

            let exponent_start = self.index;
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.index += 1;
            }

            if self.index == exponent_start {
                return Err(Error::InvalidNumber(
                    self.input[start..self.index].to_string(),
                ));
            }
        }

        match self.peek() {
            None | Some(',' | ')' | ':') => {}
            Some(c) => return Err(Error::UnexpectedCharacter(c)),
        }

        let token = &self.input[start..self.index];
        Ok(if is_float {
            NumberToken::Float(token)
        } else {
            NumberToken::Integer(token)
        })
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

    fn parse_number<V>(&mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let token = match self.scan_number()? {
            NumberToken::Integer(token) => {
                if let Ok(value) = token.parse::<u64>() {
                    return visitor.visit_u64(value);
                }

                if let Ok(value) = token.parse::<i64>() {
                    return visitor.visit_i64(value);
                }

                token
            }
            NumberToken::Float(token) => token,
        };

        let value: f64 = token
            .parse()
            .map_err(|_| Error::InvalidNumber(token.to_string()))?;
        visitor.visit_f64(value)
    }
}

enum NumberToken<'de> {
    Integer(&'de str),
    Float(&'de str),
}

macro_rules! deserialize_number {
    ($method:ident) => {
        fn $method<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            self.parse_number(visitor)
        }
    };
}

macro_rules! deserialize_float {
    ($de_method:ident, $type:ident, $visit_method:ident) => {
        fn $de_method<V>(self, visitor: V) -> Result<V::Value>
        where
            V: Visitor<'de>,
        {
            let token = match self.scan_number()? {
                NumberToken::Integer(token) | NumberToken::Float(token) => token,
            };

            let value: $type = token
                .parse()
                .map_err(|_| Error::InvalidNumber(token.to_string()))?;
            visitor.$visit_method(value)
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

    deserialize_float!(deserialize_f32, f32, visit_f32);
    deserialize_float!(deserialize_f64, f64, visit_f64);

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
    use crate::Value;

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
    fn f64_de() {
        let val = "1e20";
        let val: Value = from_str(&val).unwrap();
        dbg!(&val);
    }

    // Floats are compared by bit pattern, not by value: `-0.0 == 0.0` is true
    // and `NaN == NaN` is false, so `assert_eq!` on f64 hides exactly the bugs
    // these tests are looking for.
    fn assert_bits(input: &str, expected: f64) {
        let got: f64 =
            from_str(input).unwrap_or_else(|e| panic!("{input:?} failed to parse: {e:?}"));
        assert_eq!(
            got.to_bits(),
            expected.to_bits(),
            "{input:?}: got {got:e} ({:#018x}), want {expected:e} ({:#018x})",
            got.to_bits(),
            expected.to_bits()
        );
    }

    #[test]
    fn test_float_exponent_forms() {
        assert_bits("1e20", 1e20);
        assert_bits("1e-7", 1e-7);
        assert_bits("-1e20", -1e20);
        assert_bits("-1e-7", -1e-7);
        assert_bits("1.5e-10", 1.5e-10);
        assert_bits("-2.5e-7", -2.5e-7);
        assert_bits("1.5e10", 1.5e10);
        assert_bits("-1.5e10", -1.5e10);
        assert_bits("-1e3", -1e3);
        assert_bits("0e0", 0.0);

        // This crate currently accepts uppercase E as an extension. Canonical
        // RISON only permits lowercase e.
        assert_bits("1E20", 1e20);
        assert_bits("-1.5E-10", -1.5e-10);
    }

    #[test]
    fn test_float_zero_forms_keep_their_sign() {
        for input in ["-0.0", "-0e0", "-0e-0", "-0.0e0", "-0.0e-0"] {
            let value: f64 = from_str(input).unwrap();
            assert!(value.is_sign_negative(), "{input:?} lost its sign");
        }
    }

    #[test]
    fn test_float_landmarks() {
        assert_bits("5e-324", f64::from_bits(1)); // smallest subnormal
        assert_bits("2.2250738585072014e-308", f64::MIN_POSITIVE);
        assert_bits("1.7976931348623157e308", f64::MAX);
        assert_bits("0.1", f64::from_bits(0x3fb9_9999_9999_999a));

        // The two values that hung Java (CVE-2010-4476) and PHP (CVE-2010-4645);
        // they are adjacent doubles straddling the normal/subnormal boundary.
        assert_bits(
            "2.2250738585072012e-308",
            f64::from_bits(0x0010_0000_0000_0000),
        );
        assert_bits(
            "2.2250738585072011e-308",
            f64::from_bits(0x000f_ffff_ffff_ffff),
        );
    }

    #[test]
    fn test_float_shortest_roundtrip_aliases() {
        // The smallest subnormal owns a huge decimal interval, so five different
        // one-digit strings all name the same double. Only the shortest-and-nearest
        // is what a printer should emit.
        for input in ["3e-324", "4e-324", "5e-324", "6e-324", "7e-324"] {
            assert_bits(input, f64::from_bits(1));
        }
        assert_bits("2e-324", 0.0); // rounds down, ties-to-even
        assert_bits("8e-324", f64::from_bits(2)); // rounds up
    }

    #[test]
    fn test_float_rejects_malformed() {
        // rison forbids `+` in the exponent ("the e+ exponent format is forbidden")
        // and the spec also removes the uppercase `E` forms.
        for input in [
            "1e", "1e-", "1e+20", "1.0e", "1.0e-", "1.0e+20", ".5", "-.5", "1.", "1.e5", "1..0",
            "1.0.0", "1e2e3", "1.0e2e3", "+1", "-", "--1",
        ] {
            assert!(
                from_str::<f64>(input).is_err(),
                "{input:?} should be rejected, got {:?}",
                from_str::<f64>(input)
            );
        }
    }

    #[test]
    fn test_float_roundtrip_bit_exact() {
        for original in [
            0.0,
            -0.0,
            1.0,
            -2.5,
            0.1,
            1e20,
            1e-7,
            f64::MIN_POSITIVE,
            f64::from_bits(1),
            f64::MAX,
            f64::MIN,
            std::f64::consts::PI,
        ] {
            let rison = crate::to_string(&original).unwrap();
            let back: f64 = from_str(&rison).unwrap_or_else(|e| {
                panic!("{original:e} serialized to {rison:?}, which failed: {e:?}")
            });
            assert_eq!(
                back.to_bits(),
                original.to_bits(),
                "{original:e} -> {rison:?} -> {back:e}"
            );
        }
    }

    #[test]
    fn test_negative_zero_keeps_sign() {
        // `-0.0` and `0.0` compare equal but are different numbers:
        // 1.0 / -0.0 is -inf. The sign bit has to survive the round trip.
        let from_float: f64 = from_str("-0.0").unwrap();
        assert!(from_float.is_sign_negative(), "-0.0 lost its sign");

        // `-0` is a legal rison number and takes the integer path.
        let from_int: f64 = from_str("-0").unwrap();
        assert!(from_int.is_sign_negative(), "-0 lost its sign");
    }

    #[test]
    fn test_integer_boundaries() {
        assert_eq!(from_str::<u64>("18446744073709551615").unwrap(), u64::MAX);
        assert_eq!(from_str::<i64>("-9223372036854775808").unwrap(), i64::MIN);

        // Above u64::MAX there is no integer type left; rison's only number type
        // is a double, so it should degrade to f64 rather than error.
        assert_bits("18446744073709551616", 18446744073709551616.0);

        // Beyond 2^53 consecutive integers are not representable, so these two
        // distinct literals must land on the same double.
        assert_bits("9007199254740993", 9007199254740992.0);
    }

    #[test]
    fn test_generic_number_representations() {
        use crate::Number;

        for (input, expected) in [
            ("0", Number::from(0_u64)),
            ("42", Number::from(42_u64)),
            ("-7", Number::from(-7_i64)),
            ("18446744073709551615", Number::from(u64::MAX)),
            ("-9223372036854775808", Number::from(i64::MIN)),
            ("1.0", Number::from_f64(1.0).unwrap()),
            ("1e2", Number::from_f64(100.0).unwrap()),
            (
                "18446744073709551616",
                Number::from_f64(18446744073709551616.0).unwrap(),
            ),
            (
                "-9223372036854775809",
                Number::from_f64(-9223372036854775808.0).unwrap(),
            ),
        ] {
            assert_eq!(from_str::<Number>(input).unwrap(), expected, "{input:?}");
            assert_eq!(
                from_str::<Value>(input).unwrap(),
                Value::Number(expected),
                "{input:?}"
            );
        }
    }

    #[test]
    fn test_float_targets_accept_integer_tokens() {
        for input in ["0", "-0", "42", "-42", "18446744073709551616"] {
            assert_eq!(
                from_str::<f32>(input).unwrap().to_bits(),
                input.parse::<f32>().unwrap().to_bits(),
                "{input:?}"
            );
            assert_eq!(
                from_str::<f64>(input).unwrap().to_bits(),
                input.parse::<f64>().unwrap().to_bits(),
                "{input:?}"
            );
        }
    }

    #[test]
    fn test_number_scanner_preserves_delimiters() {
        for (input, is_float) in [
            ("42", false),
            ("-7", false),
            ("1.5", true),
            ("1e2", true),
            ("1.5e-2", true),
            ("1E2", true),
        ] {
            for delimiter in ["", ",", ")", ":"] {
                let source = format!("{input}{delimiter}");
                let mut deserializer = Deserializer::new(&source);
                let (token, token_is_float) = match deserializer.scan_number().unwrap() {
                    NumberToken::Integer(token) => (token, false),
                    NumberToken::Float(token) => (token, true),
                };

                assert_eq!((token, token_is_float), (input, is_float));
                assert_eq!(&source[deserializer.index..], delimiter);
            }
        }
    }

    #[test]
    fn test_number_scanner_rejects_malformed() {
        for input in [
            "", "-", "--1", ".5", "-.5", "1.", "1e", "1e-", "1.0e", "1.0e-", "1e+2", "1.2-3",
            "1.2e3e4", "1..2", "1.0.0", "1e2e3",
        ] {
            assert!(
                Deserializer::new(input).scan_number().is_err(),
                "{input:?} should not produce a number token"
            );
        }
    }

    #[test]
    fn test_float_overflow_and_underflow() {
        // Both fail silently in `str::parse`. Decide deliberately whether rison
        // should propagate that or reject the literal.
        assert_bits("1e-400", 0.0);
        assert_bits("1e999", f64::INFINITY);
    }

    #[test]
    fn test_float_inside_structure() {
        // The scan has to stop at the delimiter and hand the cursor back intact.
        #[derive(Deserialize, Debug, PartialEq)]
        struct S {
            a: f64,
            b: f64,
            c: Vec<f64>,
        }
        let got: S = from_str("(a:1e20,b:-2.5e-7,c:!(5e-324,1,0.5))").unwrap();
        assert_eq!(got.a.to_bits(), 1e20f64.to_bits());
        assert_eq!(got.b.to_bits(), (-2.5e-7f64).to_bits());
        assert_eq!(got.c[0].to_bits(), f64::from_bits(1).to_bits());
        assert_eq!(got.c[1].to_bits(), 1.0f64.to_bits());
        assert_eq!(got.c[2].to_bits(), 0.5f64.to_bits());
    }

    #[test]
    fn test_nonfinite_is_null() {
        // rison has no syntax for inf or NaN, so the serializer collapses them
        // to `!n`. That is lossy but mandated; this pins the behaviour.
        assert_eq!(crate::to_string(&f64::INFINITY).unwrap(), "!n");
        assert_eq!(crate::to_string(&f64::NEG_INFINITY).unwrap(), "!n");
        assert_eq!(crate::to_string(&f64::NAN).unwrap(), "!n");
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

    #[test]
    fn lossy() {
        let string = "99.99999999999999";
        let rison: f32 = from_str(string).unwrap();

        dbg!(string, rison);
    }
}
