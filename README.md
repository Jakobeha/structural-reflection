# structural-reflection: reflection crate for type names and structures, with structural subtyping

[![version](https://img.shields.io/crates/v/structural-reflection)](https://crates.io/crates/structural-reflection)
[![documentation](https://docs.rs/structural-reflection/badge.svg)](https://docs.rs/structural-reflection)
![LICENSE](https://img.shields.io/crates/l/structural-reflection)

`structural-reflection` is a small reflection crate which provides:

- [`RustType`](https://docs.rs/structural-reflection/latest/data/RustType), [`RustTypeName`](https://docs.rs/structural-reflection/latest/data/RustTypeName), and [`TypeStructure`](https://docs.rs/structural-reflection/latest/data/TypeStructure): runtime representation of rust type info
- [`HasTypeName`](https://docs.rs/structural-reflection/latest/derive/HasTypeName) and [`HasStructure`](https://docs.rs/structural-reflection/latest/derive/HasStructure): derivable traits which let you get the above representations from the compile-time type

- Registry lets you register and get representations for types which don't implement the above traits.
- [`RustTypeName`] can be parsed from and printed to a string
- Structural subtyping, e.g. structure with more fields is a subtype of structure with less fields (see [`TypeStructure::is_structural_subtype_of`](https://docs.rs/structural-reflection/latest/data/TypeStructure/struct.TypeStructure.html#method.is_structural_subtype_of) for all rules)
- Biased unification (the unified type is always a subtype of lhs type but not necessarily rhs, see [`TypeStructure::unify`](https://docs.rs/structural-reflection/latest/data/TypeStructure/struct.TypeStructure.html#method.unify) for all rules)
- Unknown ("Opaque") types `Opaque`, `OpaqueTuple`, and `OpaqueFields`: they produce "unknown" on either side of `is_structural_subtype_of`, and become the other type when lhs of `unify`.

Use cases:

- Rust type name parsing and manipulation (though this is overengineered for that one case...)
- Runtime type inspection
- Safe `Result` conversion from a value to a structural supertype
- Support for DSLs in other languages to use and define their own Rust types with safe transmute

The key focus of `structural-reflection` is **structural subtyping**: `structural-reflection` defines its own type system and subtyping rules, and you can convert values into supertypes at runtime (no compile time type-checking or conversions). `structural-subtyping` is *not* intended to add subtyping to Rust itself (which is probably a bad idea), but instead allow Rust to interop with other languages which already have subtyping.

Compared to other libraries ([reflect](https://crates.io/crates/reflect) and [bevy_reflect](https://crates.io/crates/bevy_reflect)), `structural-reflection` has different use cases. `reflect` and `bevy_reflect` involve safely manipulating Rust values via reflection directly, and provide much more operations in that regard (e.g. accessing fields by string name). In `structural-reflection`, though you can query type info, the only way to manipulate Rust values is to convert them into structural supertypes.