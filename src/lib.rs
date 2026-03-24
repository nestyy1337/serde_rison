//! # serde_rison
//!
//! A [serde](https://serde.rs) implementation for the
//! [RISON](https://github.com/Nanonid/rison) data format.
//!
//! RISON is a compact, URI-friendly alternative to JSON. It uses `()` instead
//! of `{}`, `!()` instead of `[]`, `!t`/`!f`/`!n` for booleans and null, and
//! supports unquoted strings where possible.
//!
//! ## Example
//!
//! ```
//! use serde::{Deserialize, Serialize};
//!
//! #[derive(Serialize, Deserialize, Debug, PartialEq)]
//! struct Person {
//!     name: String,
//!     age: u32,
//! }
//!
//! let person = Person {
//!     name: "John".into(),
//!     age: 30,
//! };
//!
//! // Serialize to RISON
//! let rison = serde_rison::to_string(&person).unwrap();
//! assert_eq!(rison, "(name:John,age:30)");
//!
//! // Deserialize from RISON
//! let back: Person = serde_rison::from_str(&rison).unwrap();
//! assert_eq!(person, back);
//! ```

mod de;
mod error;
mod number;
mod ser;
mod value;

pub use de::Deserializer;
pub use de::from_str;
pub use error::Error;
pub use number::Number;
pub use ser::Serializer;
pub use ser::to_string;
pub use value::Value;
pub use value::from_value;

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use serde::{Deserialize, Serialize};

    use super::*;

    #[derive(Serialize, Deserialize)]
    struct SomeStruct {
        a: i32,
        b: String,
        c: Vec<i32>,
    }

    #[test]
    fn printing_rison() {
        let some_struct = SomeStruct {
            a: 1,
            b: "!!".to_string(),
            c: vec![1, 2, 3],
        };
        let string = crate::to_string(&some_struct).unwrap();
        println!("{string}");
    }

    #[test]
    fn parsing_rison() {
        let string = "(a:1,b:'!!',c:!(1,2,3))";
        let some_struct: SomeStruct = crate::from_str(string).unwrap();
        assert_eq!(some_struct.a, 1);
        assert_eq!(some_struct.b, "!");
        assert_eq!(some_struct.c, vec![1, 2, 3]);
    }
}
