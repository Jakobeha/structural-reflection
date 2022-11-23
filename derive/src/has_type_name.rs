use lazy_static::lazy_static;
use proc_macro2::TokenStream;

use quote::quote;
use regex::Regex;
use syn::parse::{Parse, ParseStream};
use syn::parse_quote;
use crate::common::{common_derive, recursive_impl_generics};

struct HasTypeNameAttr {
    qualifier: Option<String>
}

pub(crate) fn derive_impl(input: syn::DeriveInput) -> syn::Result<TokenStream> {
    let vis = &input.vis;
    let ident = &input.ident;
    let static_id_ident = syn::Ident::new(&format!("__{}Static", ident), ident.span());
    let attr: Option<HasTypeNameAttr> = input.attrs.iter()
        .find(|attr| attr.path.is_ident("has_type_name"))
        .map(|attr| attr.parse_args()).transpose()?;
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
    let qualifier = match attr.as_ref().and_then(|attr| attr.qualifier.as_ref()) {
        None => quote!(structural_reflection::Qualifier::local()),
        Some(qualifier) => quote!(structural_reflection::Qualifier::try_from(#qualifier).unwrap())
    };
    let simple_name = syn::LitStr::new(&ident.to_string(), ident.span());
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
                    qualifier: #qualifier,
                    simple_name: String::from(#simple_name),
                    generic_args: #generic_args,
                }
            }
        }
    })
}

impl HasTypeNameAttr {
    fn new() -> Self {
        Self {
            qualifier: None
        }
    }
}

lazy_static! {
    static ref QUALIFIER_RE: Regex = Regex::new(r"^[a-zA-Z0-9_]+(::[a-zA-Z0-9_]+)*$").unwrap();
}

impl Parse for HasTypeNameAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut result = HasTypeNameAttr::new();
        loop {
            let name = input.parse::<syn::Ident>()?;
            input.parse::<syn::Token![=]>()?;
            match name.to_string().as_str() {
                "qualifier" => {
                    let value = input.parse::<syn::LitStr>()?;
                    if result.qualifier.is_some() {
                        return Err(syn::Error::new_spanned(value, "Duplicate qualifier attribute"));
                    }
                    let value_str = value.value();
                    // Ensure this is a valid qualifier
                    if !QUALIFIER_RE.is_match(&value_str) {
                        return Err(syn::Error::new_spanned(value, "Invalid qualifier format"));
                    }
                    result.qualifier = Some(value_str);
                }
                _ => return Err(syn::Error::new(name.span(), "Unknown attribute"))
            }
            if input.is_empty() {
                break;
            } else {
                input.parse::<syn::Token![,]>()?;
            }
        }
        Ok(result)
    }
}