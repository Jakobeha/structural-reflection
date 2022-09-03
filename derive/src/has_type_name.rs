use proc_macro2::{Ident, Literal, TokenStream};

use quote::quote;
use syn::parse_quote;
use crate::common::{common_derive, recursive_impl_generics};

pub(crate) fn derive_impl(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let vis = &input.vis;
    let ident = &input.ident;
    let static_id_ident = Ident::new(&format!("__{}Static", ident), ident.span());
    let (_, type_generics, _) = input.generics.split_for_impl();
    #[allow(unused_variables)]
    let (type_params, where_clause) = common_derive(&input.generics);
    let impl_generics = recursive_impl_generics(&input.generics, &parse_quote!(structural_reflection::HasTypeName));
    let type_params = input.generics.type_params().map(|type_param| &type_param.ident).collect::<Vec<_>>();
    let (static_impl_generics, static_type_generics) = if type_params.is_empty() {
        (quote!(), quote!())
    } else {
        (quote!(<#(#type_params: 'static),*>), quote!(<#(#type_params::StaticId),*>))
    };
    // TODO: Actually parse #[has_type_name(qualifiers = "...")] attr if present
    let qualifiers = quote!(std::vec::Vec::new());
    let simple_name = Literal::string(&ident.to_string());
    let generic_args = quote!(vec![#( #type_params::type_name() ),*]);

    let phantom_type = quote!(std::marker::PhantomData<(#(#type_params),*)>);
    Ok(quote! {
        // Static doesn't actually need internals as only the TypeId is used
        #[automatically_derived]
        #[doc(hidden)]
        #vis struct #static_id_ident #static_impl_generics {
            __phantom: #phantom_type
        }

        #[automatically_derived]
        impl #impl_generics structural_reflection::HasTypeName for #ident #type_generics #where_clause {
            type StaticId = #static_id_ident #static_type_generics;

            fn type_name() -> structural_reflection::RustTypeName {
                structural_reflection::RustTypeName::Ident {
                    qualifiers: #qualifiers,
                    simple_name: String::from(#simple_name),
                    generic_args: #generic_args,
                }
            }
        }
    })
}