use proc_macro2::{Literal, TokenStream};

use quote::quote;
use syn::{Data, DataStruct, DataEnum, Fields};
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
    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let body = derive_body(&s.fields)?;
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics structural_reflection::HasStructure for #ident #type_generics #where_clause {
            fn structure() -> structural_reflection::TypeStructure {
                structural_reflection::TypeStructure::CReprStruct {
                    body: #body
                }
            }
        }
    })
}

fn derive_c_enum(input: &syn::DeriveInput, s: &DataEnum) -> syn::Result<TokenStream> {
    let ident = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let variants = s.variants.iter().map(|variant| {
        let name = Literal::string(&variant.ident.to_string());
        let body = derive_body(&variant.fields)?;
        Ok::<TokenStream, syn::Error>(quote!(structural_reflection::TypeEnumVariant {
            name: #name,
            body: #body
        }))
    }).try_collect::<Vec<TokenStream>>()?;
    Ok(quote! {
        #[automatically_derived]
        impl #impl_generics structural_reflection::HasStructure for #ident #type_generics #where_clause {
            fn structure() -> structural_reflection::TypeStructure {
                structural_reflection::TypeStructure::CReprEnum {
                    variants: vec![#(#variants),*]
                }
            }
        }
    })
}

fn derive_body(fields: &Fields) -> syn::Result<TokenStream> {
    Ok(match fields {
        Fields::Unit => quote!(structural_reflection::TypeStructureBody::None),
        Fields::Unnamed(fields) => {
            let fields = fields.unnamed.iter().map(|field| {
                let ty = &field.ty;
                quote!(structural_reflection::RustType::of::<#ty>())
            });
            quote!(structural_reflection::TypeStructureBody::Tuple(vec![#( #fields ),*]))
        }
        Fields::Named(fields) => {
            let fields = fields.named.iter().map(|field| {
                let name = Literal::string(&variant.ident.to_string());
                let ty = &field.ty;
                quote!(structural_reflection::TypeStructureBodyField {
                    name: #name,
                    rust_type: structural_reflection::RustType::of::<#ty>(),
                })
            });
            quote!(structural_reflection::TypeStructureBody::Fields(vec![#( #fields ),*]))
        }
    })
}