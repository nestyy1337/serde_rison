use proptest::prelude::*;
use serde_rison::{Number, Value};

macro_rules! test_roundtrip {
    ($name:ident, $ty:ty) => {
        proptest! {
            #[test]
            fn $name(x in any::<$ty>()) {
                let s = serde_rison::to_string(&x).unwrap();
                prop_assert_eq!(serde_rison::from_str::<$ty>(&s).unwrap(), x);
            }
        }
    };
}

test_roundtrip!(i64_roundtrips, i64);
test_roundtrip!(u64_roundtrips, u64);
// test_roundtrip!(f64_roundtrips, f64);
test_roundtrip!(bool_roundtrips, bool);
test_roundtrip!(string_roundtrips, String);
test_roundtrip!(vec_roundtrips, Vec<i32>);
test_roundtrip!(option_roundtrips, Option<String>);
test_roundtrip!(tuple_roundtrips, (i32, String));
test_roundtrip!(map_roundtrips, std::collections::HashMap<String, i32>);
test_roundtrip!(set_roundtrips, std::collections::HashSet<i32>);
test_roundtrip!(unit_roundtrips, ());

proptest! {
    #[test]
    fn f64_roundtrips(x in any::<f64>()) {
        let s = serde_rison::to_string(&x).unwrap();
        prop_assert_eq!(serde_rison::from_str::<f64>(&s).unwrap().to_bits(), x.to_bits());
    }
}

proptest! {
    #[test]
    fn bad_exclamation_tokens(x in "!([^tfn(])?") {
        let s: Result<serde_rison::Value, serde_rison::Error> = serde_rison::from_str(&x);
        prop_assert!(s.is_err(), "only !t, !f, !n, !( are valid");
    }
}

proptest! {
    #[test]
    fn unterminated_string(x in "'[^'a-zA-Z0-9]*") {
        let s: Result<serde_rison::Value, serde_rison::Error> = serde_rison::from_str(&x);
        prop_assert!(s.is_err(), "unterminated always fails");
    }
}

#[test]
fn invalid_escape() {
    let invalid = "'it!x'";
    let s: Result<serde_rison::Value, serde_rison::Error> = serde_rison::from_str(invalid);
    assert!(s.is_err(), "x is invalid after !");
}

#[test]
fn exclamation_escaping_quote() {
    let invalid = "'a!'";
    let s: Result<serde_rison::Value, serde_rison::Error> = serde_rison::from_str(invalid);
    assert!(s.is_err(), "exclamation escaping the closing '");
}

const ID: &str = r"[^ '!:(),*@$0-9\-][^ '!:(),*@$]{0,8}";
const FLOAT: &str = r"-?[0-9]{1,15}(\.[0-9]{1,15})?[eE]-?[0-9]{1,2}|-?[0-9]{1,15}\.[0-9]{1,15}";

fn quote(s: &str) -> String {
    format!("'{}'", s.replace('!', "!!").replace('\'', "!'"))
}

fn string() -> impl Strategy<Value = (String, String)> {
    prop_oneof![
        ID.prop_flat_map(
            |s| prop_oneof![Just(s.clone()), Just(quote(&s))].prop_map(move |t| (t, s.clone()))
        ),
        any::<String>().prop_map(|s| (quote(&s), s)),
    ]
}

fn rison() -> impl Strategy<Value = (String, Value)> {
    let leaf = prop_oneof![
        Just(("!t".to_owned(), Value::Bool(true))),
        Just(("!f".to_owned(), Value::Bool(false))),
        Just(("!n".to_owned(), Value::Null)),
        (any::<i64>(), 0..3usize).prop_map(|(n, zeros)| {
            let digits = n.unsigned_abs().to_string();
            let sign = if n < 0 { "-" } else { "" };
            (
                format!("{sign}{}{digits}", "0".repeat(zeros)),
                Value::Number(Number::from(n)),
            )
        }),
        (any::<i32>(), 0..3usize).prop_map(|(n, zeros)| {
            let digits = n.unsigned_abs().to_string();
            let sign = if n < 0 { "-" } else { "" };
            (
                format!("{sign}{}{digits}", "0".repeat(zeros)),
                Value::Number(Number::from(n)),
            )
        }),
        (any::<u64>()).prop_map(|n| { (n.to_string(), Value::Number(Number::from(n))) }),
        (any::<u32>()).prop_map(|n| { (n.to_string(), Value::Number(Number::from(n))) }),
        (any::<u16>()).prop_map(|n| { (n.to_string(), Value::Number(Number::from(n))) }),
        (any::<u8>()).prop_map(|n| { (n.to_string(), Value::Number(Number::from(n))) }),
        FLOAT.prop_map(|s| {
            let n = Number::from_f64(s.parse().unwrap()).unwrap();
            (s, Value::Number(n))
        }),
        string().prop_map(|(t, s)| (t, Value::String(s))),
    ];
    leaf.prop_recursive(4, 64, 6, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..6).prop_map(|xs| {
                let (texts, values): (Vec<_>, Vec<_>) = xs.into_iter().unzip();
                (format!("!({})", texts.join(",")), Value::Array(values))
            }),
            prop::collection::vec((string(), inner), 0..6).prop_map(|kvs| {
                let texts: Vec<_> = kvs
                    .iter()
                    .map(|((k, _), (v, _))| format!("{k}:{v}"))
                    .collect();
                let map = kvs.into_iter().map(|((_, k), (_, v))| (k, v)).collect();
                (format!("({})", texts.join(",")), Value::Object(map))
            }),
        ]
    })
}

proptest! {
    #[test]
    fn parses_to_expected_value((text, expected) in rison()) {
        prop_assert_eq!(serde_rison::from_str::<Value>(&text)?, expected, "{:?}", text);
    }
}
