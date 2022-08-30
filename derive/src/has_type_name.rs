use proc_macro2::{Literal, TokenStream};

use quote::quote;
use syn::spanned::Spanned;

pub(crate) fn derive_impl(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let type_generics2 = input.generics.params.iter();
    // TODO: Actually parse #[has_type_name(qualifiers = "...")] attr if present
    let qualifiers = quote!(std::vec::Vec::new());
    let simple_name = Literal::string(&ident.to_string());
    let generic_args = quote!(vec![#( #type_generics2::type_name() ),*]);
    Ok(quote! {
        impl #impl_generics structural_reflection::HasTypeName for #ident #type_generics #where_clause {
            fn type_name() -> structural_reflection::RustTypeName {
                structural_reflection::RustTypeName::Ident {
                    qualifiers: #qualifiers,
                    simple_name: #simple_name,
                    generic_args: #generic_args,
                }
            }
        }
    })
}