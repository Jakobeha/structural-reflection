use proc_macro2::TokenStream;

use quote::quote;
use syn::{Data, DataStruct, DataEnum};
use syn::spanned::Spanned;

pub(crate) fn derive_impl(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let repr = input.attrs.iter()
        .find(|a| a.path.segments.last().unwrap().ident == "repr")
        .and_then(|a| a.parse_args::<syn::Ident>().ok())
        .map_or_else(|| String::from("Rust"), |ident| ident.to_string());
    match repr.as_str() {
        "C" | "transparent" => derive_c_impl(input),
        _ => Err(syn::Error::new(
            input.span(),
            "HasStructure can only be derived for types with `repr(C)` or `repr(transparent)`",
        )),
    }
}

fn derive_c_impl(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    match &input.data {
        Data::Struct(s) => derive_c_struct(&input, s),
        Data::Enum(e) => derive_c_enum(&input, e),
        Data::Union(u) => Err(syn::Error::new(
            u.union_token.span(),
            "HasStructure cannot be derived for unions (maybe in the future, submit an issue or PR)",
        )),
    }
}

fn derive_c_struct(input: &syn::DeriveInput, s: &DataStruct) -> syn::Result<TokenStream> {
    todo!()
}

fn derive_c_enum(input: &syn::DeriveInput, s: &DataEnum) -> syn::Result<TokenStream> {
    todo!()
}