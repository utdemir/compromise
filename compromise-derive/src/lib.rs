#[cfg(not(feature = "panicking"))]
use std::path::Path;

use proc_macro::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use proc_macro2::{Delimiter, Group, Span, TokenStream as TokenStream2, TokenTree};
use quote::{format_ident, quote};
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::{
    Attribute, Block, FnArg, GenericParam, Generics, ImplItem, ImplItemFn, ItemFn, ItemImpl,
    ItemStruct, Pat, PathArguments, Result, ReturnType, Signature, Token, Type, TypePath,
    Visibility,
};

/// Implements forward declarations by dispatching to matching functions and types in
/// `crate::zz_slop`.
#[proc_macro_attribute]
pub fn slop(args: TokenStream, input: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return syn::Error::new(proc_macro2::Span::call_site(), "`slop` takes no arguments")
            .into_compile_error()
            .into();
    }

    let input = TokenStream2::from(input);
    #[cfg(feature = "panicking")]
    let result = expand(input, ExpansionMode::Panicking);
    #[cfg(not(feature = "panicking"))]
    let result = source_module(&input.clone().into())
        .and_then(|module| expand(input, ExpansionMode::Delegating(module)));

    match result {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}

#[allow(dead_code)]
enum ExpansionMode {
    Delegating(Option<syn::Ident>),
    Panicking,
}

