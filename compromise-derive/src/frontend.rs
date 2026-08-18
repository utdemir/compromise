use proc_macro2::{Delimiter, Group, Span, TokenStream as TokenStream2, TokenTree};
use quote::quote;
use syn::parse::discouraged::Speculative;
use syn::parse::{Parse, ParseStream};
use syn::visit::Visit;
use syn::{
    Attribute, FnArg, ImplItem, ImplItemFn, ItemFn, ItemImpl, ItemStruct, Pat, Result, ReturnType,
    Signature, Token, Type, Visibility,
};

pub(crate) struct Declaration {
    #[allow(dead_code)] // Used by the delegating backend selected in the other feature mode.
    pub(crate) source_span: proc_macro::Span,
    pub(crate) item: Item,
}

pub(crate) enum Item {
    Function(Function),
    Struct(ItemStruct),
    Impl(Impl),
}

pub(crate) struct Function {
    pub(crate) attrs: Vec<Attribute>,
    pub(crate) vis: Visibility,
    pub(crate) sig: Signature,
    #[allow(dead_code)] // Used by the delegating backend selected in the other feature mode.
    pub(crate) analysis: SignatureAnalysis,
}

pub(crate) struct Impl {
    pub(crate) item: ItemImpl,
    pub(crate) items: Vec<ImplMember>,
}

pub(crate) enum ImplMember {
    Method(Method),
    Other(ImplItem),
}

pub(crate) struct Method {
    pub(crate) item: ImplItemFn,
    #[allow(dead_code)] // Used by the delegating backend selected in the other feature mode.
    pub(crate) analysis: SignatureAnalysis,
}

pub(crate) fn parse(input: proc_macro::TokenStream) -> Result<Declaration> {
    let source_span = input
        .clone()
        .into_iter()
        .next()
        .map_or_else(proc_macro::Span::call_site, |token| token.span());
    let item = parse_item(TokenStream2::from(input))?;
    Ok(Declaration { source_span, item })
}

fn parse_item(input: TokenStream2) -> Result<Item> {
    if let Ok(function) = syn::parse2::<SlopDeclaration>(input.clone()) {
        return function.validate(false).map(Item::Function);
    }

    if let Ok(item_fn) = syn::parse2::<ItemFn>(input.clone()) {
        return Err(syn::Error::new_spanned(
            item_fn.block,
            "`slop` functions must be declared without a body, using `;`",
        ));
    }

    if let Ok(item_struct) = syn::parse2::<ItemStruct>(input.clone()) {
        if !matches!(item_struct.fields, syn::Fields::Unit) {
            return Err(syn::Error::new_spanned(
                item_struct.fields,
                "slop: structs must be unit-style forward declarations, using `;`",
            ));
        }
        return Ok(Item::Struct(item_struct));
    }

    let unsupported_span = input
        .clone()
        .into_iter()
        .last()
        .map_or_else(Span::call_site, |tree| tree.span());
    parse_impl(input).map(Item::Impl).map_err(|error| {
        if error.to_string().starts_with("slop:") {
            error
        } else {
            syn::Error::new(
                unsupported_span,
                "`slop` can only annotate a bodyless function, a unit struct, or an impl",
            )
        }
    })
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
    fn validate(self, method: bool) -> Result<Function> {
        let Self { attrs, vis, sig } = self;
        let analysis = analyze_signature(&sig, method)?;
        Ok(Function {
            attrs,
            vis,
            sig,
            analysis,
        })
    }

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

fn parse_impl(input: TokenStream2) -> Result<Impl> {
    let (header, body) = split_impl(input)?;
    let item_impl: ItemImpl = syn::parse2(quote!(#header {}))?;
    let body: ImplBody = syn::parse2(body.stream())?;
    let items = body
        .items
        .into_iter()
        .map(|item| match item {
            ImplItem::Fn(method) => validate_method(method).map(ImplMember::Method),
            item => Ok(ImplMember::Other(item)),
        })
        .collect::<Result<_>>()?;
    Ok(Impl {
        item: item_impl,
        items,
    })
}

fn split_impl(input: TokenStream2) -> Result<(TokenStream2, Group)> {
    let mut trees: Vec<TokenTree> = input.into_iter().collect();
    let Some(TokenTree::Group(body)) = trees.pop() else {
        return Err(syn::Error::new(Span::call_site(), "expected an impl block"));
    };
    if body.delimiter() != Delimiter::Brace {
        return Err(syn::Error::new(body.span(), "expected an impl block"));
    }
    Ok((trees.into_iter().collect(), body))
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

fn validate_method(method: ImplItemFn) -> Result<Method> {
    let analysis = analyze_signature(&method.sig, true)?;
    Ok(Method {
        item: method,
        analysis,
    })
}

#[allow(dead_code)] // Its fields are consumed only by the delegating backend.
pub(crate) struct SignatureAnalysis {
    pub(crate) return_contains_self: bool,
    pub(crate) arguments_containing_self: Vec<bool>,
}

fn analyze_signature(sig: &Signature, method: bool) -> Result<SignatureAnalysis> {
    if sig.variadic.is_some() {
        return Err(syn::Error::new_spanned(
            &sig.variadic,
            "slop: variadic functions are not supported",
        ));
    }

    for input in &sig.inputs {
        match input {
            FnArg::Receiver(receiver) => {
                if !method || receiver.colon_token.is_some() {
                    return Err(syn::Error::new_spanned(
                        receiver,
                        "slop: typed receivers are not supported",
                    ));
                }
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
    let arguments_containing_self: Vec<_> = sig
        .inputs
        .iter()
        .map(|input| match input {
            FnArg::Typed(argument) => self_usage(&argument.ty).contains,
            FnArg::Receiver(_) => false,
        })
        .collect();
    if sig.constness.is_some()
        && (return_contains_self || arguments_containing_self.iter().any(|contains| *contains))
    {
        return Err(syn::Error::new_spanned(
            sig,
            "slop: const functions cannot use `Self` conversion traits",
        ));
    }

    Ok(SignatureAnalysis {
        return_contains_self,
        arguments_containing_self,
    })
}

#[derive(Clone, Copy, Default)]
struct SelfUsage {
    contains: bool,
    borrowed: bool,
}

fn self_usage(ty: &Type) -> SelfUsage {
    let mut visitor = SelfUsageVisitor::default();
    visitor.visit_type(ty);
    visitor.usage
}

#[derive(Default)]
struct SelfUsageVisitor {
    usage: SelfUsage,
    reference_depth: usize,
}

impl<'ast> Visit<'ast> for SelfUsageVisitor {
    fn visit_type_reference(&mut self, reference: &'ast syn::TypeReference) {
        self.reference_depth += 1;
        syn::visit::visit_type_reference(self, reference);
        self.reference_depth -= 1;
    }

    fn visit_type_path(&mut self, path: &'ast syn::TypePath) {
        if path.qself.is_none()
            && path.path.segments.len() == 1
            && path.path.segments[0].ident == "Self"
        {
            self.usage.contains = true;
            self.usage.borrowed |= self.reference_depth != 0;
        } else {
            syn::visit::visit_type_path(self, path);
        }
    }
}
