use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Block, FnArg, GenericParam, Generics, ItemStruct, Pat, Result, Signature};

use crate::frontend::{Declaration, Function, ImplMember};

pub(crate) fn generate(declaration: Declaration) -> Result<TokenStream2> {
    Ok(match declaration.item {
        crate::frontend::Item::Function(function) => function_tokens(function),
        crate::frontend::Item::Struct(item) => structure(item),
        crate::frontend::Item::Impl(mut declaration) => {
            declaration.item.items = declaration
                .items
                .into_iter()
                .map(|item| match item {
                    ImplMember::Method(mut method) => {
                        method.item.block = body(&method.item.sig);
                        syn::ImplItem::Fn(method.item)
                    }
                    ImplMember::Other(item) => item,
                })
                .collect();
            let item = declaration.item;
            quote!(#item)
        }
    })
}

fn function_tokens(function: Function) -> TokenStream2 {
    let Function {
        attrs, vis, sig, ..
    } = function;
    let body = body(&sig);
    quote!(#(#attrs)* #vis #sig #body)
}

fn structure(item: ItemStruct) -> TokenStream2 {
    let ItemStruct {
        attrs,
        vis,
        ident,
        generics,
        ..
    } = item;
    let mut declaration_generics = generics.clone();
    declaration_generics.where_clause = None;
    let marker = marker_type(&generics);
    let where_clause = &generics.where_clause;
    quote! {
        #(#attrs)* #vis struct #ident #declaration_generics
            (::core::marker::PhantomData<#marker>) #where_clause;
    }
}

fn body(sig: &Signature) -> Block {
    let name = &sig.ident;
    let uses = sig.inputs.iter().map(|input| match input {
        FnArg::Receiver(receiver)
            if receiver.reference.is_none() && receiver.mutability.is_some() =>
        {
            quote!(let _ = &mut self;)
        }
        FnArg::Receiver(_) => quote!(let _ = self;),
        FnArg::Typed(argument) => {
            let Pat::Ident(pattern) = argument.pat.as_ref() else {
                unreachable!("parameter patterns were validated before generating a body")
            };
            let ident = &pattern.ident;
            if pattern.mutability.is_some() {
                quote!(let _ = &mut #ident;)
            } else {
                quote!(let _ = #ident;)
            }
        }
    });
    syn::parse_quote_spanned! {name.span()=>
        {
            #(#uses)*
            panic!(concat!(module_path!(), ".", stringify!(#name)))
        }
    }
}

fn marker_type(generics: &Generics) -> TokenStream2 {
    let markers = generics.params.iter().filter_map(|param| match param {
        GenericParam::Type(param) => {
            let ident = &param.ident;
            Some(quote!(#ident))
        }
        GenericParam::Lifetime(param) => {
            let lifetime = &param.lifetime;
            Some(quote!(&#lifetime ()))
        }
        GenericParam::Const(_) => None,
    });
    quote!((#(#markers,)*))
}
