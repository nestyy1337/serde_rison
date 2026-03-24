mod de;
mod se;

use std::collections::BTreeMap;

use crate::Number;

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum Value {
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(bool) => write!(f, "{bool}"),
            Self::Number(number) => write!(f, "{number}"),
            Self::String(string) => write!(f, "{string:?}"),
            Self::Array(array) => f.debug_list().entries(array).finish(),
            Self::Object(object) => f.debug_map().entries(object).finish(),
        }
    }
}

pub fn from_value<T>(value: Value) -> crate::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(value)
}
