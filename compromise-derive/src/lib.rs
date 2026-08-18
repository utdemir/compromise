use proc_macro::TokenStream;
mod frontend;

#[cfg_attr(feature = "panicking", path = "panicking.rs")]
#[cfg_attr(not(feature = "panicking"), path = "delegating.rs")]
mod backend;

#[proc_macro_attribute]
pub fn slop(args: TokenStream, input: TokenStream) -> TokenStream {
    if !args.is_empty() {
        return syn::Error::new(proc_macro2::Span::call_site(), "`slop` takes no arguments")
            .into_compile_error()
            .into();
    }

    match frontend::parse(input).and_then(backend::generate) {
        Ok(output) => output.into(),
        Err(error) => error.into_compile_error().into(),
    }
}
