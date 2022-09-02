use proc_macro2::{Ident, Literal, TokenStream};

use quote::quote;

pub(crate) fn derive_impl(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let static_id_ident = Ident::new(&format!("__{}Static", ident), ident.span());
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let type_generics2 = input.generics.params.iter();
    // TODO: Actually parse #[has_type_name(qualifiers = "...")] attr if present
    let qualifiers = quote!(std::vec::Vec::new());
    let simple_name = Literal::string(&ident.to_string());
    let generic_args = quote!(vec![#( #type_generics2::type_name() ),*]);

    let lifetimes = input.generics.lifetimes();
    let type_params = input.generics.type_params();
    let phantom_type = quote!(std::marker::PhantomData<#(#lifetimes)* (#(#type_params),*)>);
    Ok(quote! {
        /// Static doesn't actually need internals as only the TypeId is used
        #[automatically_derived]
        struct #static_id_ident #impl_generics #where_clause {
            __phantom: #phantom_type
        }

        #[automatically_derived]
        impl #impl_generics structural_reflection::HasTypeName for #ident #type_generics #where_clause {
            type StaticId = #static_id_ident #type_generics;

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