use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{GenericParam, Generics, parse_quote, TraitBound, TypeParamBound, WhereClause};

pub fn common_derive(generics: &Generics) -> (Vec<&Ident>, Option<WhereClause>) {
    let type_params = generics.type_params().map(|type_param| &type_param.ident).collect::<Vec<_>>();
    let mut where_clause = generics.where_clause.clone();
    if !type_params.is_empty() {
        if where_clause.is_none() {
            where_clause = Some(WhereClause {
                where_token: Default::default(),
                predicates: Default::default()
            })
        }
        let where_clause = where_clause.as_mut().unwrap();
        for type_param in &type_params {
            where_clause.predicates.push(parse_quote!(#type_param::StaticId: Sized));
        }
    }
    (type_params, where_clause)
}

pub fn recursive_impl_generics(generics: &Generics, trait_bound: &TraitBound) -> TokenStream {
    let mut params = generics.params.clone();
    for param in params.iter_mut() {
        if let GenericParam::Type(type_param) = param {
            type_param.bounds.push(TypeParamBound::Trait(trait_bound.clone()));
        }
    }
    if params.is_empty() {
        quote!()
    } else {
        quote!(<#params>)
    }
}