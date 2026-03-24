# serde_rison

A [serde](https://serde.rs) implementation for the [RISON](https://github.com/Nanonid/rison) data format.

Heavily inspired by David Tolnay's [serde_json](https://github.com/serde-rs/json). RISON is structurally similar to JSON, so much of the architecture carries over. This was a quick project for company's needs so I'm not planning on implementing generic Serializer over W: io::Write + Formatter, although if there is need on our side I'll convert it.

## What's different from serde_json

- No `Writer` / `Formatter` abstraction: just a plain `to_string` / `from_str` interface
- No `io::Write` support: serialization builds a `String` in memory
- No pretty-printing
- No `to_value` yet

## Usage

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct Person {
    name: String,
    age: u32,
    active: bool,
}

let person = Person {
    name: "John".into(),
    age: 30,
    active: true,
};

// Serialize
let rison = serde_rison::to_string(&person).unwrap();
assert_eq!(rison, "(name:John,age:30,active:!t)");

// Deserialize
let back: Person = serde_rison::from_str(&rison).unwrap();
assert_eq!(person, back);
```