#[cfg(not(feature = "panicking"))]
fn source_module(input: &TokenStream) -> Result<Option<syn::Ident>> {
    let span = input
        .clone()
        .into_iter()
        .next()
        .map(|token| token.span())
        .unwrap_or_else(proc_macro::Span::call_site);
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

fn expand(input: TokenStream2, mode: ExpansionMode) -> Result<TokenStream2> {
    if let Ok(function) = syn::parse2::<SlopDeclaration>(input.clone()) {
        return function.expand(&mode);
    }

    if let Ok(item_fn) = syn::parse2::<ItemFn>(input.clone()) {
        return Err(syn::Error::new_spanned(
            item_fn.block,
            "`slop` functions must be declared without a body, using `;`",
        ));
    }

    if let Ok(item_struct) = syn::parse2::<ItemStruct>(input.clone()) {
        return expand_struct(item_struct, &mode);
    }

    expand_impl(input, &mode).map_err(|error| {
        if error.to_string().starts_with("slop:") {
            error
        } else {
            syn::Error::new(
                error.span(),
                "`slop` can only annotate a bodyless function, a unit struct, or an impl",
            )
        }
    })
}

fn slop_path(module: &Option<syn::Ident>) -> TokenStream2 {
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

struct SlopDeclaration {
    attrs: Vec<Attribute>,
    vis: Visibility,
    sig: Signature,
}

impl Parse for SlopDeclaration {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let vis = input.parse()?;
        let sig = input.parse()?;
        input.parse::<Token![;]>()?;
        Ok(Self { attrs, vis, sig })
    }
}

impl SlopDeclaration {
    fn expand(self, mode: &ExpansionMode) -> Result<TokenStream2> {
        let Self { attrs, vis, sig } = self;
        let body = dispatch_body(&sig, mode, false)?;
        Ok(quote! {
            #(#attrs)* #vis #sig #body
        })
    }
}

fn expand_struct(item: ItemStruct, mode: &ExpansionMode) -> Result<TokenStream2> {
    if !matches!(item.fields, syn::Fields::Unit) {
        return Err(syn::Error::new_spanned(
            item.fields,
            "slop: structs must be unit-style forward declarations, using `;`",
        ));
    }

    let ItemStruct {
        attrs,
        vis,
        ident,
        generics,
        ..
    } = item;
    if matches!(mode, ExpansionMode::Panicking) {
        let mut declaration_generics = generics.clone();
        declaration_generics.where_clause = None;
        let marker = marker_type(&generics);
        let where_clause = &generics.where_clause;
        return Ok(quote! {
            #(#attrs)* #vis struct #ident #declaration_generics
                (::core::marker::PhantomData<#marker>) #where_clause;
        });
    }

    let ExpansionMode::Delegating(module) = mode else {
        unreachable!()
    };
    let facade = facade_path()?;
    let backing = slop_path(module);
    let (impl_generics, type_generics, where_clause) = generics.split_for_impl();
    let mut declaration_generics = generics.clone();
    declaration_generics.where_clause = None;
    let (ref_generics, ref_lifetime) = with_lifetime(&generics, "__slop");
    let (mut_ref_generics, mut_ref_lifetime) = with_lifetime(&generics, "__slop_mut");
    let (ref_impl_generics, _, ref_where_clause) = ref_generics.split_for_impl();
    let (mut_ref_impl_generics, _, mut_ref_where_clause) = mut_ref_generics.split_for_impl();

    Ok(quote! {
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

        impl #mut_ref_impl_generics #facade::FromSlop<&#mut_ref_lifetime mut #ident #type_generics>
            for &#mut_ref_lifetime mut #backing::#ident #type_generics #mut_ref_where_clause
        {
            fn from_slop(value: &#mut_ref_lifetime mut #ident #type_generics) -> Self {
                &mut value.0
            }
        }
    })
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
    let markers: Vec<_> = markers.collect();
    if markers.is_empty() {
        quote!(())
    } else {
        quote!((#(#markers,)*))
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

fn expand_impl(input: TokenStream2, mode: &ExpansionMode) -> Result<TokenStream2> {
    let (header, body) = split_impl(input)?;
    let empty_impl = quote!(#header {});
    let mut item_impl: ItemImpl = syn::parse2(empty_impl)?;
    let body: ImplBody = syn::parse2(body.stream())?;
    item_impl.items = body
        .items
        .into_iter()
        .map(|item| match item {
            ImplItem::Fn(method) => expand_method(method, mode).map(ImplItem::Fn),
            item => Ok(item),
        })
        .collect::<Result<_>>()?;
    Ok(quote!(#item_impl))
}

fn split_impl(input: TokenStream2) -> Result<(TokenStream2, Group)> {
    let mut trees: Vec<TokenTree> = input.into_iter().collect();
    let Some(TokenTree::Group(body)) = trees.pop() else {
        return Err(syn::Error::new(Span::call_site(), "expected an impl block"));
    };
    if body.delimiter() != Delimiter::Brace || !trees.is_empty() && !matches_impl_header(&trees) {
        return Err(syn::Error::new(body.span(), "expected an impl block"));
    }
    Ok((trees.into_iter().collect(), body))
}

fn matches_impl_header(trees: &[TokenTree]) -> bool {
    trees
        .iter()
        .any(|tree| matches!(tree, TokenTree::Ident(ident) if ident == "impl"))
}

struct ImplBody {
    items: Vec<ImplItem>,
}

impl Parse for ImplBody {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let mut items = Vec::new();
        while !input.is_empty() {
            let fork = input.fork();
            if let Ok(method) = fork.parse::<SlopDeclaration>() {
                input.advance_to(&fork);
                items.push(ImplItem::Fn(method.into_impl_item()));
                continue;
            }

            let item: ImplItem = input.parse()?;
            if let ImplItem::Fn(method) = &item {
                return Err(syn::Error::new_spanned(
                    &method.block,
                    "slop: methods in a `slop` impl must be declared without a body, using `;`",
                ));
            }
            items.push(item);
        }
        Ok(Self { items })
    }
}

impl SlopDeclaration {
    fn into_impl_item(self) -> ImplItemFn {
        let Self { attrs, vis, sig } = self;
        ImplItemFn {
            attrs,
            vis,
            defaultness: None,
            sig,
            block: syn::parse_quote!({}),
        }
    }
}

fn expand_method(mut method: ImplItemFn, mode: &ExpansionMode) -> Result<ImplItemFn> {
    method.block = dispatch_body(&method.sig, mode, true)?;
    Ok(method)
}

fn dispatch_body(sig: &Signature, mode: &ExpansionMode, method: bool) -> Result<Block> {
    if sig.variadic.is_some() {
        return Err(syn::Error::new_spanned(
            &sig.variadic,
            "slop: variadic functions are not supported",
        ));
    }

    let facade = if matches!(mode, ExpansionMode::Delegating(_)) {
        Some(facade_path()?)
    } else {
        None
    };
    let name = &sig.ident;
    let mut args = Vec::new();

    for input in &sig.inputs {
        match input {
            FnArg::Receiver(receiver) => {
                if !method || receiver.colon_token.is_some() {
                    return Err(syn::Error::new_spanned(
                        receiver,
                        "slop: typed receivers are not supported",
                    ));
                }
                let arg = match (&receiver.reference, receiver.mutability.is_some()) {
                    (None, false) => quote!(self.0),
                    (None, true) => quote!(self.0),
                    (Some(_), false) => quote!(&self.0),
                    (Some(_), true) => quote!(&mut self.0),
                };
                args.push(arg);
            }
            FnArg::Typed(argument) => {
                let Pat::Ident(pattern) = argument.pat.as_ref() else {
                    return Err(syn::Error::new_spanned(
                        &argument.pat,
                        "slop: parameters must use identifier patterns",
                    ));
                };
                if pattern.by_ref.is_some() || pattern.subpat.is_some() {
                    return Err(syn::Error::new_spanned(
                        pattern,
                        "slop: parameters must use plain identifier patterns",
                    ));
                }
                let ident = &pattern.ident;
                if self_usage(&argument.ty).contains {
                    let facade = facade.as_ref();
                    args.push(quote!(#facade::IntoSlop::into_slop(#ident)));
                } else {
                    args.push(quote!(#ident));
                }
            }
        }
    }

    let return_contains_self = match &sig.output {
        ReturnType::Default => false,
        ReturnType::Type(_, ty) => {
            let usage = self_usage(ty);
            if usage.borrowed {
                return Err(syn::Error::new_spanned(
                    ty,
                    "slop: return types containing borrowed `Self` are not supported",
                ));
            }
            usage.contains
        }
    };
    let argument_conversion = sig.inputs.iter().any(|input| match input {
        FnArg::Typed(argument) => self_usage(&argument.ty).contains,
        FnArg::Receiver(_) => false,
    });
    if sig.constness.is_some() && (return_contains_self || argument_conversion) {
        return Err(syn::Error::new_spanned(
            sig,
            "slop: const functions cannot use `Self` conversion traits",
        ));
    }

    if matches!(mode, ExpansionMode::Panicking) {
        return Ok(panic_body(sig));
    }

    let ExpansionMode::Delegating(module) = mode else {
        unreachable!()
    };
    let facade = facade.as_ref().expect("delegating mode has a facade");
    let target = slop_path(module);
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
    let expression = if return_contains_self {
        quote!(#facade::FromSlop::from_slop(#call))
    } else {
        call
    };
    syn::parse2(quote!({ #expression }))
}

fn panic_body(sig: &Signature) -> Block {
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

#[derive(Clone, Copy, Default)]
struct SelfUsage {
    contains: bool,
    borrowed: bool,
}

impl SelfUsage {
    fn merge(self, other: Self) -> Self {
        Self {
            contains: self.contains || other.contains,
            borrowed: self.borrowed || other.borrowed,
        }
    }
}

fn self_usage(ty: &Type) -> SelfUsage {
    match ty {
        Type::Path(path) => path_self_usage(path),
        Type::Reference(reference) => {
            let usage = self_usage(&reference.elem);
            SelfUsage {
                contains: usage.contains,
                borrowed: usage.contains,
            }
        }
        Type::Array(array) => self_usage(&array.elem),
        Type::Slice(slice) => self_usage(&slice.elem),
        Type::Tuple(tuple) => tuple
            .elems
            .iter()
            .map(self_usage)
            .fold(SelfUsage::default(), SelfUsage::merge),
        Type::Paren(paren) => self_usage(&paren.elem),
        Type::Group(group) => self_usage(&group.elem),
        Type::Ptr(pointer) => self_usage(&pointer.elem),
        _ => SelfUsage::default(),
    }
}

fn path_self_usage(path: &TypePath) -> SelfUsage {
    if path.qself.is_none()
        && path.path.segments.len() == 1
        && path.path.segments[0].ident == "Self"
    {
        return SelfUsage {
            contains: true,
            borrowed: false,
        };
    }
    path.path
        .segments
        .iter()
        .map(|segment| match &segment.arguments {
            PathArguments::AngleBracketed(arguments) => {
                arguments
                    .args
                    .iter()
                    .fold(SelfUsage::default(), |usage, argument| {
                        let argument_usage = match argument {
                            syn::GenericArgument::Type(ty) => self_usage(ty),
                            syn::GenericArgument::AssocType(assoc) => self_usage(&assoc.ty),
                            _ => SelfUsage::default(),
                        };
                        usage.merge(argument_usage)
                    })
            }
            PathArguments::Parenthesized(arguments) => {
                let inputs = arguments
                    .inputs
                    .iter()
                    .map(self_usage)
                    .fold(SelfUsage::default(), SelfUsage::merge);
                match &arguments.output {
                    ReturnType::Default => inputs,
                    ReturnType::Type(_, ty) => inputs.merge(self_usage(ty)),
                }
            }
            PathArguments::None => SelfUsage::default(),
        })
        .fold(SelfUsage::default(), SelfUsage::merge)
}
