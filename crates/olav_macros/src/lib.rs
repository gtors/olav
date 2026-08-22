//! Proc-macro entry point for `olav`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

mod ast;
mod codegen;
mod parse;

/// The `xml!` compile-time XML template macro.
///
/// See the `olav` crate documentation for the full syntax reference.
///
/// Note: splice expressions are scanned until the next template token, so
/// expressions containing top-level braces (e.g. closures with block bodies)
/// must be precomputed or wrapped in parentheses: `@(expr)`.
#[proc_macro]
pub fn xml(input: TokenStream) -> TokenStream {
    let input2 = TokenStream2::from(input);
    match expand(input2) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn expand(input: TokenStream2) -> syn::Result<TokenStream2> {
    let mut parser = parse::Parser::new(input);
    let nodes = parser.parse_body()?;
    Ok(codegen::generate(&nodes))
}
