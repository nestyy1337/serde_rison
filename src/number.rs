use core::fmt::{self, Debug, Display};
use core::hash::{Hash, Hasher};
use serde::de::{self, Unexpected, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer, forward_to_deserialize_any};

use crate::error::Error;

/// Represents a RISON number, whether integer or floating point.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Number {
    pub(crate) n: N,
}

#[derive(Copy, Clone)]
pub(crate) enum N {
    PosInt(u64),
    /// Always less than zero.
    NegInt(i64),
    /// Always finite.
    Float(f64),
}

impl PartialEq for N {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (N::PosInt(a), N::PosInt(b)) => a == b,
            (N::NegInt(a), N::NegInt(b)) => a == b,
            (N::Float(a), N::Float(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for N {}

impl Hash for N {
    fn hash<H: Hasher>(&self, h: &mut H) {
        match *self {
            N::PosInt(i) => i.hash(h),
            N::NegInt(i) => i.hash(h),
            N::Float(f) => {
                if f == 0.0f64 {
                    0.0f64.to_bits().hash(h);
                } else {
                    f.to_bits().hash(h);
                }
            }
        }
    }
}

impl Number {
    #[must_use]
    pub fn is_i64(&self) -> bool {
        match self.n {
            N::PosInt(v) => i64::try_from(v).is_ok(),
            N::NegInt(_) => true,
            N::Float(_) => false,
        }
    }

    #[must_use]
    pub fn is_u64(&self) -> bool {
        matches!(self.n, N::PosInt(_))
    }

    #[must_use]
    pub fn is_f64(&self) -> bool {
        matches!(self.n, N::Float(_))
    }

    #[must_use]
    pub fn as_i64(&self) -> Option<i64> {
        match self.n {
            N::PosInt(v) if i64::try_from(v).is_ok() => Some(v as i64),
            N::NegInt(v) => Some(v),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_u64(&self) -> Option<u64> {
        match self.n {
            N::PosInt(v) => Some(v),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        match self.n {
            N::PosInt(v) => Some(v as f64),
            N::NegInt(v) => Some(v as f64),
            N::Float(v) => Some(v),
        }
    }

    #[must_use]
    pub fn from_f64(f: f64) -> Option<Number> {
        if f.is_finite() {
            Some(Number { n: N::Float(f) })
        } else {
            None
        }
    }

    pub(crate) fn unexpected(&self) -> Unexpected<'_> {
        match self.n {
            N::PosInt(v) => Unexpected::Unsigned(v),
            N::NegInt(v) => Unexpected::Signed(v),
            N::Float(v) => Unexpected::Float(v),
        }
    }
}

impl Display for Number {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.n {
            N::PosInt(v) => Display::fmt(&v, f),
            N::NegInt(v) => Display::fmt(&v, f),
            N::Float(v) => Display::fmt(&v, f),
        }
    }
}

impl Debug for Number {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Number({self})")
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.n {
            N::PosInt(v) => serializer.serialize_u64(v),
            N::NegInt(v) => serializer.serialize_i64(v),
            N::Float(v) => serializer.serialize_f64(v),
        }
    }
}

impl<'de> Deserialize<'de> for Number {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct NumberVisitor;

        impl Visitor<'_> for NumberVisitor {
            type Value = Number;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a RISON number")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(v.into())
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(v.into())
            }

            fn visit_f64<E>(self, v: f64) -> Result<Number, E>
            where
                E: de::Error,
            {
                Number::from_f64(v).ok_or_else(|| de::Error::custom("not a finite RISON number"))
            }
        }

        deserializer.deserialize_any(NumberVisitor)
    }
}

impl<'de> Deserializer<'de> for Number {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.n {
            N::PosInt(v) => visitor.visit_u64(v),
            N::NegInt(v) => visitor.visit_i64(v),
            N::Float(v) => visitor.visit_f64(v),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

impl<'de> Deserializer<'de> for &'de Number {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.n {
            N::PosInt(v) => visitor.visit_u64(v),
            N::NegInt(v) => visitor.visit_i64(v),
            N::Float(v) => visitor.visit_f64(v),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

macro_rules! impl_from_unsigned {
    ($($ty:ty),*) => {
        $(
            impl From<$ty> for Number {
                fn from(u: $ty) -> Self {
                    Number { n: N::PosInt(u as u64) }
                }
            }
        )*
    };
}

macro_rules! impl_from_signed {
    ($($ty:ty),*) => {
        $(
            impl From<$ty> for Number {
                fn from(i: $ty) -> Self {
                    let n = if i < 0 {
                        N::NegInt(i as i64)
                    } else {
                        N::PosInt(i as u64)
                    };
                    Number { n }
                }
            }
        )*
    };
}

impl_from_unsigned!(u8, u16, u32, u64, usize);
impl_from_signed!(i8, i16, i32, i64, isize);

#[cfg(feature = "json")]
impl From<serde_json::Number> for Number {
    fn from(n: serde_json::Number) -> Self {
        if let Some(u) = n.as_u64() {
            u.into()
        } else if let Some(i) = n.as_i64() {
            i.into()
        } else if let Some(f) = n.as_f64() {
            Number::from_f64(f).expect("serde_json::Number should always be finite")
        } else {
            unreachable!("serde_json::Number is always one of u64, i64, or f64")
        }
    }
}
