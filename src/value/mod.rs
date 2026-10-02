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

#[cfg(feature = "json")]
impl TryFrom<serde_json::Value> for Value {
    type Error = crate::Error;

    /// Fails only for numbers outside the f64 range.
    fn try_from(value: serde_json::Value) -> crate::Result<Self> {
        Ok(match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(b),
            serde_json::Value::Number(n) => Self::Number(n.try_into()?),
            serde_json::Value::String(s) => Self::String(s),
            serde_json::Value::Array(arr) => Self::Array(
                arr.into_iter()
                    .map(Value::try_from)
                    .collect::<crate::Result<_>>()?,
            ),
            serde_json::Value::Object(map) => Self::Object(
                map.into_iter()
                    .map(|(k, v)| Ok((k, Value::try_from(v)?)))
                    .collect::<crate::Result<_>>()?,
            ),
        })
    }
}

pub fn from_value<T>(value: Value) -> crate::Result<T>
where
    T: serde::de::DeserializeOwned,
{
    T::deserialize(value)
}

#[cfg(all(test, feature = "json"))]
mod json_tests {
    use super::*;
    use crate::number::N;

    #[test]
    fn null() {
        let v = Value::try_from(serde_json::Value::Null).unwrap();
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn bools() {
        let t = Value::try_from(serde_json::Value::Bool(true)).unwrap();
        let f = Value::try_from(serde_json::Value::Bool(false)).unwrap();
        assert_eq!(t, Value::Bool(true));
        assert_eq!(f, Value::Bool(false));
    }

    #[test]
    fn positive_integer() {
        let v = Value::try_from(serde_json::json!(42)).unwrap();
        assert_eq!(v, Value::Number(Number { n: N::PosInt(42) }));
    }

    #[test]
    fn negative_integer() {
        let v = Value::try_from(serde_json::json!(-7)).unwrap();
        assert_eq!(v, Value::Number(Number { n: N::NegInt(-7) }));
    }

    #[test]
    fn float() {
        let v = Value::try_from(serde_json::json!(2.5)).unwrap();
        assert!(matches!(v, Value::Number(n) if n.as_f64() == Some(2.5)));
    }

    #[test]
    fn string() {
        let v = Value::try_from(serde_json::json!("hello")).unwrap();
        assert_eq!(v, Value::String("hello".into()));
    }

    #[test]
    fn empty_array() {
        let v = Value::try_from(serde_json::json!([])).unwrap();
        assert_eq!(v, Value::Array(vec![]));
    }

    #[test]
    fn mixed_array() {
        let v = Value::try_from(serde_json::json!([1, "two", null, false])).unwrap();
        let Value::Array(arr) = v else {
            panic!("expected array")
        };
        assert_eq!(arr.len(), 4);
        assert_eq!(arr[1], Value::String("two".into()));
        assert_eq!(arr[2], Value::Null);
        assert_eq!(arr[3], Value::Bool(false));
    }

    #[test]
    fn object() {
        let v = Value::try_from(serde_json::json!({"a": 1, "b": "two"})).unwrap();
        let Value::Object(map) = v else {
            panic!("expected object")
        };
        assert_eq!(map.len(), 2);
        assert_eq!(map["b"], Value::String("two".into()));
    }

    #[test]
    fn nested_structure() {
        let v = Value::try_from(serde_json::json!({
            "filters": [{"key": "status", "value": "open"}],
            "time": {"from": "now-30m", "to": "now"}
        }))
        .unwrap();
        let Value::Object(map) = v else {
            panic!("expected object")
        };
        let Value::Array(filters) = &map["filters"] else {
            panic!("expected array")
        };
        let Value::Object(filter) = &filters[0] else {
            panic!("expected object")
        };
        assert_eq!(filter["key"], Value::String("status".into()));
    }

    #[test]
    fn roundtrip_serialize() {
        let json = serde_json::json!({
            "filters": [],
            "refreshInterval": {"pause": true, "value": 60000},
            "time": {"from": "2024-01-01T00:00:00Z", "to": "2024-01-01T01:00:00Z"}
        });
        let rison_value = Value::try_from(json).unwrap();
        let rison_str = crate::to_string(&rison_value).unwrap();

        assert!(rison_str.starts_with('('));
        assert!(rison_str.ends_with(')'));
        assert!(rison_str.contains("filters:!()"));
        assert!(rison_str.contains("pause:!t"));
        assert!(rison_str.contains("value:60000"));
    }
}
