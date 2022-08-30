#![feature(iterator_try_collect)]

use proc_macro::TokenStream;
use syn::parse_macro_input;

mod has_type_name;
mod has_structure;
mod common;

/// Generates an implementation of the `HasTypeName` trait, which returns the type name.
///
/// There are no requirements for the body, however when using, the generic parameters must also derive `HasTypeName`.
///
/// # Example
///
/// ```rust
/// use structural_reflection::derive::HasTypeName;
///
/// #[derive(HasTypeName)]
/// #[repr(C)]
/// pub struct FooBar<T> {
///     baz: T
/// }
/// ```
#[proc_macro_derive(HasTypeName, attributes(has_type_name))]
pub fn derive_has_type_name(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    has_type_name::derive_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}

/// Generates an implementation of the `HasStructure` trait.
///
/// - `#[has_structure(opaque)]`: causes the field to be considered an opaque type.
/// - `#[has_structure(name = alternate)]`: causes the field to be named `alternate`.
///
/// The struct must have `repr(C)` or `repr(transparent)`, otherwise this will not compile.
/// Additionally, the fields must all derive `HasStructure` and be sized.
/// If a field doesn't you can use `#[has_structure(opaque)]` on it, which supports any sized type.
///
/// # Example
///
/// ```rust
/// use structural_reflection::derive::{HasTypeName, HasStructure};
///
/// #[derive(HasTypeName, HasStructure)]
/// #[repr(C)]
/// pub struct FooBar<T> {
///     baz: T,
///     qux: *const ()
/// }
/// ```
#[proc_macro_derive(HasStructure, attributes(has_structure))]
pub fn derive_has_structure(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);
    has_structure::derive_impl(input)
        .unwrap_or_else(|err| err.to_compile_error())
        .into()
}