use std::path::Path;

use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{format_ident, quote};
use syn::{Block, FnArg, GenericParam, Generics, ItemStruct, Pat, Result, Signature};

use crate::frontend::{Declaration, Function, ImplMember, SignatureAnalysis};

struct Generator {
    module: Option<syn::Ident>,
    facade: TokenStream2,
}

pub(crate) fn generate(declaration: Declaration) -> Result<TokenStream2> {
    let generator = Generator {
        module: source_module(declaration.source_span)?,
        facade: facade_path()?,
    };
    Ok(generator.generate(declaration.item))
}

impl Generator {
    fn generate(&self, item: crate::frontend::Item) -> TokenStream2 {
        match item {
            crate::frontend::Item::Function(function) => self.function(function),
            crate::frontend::Item::Struct(item) => self.structure(item),
            crate::frontend::Item::Impl(mut declaration) => {
                declaration.item.items = declaration
                    .items
                    .into_iter()
                    .map(|item| match item {
                        ImplMember::Method(mut method) => {
                            method.item.block = self.body(&method.item.sig, &method.analysis);
                            syn::ImplItem::Fn(method.item)
                        }
                        ImplMember::Other(item) => item,
                    })
                    .collect();
                let item = declaration.item;
                quote!(#item)
            }
        }
    }

    fn function(&self, function: Function) -> TokenStream2 {
        let Function {
            attrs,
            vis,
            sig,
            analysis,
        } = function;
        let body = self.body(&sig, &analysis);
        quote!(#(#attrs)* #vis #sig #body)
    }

    fn structure(&self, item: ItemStruct) -> TokenStream2 {
        let ItemStruct {
            attrs,
            vis,
            ident,
            generics,
            ..
        } = item;
        let facade = &self.facade;
        let backing = slop_path(self.module.as_ref());
        let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
        let mut declaration_generics = generics.clone();
        declaration_generics.where_clause = None;
        let (ref_generics, ref_lifetime) = with_lifetime(&generics, "__slop");
        let (ref_impl_generics, _, ref_where_clause) = ref_generics.split_for_impl();

        quote! {
            #(#attrs)* #vis struct #ident #declaration_generics (#backing::#ident #type_generics) #where_clause;

            impl #impl_generics #facade::FromSlop<#backing::#ident #type_generics>
                for #ident #type_generics #where_clause
            {
                fn from_slop(value: #backing::#ident #type_generics) -> Self {
                    Self(value)
                }
            }

            impl #impl_generics #facade::FromSlop<#ident #type_generics>
                for #backing::#ident #type_generics #where_clause
            {
                fn from_slop(value: #ident #type_generics) -> Self {
                    value.0
                }
            }

            impl #ref_impl_generics #facade::FromSlop<&#ref_lifetime #ident #type_generics>
                for &#ref_lifetime #backing::#ident #type_generics #ref_where_clause
            {
                fn from_slop(value: &#ref_lifetime #ident #type_generics) -> Self {
                    &value.0
                }
            }

            impl #ref_impl_generics #facade::FromSlop<&#ref_lifetime mut #ident #type_generics>
                for &#ref_lifetime mut #backing::#ident #type_generics #ref_where_clause
            {
                fn from_slop(value: &#ref_lifetime mut #ident #type_generics) -> Self {
                    &mut value.0
                }
            }
        }
    }

    fn body(&self, sig: &Signature, analysis: &SignatureAnalysis) -> Block {
        let facade = &self.facade;
        let name = &sig.ident;
        let args = sig
            .inputs
            .iter()
            .zip(&analysis.arguments_containing_self)
            .map(|(input, contains_self)| match input {
                FnArg::Receiver(receiver) => {
                    match (&receiver.reference, receiver.mutability.is_some()) {
                        (None, _) => quote!(self.0),
                        (Some(_), false) => quote!(&self.0),
                        (Some(_), true) => quote!(&mut self.0),
                    }
                }
                FnArg::Typed(argument) => {
                    let Pat::Ident(pattern) = argument.pat.as_ref() else {
                        unreachable!("parameter patterns were validated before generating a body")
                    };
                    let ident = &pattern.ident;
                    if *contains_self {
                        quote!(#facade::IntoSlop::into_slop(#ident))
                    } else {
                        quote!(#ident)
                    }
                }
            });
        let target = slop_path(self.module.as_ref());
        let call = quote!(#target::#name(#(#args),*));
        let call = if sig.unsafety.is_some() {
            quote!(unsafe { #call })
        } else {
            call
        };
        let call = if sig.asyncness.is_some() {
            quote!((#call).await)
        } else {
            call
        };
        let expression = if analysis.return_contains_self {
            quote!(#facade::FromSlop::from_slop(#call))
        } else {
            call
        };
        syn::parse_quote!({ #expression })
    }
}

fn source_module(span: proc_macro::Span) -> Result<Option<syn::Ident>> {
    let file = span.file();
    let file = Path::new(&file);
    let file_name = file
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| syn::Error::new(Span::from(span), "`slop` could not derive a filename"))?;

    let stem = match file_name {
        "lib.rs" | "main.rs" => return Ok(None),
        "mod.rs" => file
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str()),
        _ => file.file_stem().and_then(|name| name.to_str()),
    }
    .ok_or_else(|| syn::Error::new(Span::from(span), "`slop` could not derive a module name"))?;

    let ident = syn::parse_str::<syn::Ident>(stem)
        .or_else(|_| syn::parse_str::<syn::Ident>(&format!("r#{stem}")))
        .map_err(|_| {
            syn::Error::new(
                Span::from(span),
                format!("`{stem}` cannot be used as a `zz_slop` module name"),
            )
        })?;
    Ok(Some(ident))
}

fn slop_path(module: Option<&syn::Ident>) -> TokenStream2 {
    match module {
        Some(module) => quote!(crate::zz_slop::#module),
        None => quote!(crate::zz_slop),
    }
}

fn facade_path() -> Result<TokenStream2> {
    match crate_name("compromise").map_err(|error| {
        syn::Error::new(
            Span::call_site(),
            format!("`compromise` facade crate is required: {error}"),
        )
    })? {
        FoundCrate::Itself => Ok(quote!(::compromise)),
        FoundCrate::Name(name) => {
            let name = format_ident!("{}", name.replace('-', "_"));
            Ok(quote!(::#name))
        }
    }
}

fn with_lifetime(generics: &Generics, preferred: &str) -> (Generics, syn::Lifetime) {
    let mut result = generics.clone();
    let existing = result.params.iter().filter_map(|param| match param {
        GenericParam::Lifetime(param) => Some(param.lifetime.ident.to_string()),
        _ => None,
    });
    let mut name = preferred.to_owned();
    let names: Vec<_> = existing.collect();
    while names.iter().any(|existing| existing == &name) {
        name.push('_');
    }
    let lifetime = syn::Lifetime::new(&format!("'{name}"), Span::call_site());
    result.params.insert(0, syn::parse_quote!(#lifetime));
    (result, lifetime)
}
