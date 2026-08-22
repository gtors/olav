//! Codegen: Vec\<Node\> → TokenStream (Rust code that builds a Markup)

use proc_macro2::{Ident, Span, TokenStream};
use quote::{TokenStreamExt, quote};

use crate::ast::{Attr, Node, Spanned, SpliceMod};

/// Generation context: which output variable to write to, and how literal
/// text is emitted.
#[derive(Clone, Copy)]
struct Ctx {
    out: &'static str,
    mode: TextMode,
}

impl Ctx {
    fn out(&self) -> Ident {
        Ident::new(self.out, Span::call_site())
    }
}

/// How literal text nodes are emitted.
#[derive(Clone, Copy)]
enum TextMode {
    /// Normal content: escape `& < >`.
    Escape,
    /// Inside `@cdata`: emit verbatim (CDATA sections do not interpret
    /// entity references, so escaping would corrupt the value).
    Raw,
}

const TOP: Ctx = Ctx {
    out: "__olav_out",
    mode: TextMode::Escape,
};

const CDATA: Ctx = Ctx {
    out: "__olav_cdata_buf",
    mode: TextMode::Raw,
};

pub fn generate(nodes: &[Node]) -> TokenStream {
    let body = generate_body_in(nodes, TOP);
    let cap = estimate_len(nodes);
    quote! {
        {
            let mut __olav_buf = ::olav::Markup::with_capacity(#cap);
            let __olav_out = &mut __olav_buf;
            #body
            __olav_buf
        }
    }
}

/// Static lower-bound estimate of the output size, derived from the lengths
/// of literal text/tag names in the AST. Dynamic parts contribute nothing;
/// the estimate just avoids repeated reallocation for the known part.
fn estimate_len(nodes: &[Node]) -> usize {
    nodes
        .iter()
        .map(|node| {
            match node {
                Node::Element {
                    name, attrs, body, ..
                } => {
                    // `<name>` + attrs + `</name>` (or `/>`)
                    let tag = if body.is_empty() {
                        3 + name.value.len()
                    } else {
                        5 + 2 * name.value.len()
                    };
                    tag.saturating_add(estimate_attrs(attrs))
                        .saturating_add(estimate_len(body))
                }
                Node::Text(s) => s.value.len(),
                Node::Comment { text, .. } => text.value.len().saturating_add(7),
                Node::Cdata { body, .. } => estimate_len(body).saturating_add("<![CDATA[]]>".len()),
                Node::Pi {
                    name, attrs, body, ..
                } => ("<?".len() + "?>".len())
                    .saturating_add(name.value.len())
                    .saturating_add(if body.is_empty() { 0 } else { 1 })
                    .saturating_add(estimate_attrs(attrs))
                    .saturating_add(estimate_len(body)),
                Node::Doctype { spec, .. } => spec.len().saturating_add("<!DOCTYPE >".len()),
                Node::If { then, else_, .. } => {
                    estimate_len(then).saturating_add(else_.as_deref().map_or(0, estimate_len))
                }
                Node::For { body, .. } | Node::While { body, .. } => estimate_len(body),
                Node::Match { arms, .. } => arms
                    .iter()
                    .map(|arm| estimate_len(&arm.body))
                    .fold(0usize, usize::saturating_add),
                // Splices have statically unknown length.
                Node::Splice { .. } => 0,
            }
        })
        .fold(0usize, usize::saturating_add)
}

fn estimate_attrs(attrs: &[Attr]) -> usize {
    attrs
        .iter()
        .map(|attr| match attr {
            Attr::Lit { name, value } => 3usize
                .saturating_add(name.value.len())
                .saturating_add(value.value.len()),
            Attr::Expr { name, .. } | Attr::CondExpr { name, .. } => {
                3usize.saturating_add(name.value.len())
            }
        })
        .fold(0usize, usize::saturating_add)
}

fn generate_body_in(nodes: &[Node], ctx: Ctx) -> TokenStream {
    let mut ts = TokenStream::new();
    for node in nodes {
        ts.extend(generate_node(node, ctx));
    }
    ts
}

fn generate_node(node: &Node, ctx: Ctx) -> TokenStream {
    match node {
        Node::Element {
            name, attrs, body, ..
        } => generate_element(name, attrs, body, ctx),
        Node::Text(s) => {
            let lit = s.value.clone();
            let out = ctx.out();
            match ctx.mode {
                TextMode::Escape => quote! { #out.push_text(#lit); },
                TextMode::Raw => quote! { #out.push_str(#lit); },
            }
        }
        Node::Splice { expr, modifier, .. } => generate_splice(expr, *modifier, ctx),
        Node::If {
            cond, then, else_, ..
        } => generate_if(cond, then, else_.as_deref(), ctx),
        Node::For {
            pat, expr, body, ..
        } => generate_for(pat, expr, body, ctx),
        Node::While { cond, body, .. } => generate_while(cond, body, ctx),
        Node::Match { expr, arms, .. } => generate_match(expr, arms, ctx),
        Node::Comment { text, .. } => {
            let t = text.value.clone();
            let out = ctx.out();
            quote! { #out.push_str("<!--"); #out.push_str(#t); #out.push_str("-->"); }
        }
        Node::Cdata { body, .. } => generate_cdata(body),
        Node::Pi {
            name, attrs, body, ..
        } => generate_pi(name, attrs, body, ctx),
        Node::Doctype { spec, .. } => generate_doctype(spec, ctx),
    }
}

fn generate_element(
    name: &Spanned<String>,
    attrs: &[Attr],
    body: &[Node],
    ctx: Ctx,
) -> TokenStream {
    let name_lit = name.value.clone();
    let attrs_ts = generate_attrs(attrs, ctx);
    let body_ts = generate_body_in(body, ctx);
    let out = ctx.out();

    if body.is_empty() {
        // self-closing
        quote! {
            #out.push_str("<");
            #out.push_str(#name_lit);
            #attrs_ts
            #out.push_str("/>");
        }
    } else {
        quote! {
            #out.push_str("<");
            #out.push_str(#name_lit);
            #attrs_ts
            #out.push_str(">");
            #body_ts
            #out.push_str("</");
            #out.push_str(#name_lit);
            #out.push_str(">");
        }
    }
}

fn generate_attrs(attrs: &[Attr], ctx: Ctx) -> TokenStream {
    let mut ts = TokenStream::new();
    for attr in attrs {
        ts.extend(generate_attr(attr, ctx));
    }
    ts
}

fn generate_attr(attr: &Attr, ctx: Ctx) -> TokenStream {
    let out = ctx.out();
    match attr {
        Attr::Lit { name, value } => {
            let n = name.value.clone();
            let v = value.value.clone();
            quote! { #out.push_attr(#n, #v); }
        }
        Attr::Expr { name, value } => {
            let n = name.value.clone();
            quote! {
                {
                    let mut __olav_tmp = ::olav::Markup::new();
                    ::olav::Render::render_to(&(#value), &mut __olav_tmp);
                    #out.push_attr(#n, __olav_tmp.as_str());
                }
            }
        }
        Attr::CondExpr { name, cond, value } => {
            let n = name.value.clone();
            if value.is_empty() {
                quote! {
                    if #cond {
                        #out.push_attr(#n, "true");
                    }
                }
            } else {
                quote! {
                    if #cond {
                        let mut __olav_tmp = ::olav::Markup::new();
                        ::olav::Render::render_to(&(#value), &mut __olav_tmp);
                        #out.push_attr(#n, __olav_tmp.as_str());
                    }
                }
            }
        }
    }
}

fn generate_splice(expr: &TokenStream, modifier: SpliceMod, ctx: Ctx) -> TokenStream {
    let out = ctx.out();
    match modifier {
        SpliceMod::Escaped => quote! {
            ::olav::Render::render_to(&(#expr), #out);
        },
        SpliceMod::Raw => quote! {
            {
                let __olav_s = (#expr).as_ref();
                #out.push_str(__olav_s);
            }
        },
    }
}

fn generate_if(cond: &TokenStream, then: &[Node], else_: Option<&[Node]>, ctx: Ctx) -> TokenStream {
    let then_ts = generate_body_in(then, ctx);
    let else_ts = match else_ {
        Some(nodes) => {
            let ts = generate_body_in(nodes, ctx);
            quote! { else { #ts } }
        }
        None => TokenStream::new(),
    };
    quote! {
        if #cond {
            #then_ts
        } #else_ts
    }
}

fn generate_for(pat: &TokenStream, expr: &TokenStream, body: &[Node], ctx: Ctx) -> TokenStream {
    let body_ts = generate_body_in(body, ctx);
    quote! {
        for #pat in #expr {
            #body_ts
        }
    }
}

fn generate_while(cond: &TokenStream, body: &[Node], ctx: Ctx) -> TokenStream {
    let body_ts = generate_body_in(body, ctx);
    quote! {
        while #cond {
            #body_ts
        }
    }
}

fn generate_match(expr: &TokenStream, arms: &[crate::ast::MatchArm], ctx: Ctx) -> TokenStream {
    let mut arms_ts = TokenStream::new();
    for arm in arms {
        let pat = &arm.pat;
        let body = generate_body_in(&arm.body, ctx);
        arms_ts.append_all(quote! {
            #pat => {
                #body
            }
        });
    }
    quote! {
        match #expr {
            #arms_ts
        }
    }
}

fn generate_cdata(body: &[Node]) -> TokenStream {
    // The inner body is generated with the CDATA context so that literal text
    // is written verbatim into a fresh buffer; spliced expressions still go
    // through `Render` and can opt out of escaping with the `.raw` modifier.
    let body_ts = generate_body_in(body, CDATA);
    quote! {
        __olav_out.push_str("<![CDATA[");
        {
            let mut __olav_cdata_tmp = ::olav::Markup::new();
            let __olav_cdata_buf = &mut __olav_cdata_tmp;
            #body_ts
            __olav_out.push_str(__olav_cdata_tmp.as_str());
        }
        __olav_out.push_str("]]>");
    }
}

fn generate_pi(name: &Spanned<String>, attrs: &[Attr], body: &[Node], ctx: Ctx) -> TokenStream {
    let n = name.value.clone();
    let attrs_ts = generate_attrs(attrs, ctx);
    let body_ts = generate_body_in(body, ctx);
    let out = ctx.out();
    let close = if body.is_empty() {
        quote! { #out.push_str("?>"); }
    } else {
        quote! { #out.push_str(" "); #body_ts #out.push_str("?>"); }
    };
    quote! {
        #out.push_str("<?");
        #out.push_str(#n);
        #attrs_ts
        #close
    }
}

fn generate_doctype(tokens: &str, ctx: Ctx) -> TokenStream {
    let out = ctx.out();
    quote! {
        #out.push_str("<!DOCTYPE ");
        #out.push_str(#tokens);
        #out.push_str(">");
    }
}
