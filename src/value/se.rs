use serde::Serialize;

use crate::Value;

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(bool) => serializer.serialize_bool(*bool),
            Self::Number(number) => number.serialize(serializer),
            Self::String(string) => serializer.serialize_str(string),
            Self::Array(array) => array.serialize(serializer),
            Self::Object(object) => object.serialize(serializer),
        }
    }
}
