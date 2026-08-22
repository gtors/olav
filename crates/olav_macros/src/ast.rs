//! AST types produced by the parser, consumed by codegen.

use proc_macro2::{Span, TokenStream};

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub value: T,
    /// Retained for future diagnostics; not read by codegen yet.
    #[allow(dead_code)]
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SpliceMod {
    Escaped,
    Raw,
}

#[derive(Debug, Clone)]
pub enum Attr {
    Lit {
        name: Spanned<String>,
        value: Spanned<String>,
    },
    Expr {
        name: Spanned<String>,
        value: TokenStream,
    },
    /// Conditional attribute. With a value: rendered only when `cond` holds.
    /// Without one (bare `attr? cond` / `attr?=`): emits `attr="true"`.
    CondExpr {
        name: Spanned<String>,
        cond: TokenStream,
        value: TokenStream,
    },
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pat: TokenStream,
    pub body: Vec<Node>,
    /// Retained for future diagnostics; not read by codegen yet.
    #[allow(dead_code)]
    pub arrow_span: Span,
}

#[derive(Debug, Clone)]
pub enum Node {
    Element {
        name: Spanned<String>,
        attrs: Vec<Attr>,
        body: Vec<Node>,
        /// Retained for future diagnostics; not read by codegen yet.
        #[allow(dead_code)]
        span: Span,
    },
    Text(Spanned<String>),
    Splice {
        expr: TokenStream,
        modifier: SpliceMod,
        /// Retained for future diagnostics; not read by codegen yet.
        #[allow(dead_code)]
        span: Span,
    },
    If {
        cond: TokenStream,
        then: Vec<Node>,
        else_: Option<Vec<Node>>,
        /// Retained for future diagnostics; not read by codegen yet.
        #[allow(dead_code)]
        span: Span,
    },
    For {
        pat: TokenStream,
        expr: TokenStream,
        body: Vec<Node>,
        /// Retained for future diagnostics; not read by codegen yet.
        #[allow(dead_code)]
        span: Span,
    },
    While {
        cond: TokenStream,
        body: Vec<Node>,
        /// Retained for future diagnostics; not read by codegen yet.
        #[allow(dead_code)]
        span: Span,
    },
    Match {
        expr: TokenStream,
        arms: Vec<MatchArm>,
        /// Retained for future diagnostics; not read by codegen yet.
        #[allow(dead_code)]
        span: Span,
    },
    Comment {
        text: Spanned<String>,
        /// Retained for future diagnostics; not read by codegen yet.
        #[allow(dead_code)]
        span: Span,
    },
    Cdata {
        body: Vec<Node>,
        /// Retained for future diagnostics; not read by codegen yet.
        #[allow(dead_code)]
        span: Span,
    },
    Pi {
        name: Spanned<String>,
        attrs: Vec<Attr>,
        body: Vec<Node>,
        /// Retained for future diagnostics; not read by codegen yet.
        #[allow(dead_code)]
        span: Span,
    },
    Doctype {
        /// The full DOCTYPE specification (root name plus any SYSTEM/PUBLIC
        /// clauses), already joined into its final textual form.
        spec: String,
        /// Retained for future diagnostics; not read by codegen yet.
        #[allow(dead_code)]
        span: Span,
    },
}
