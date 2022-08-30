# structural-reflection: reflection crate for type names and structures, with structural subtyping

[![version](https://img.shields.io/crates/v/structural-reflection)](https://crates.io/crates/structural-reflection)
[![documentation](https://docs.rs/structural-reflection/badge.svg)](https://docs.rs/structural-reflection)
![LICENSE](https://img.shields.io/crates/l/structural-reflection)

`structural-reflection` is a small reflection crate which provides:

- [`RustType`](https://docs.rs/structural-reflection/latest/data/RustType), [`RustTypeName`](https://docs.rs/structural-reflection/latest/data/RustTypeName), and [`TypeStructure`](https://docs.rs/structural-reflection/latest/data/TypeStructure): runtime representation of rust type info
- [`HasTypeName`](https://docs.rs/structural-reflection/latest/derive/HasTypeName) and [`HasStructure`](https://docs.rs/structural-reflection/latest/derive/HasStructure): derivable traits which let you get the above representations from the compile-time type


- Registry lets you register and get representations for types which don't implement the above traits.
- [`RustTypeName`] can be parsed from and printed to a string
- Structural subtyping and structural type unification, which supports unresolved (`Opaque`) types.

Use cases:

- Runtime type inspection
- Safe `Result` conversion from a value to a structural supertype
- Support for DSLs in other languages to use and define their own Rust types with safe transmute

Compared to other libraries ([reflect](https://crates.io/crates/reflect) and [bevy_reflect](https://crates.io/crates/bevy_reflect)), `structural-reflection` has different use cases. `reflect` and `bevy_reflect` involve safely manipulating Rust values via reflection directly, and provide much more operations in that regard (e.g. accessing fields by string name). In `structural-reflection` the only safe way to manipulate Rust values is to convert them into structural supertypes, but what you will probably really do is unsafely transmute to raw bytes, interact with those bytes in another program, and then unsafely transmute back.