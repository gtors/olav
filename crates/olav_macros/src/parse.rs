//! Parser: TokenStream → Vec\<Node\>

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};

use crate::ast::{Attr, MatchArm, Node, Spanned, SpliceMod};

/// Validate that `name` is a well-formed XML name (NameStartChar NameChar*).
///
/// Bracketed names (`[foo-bar]`) accept arbitrary token text, so without this
/// check a typo would silently produce broken XML.
fn validate_xml_name(name: &str, span: Span) -> syn::Result<()> {
    let mut chars = name.chars();
    let ok_start = matches!(chars.next(), Some(c) if c.is_alphabetic() || c == '_' || c == ':');
    if !ok_start {
        return Err(syn::Error::new(
            span,
            format!("`{name}` is not a valid XML name: bad first character"),
        ));
    }
    if let Some(bad) = chars.find(|c| !(c.is_alphanumeric() || matches!(c, '_' | ':' | '-' | '.')))
    {
        return Err(syn::Error::new(
            span,
            format!("`{name}` is not a valid XML name: unexpected `{bad}`"),
        ));
    }
    Ok(())
}

/// Attribute names are always statically known (identifiers or bracketed
/// token text), so duplicates can be rejected at compile time.
fn check_duplicate_attrs(attrs: &[Attr]) -> syn::Result<()> {
    for (i, attr) in attrs.iter().enumerate() {
        let name = attr_name(attr);
        if attrs[..i].iter().any(|earlier| attr_name(earlier) == name) {
            return Err(syn::Error::new(
                attr_name_span(attr),
                format!("duplicate attribute `{name}`"),
            ));
        }
    }
    Ok(())
}

fn attr_name(attr: &Attr) -> &str {
    match attr {
        Attr::Lit { name, .. } | Attr::Expr { name, .. } | Attr::CondExpr { name, .. } => {
            &name.value
        }
    }
}

fn attr_name_span(attr: &Attr) -> Span {
    match attr {
        Attr::Lit { name, .. } | Attr::Expr { name, .. } | Attr::CondExpr { name, .. } => name.span,
    }
}

/// Internal parser state. Pre-collects tokens to allow lookahead.
pub struct Parser {
    tokens: Vec<TokenTree>,
    pos: usize,
    /// True while parsing the body of `@cdata { ... }`: literal text there
    /// must not contain the CDATA terminator `]]>`.
    cdata: bool,
    /// True while parsing the body of a processing instruction: literal text
    /// there must not contain the PI terminator `?>`.
    pi: bool,
}

impl Parser {
    pub fn new(input: TokenStream) -> Self {
        Self {
            tokens: input.into_iter().collect(),
            pos: 0,
            cdata: false,
            pi: false,
        }
    }

    fn with_flags(input: TokenStream, cdata: bool, pi: bool) -> Self {
        Self {
            tokens: input.into_iter().collect(),
            pos: 0,
            cdata,
            pi,
        }
    }

    fn peek(&self) -> Option<&TokenTree> {
        self.tokens.get(self.pos)
    }

    fn peek_at(&self, n: usize) -> Option<&TokenTree> {
        self.tokens.get(self.pos + n)
    }

    fn bump(&mut self) -> Option<TokenTree> {
        let r = self.tokens.get(self.pos).cloned();
        if r.is_some() {
            self.pos += 1;
        }
        r
    }

    fn span_here(&self) -> Span {
        self.peek()
            .map(|t| t.span())
            .unwrap_or_else(Span::call_site)
    }

    fn err<T>(&self, msg: impl std::fmt::Display) -> syn::Result<T> {
        Err(syn::Error::new(self.span_here(), format!("{}", msg)))
    }

    pub fn parse_body(&mut self) -> syn::Result<Vec<Node>> {
        let mut nodes = Vec::new();
        while self.pos < self.tokens.len() {
            nodes.push(self.parse_node()?);
        }
        Ok(nodes)
    }

    fn parse_node(&mut self) -> syn::Result<Node> {
        let tt = self
            .peek()
            .cloned()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "unexpected end of input"))?;
        match &tt {
            TokenTree::Punct(p) => match p.as_char() {
                '@' => self.parse_at(),
                '?' => self.parse_pi(),
                '!' => self.parse_doctype(),
                _ => self.err(format!("unexpected punctuation `{}`", p.as_char())),
            },
            TokenTree::Ident(_) => self.parse_element_bare(),
            TokenTree::Group(g) if g.delimiter() == Delimiter::Bracket => {
                self.parse_element_brackets()
            }
            TokenTree::Literal(_) => self.parse_text(),
            _ => self.err("unexpected token"),
        }
    }

    // ==================== Elements ====================

    fn parse_element_bare(&mut self) -> syn::Result<Node> {
        let name_tt = self.bump().expect("parse_node dispatched on Ident");
        let name_span = name_tt.span();
        let TokenTree::Ident(id) = &name_tt else {
            return Err(syn::Error::new(name_span, "expected element name"));
        };
        let name_str = id.to_string();
        let name = Spanned::new(name_str, name_span);
        let attrs = self.parse_attrs()?;
        let body = self.parse_body_block_opt()?;
        Ok(Node::Element {
            name,
            attrs,
            body,
            span: name_span,
        })
    }

    fn parse_element_brackets(&mut self) -> syn::Result<Node> {
        let open_tt = self.bump().expect("parse_node dispatched on `[` group"); // the [ group
        let open_span = open_tt.span();
        let TokenTree::Group(g) = open_tt else {
            return self.err("expected `[...]` for bracketed element name");
        };
        if g.delimiter() != Delimiter::Bracket {
            return self.err("expected `[...]` for bracketed element name");
        }
        let inner = g.stream();
        // The name is the literal representation of all tokens inside brackets.
        let name_str = inner.to_string();
        // Trim whitespace
        let name_str = name_str.trim().to_string();
        if name_str.is_empty() {
            return Err(syn::Error::new(open_span, "empty bracketed element name"));
        }
        validate_xml_name(&name_str, open_span)?;
        let name = Spanned::new(name_str, open_span);

        let attrs = self.parse_attrs()?;
        let body = self.parse_body_block_opt()?;
        Ok(Node::Element {
            name,
            attrs,
            body,
            span: open_span,
        })
    }

    fn parse_attrs(&mut self) -> syn::Result<Vec<Attr>> {
        let mut attrs = Vec::new();
        // Attributes, if present, are wrapped in `(...)` group.
        match self.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
                let bumped = self.bump().expect("peeked `(...)` group");
                let TokenTree::Group(g) = bumped else {
                    return self.err("expected `(...)` attribute list");
                };
                let inner = g.stream();
                let mut inner_parser = Parser::new(inner);
                inner_parser.parse_attrs_in_parens(&mut attrs)?;
                check_duplicate_attrs(&attrs)?;
                if inner_parser.pos < inner_parser.tokens.len() {
                    return Err(syn::Error::new(
                        inner_parser.span_here(),
                        "unexpected token inside attribute list",
                    ));
                }
            }
            _ => {}
        }
        Ok(attrs)
    }

    fn parse_attrs_in_parens(&mut self, attrs: &mut Vec<Attr>) -> syn::Result<()> {
        // Skip trailing commas between attrs
        loop {
            match self.peek() {
                None => return Ok(()),
                Some(TokenTree::Punct(p)) if p.as_char() == ',' => {
                    self.bump();
                    continue;
                }
                _ => {}
            }
            // Try to parse one attr
            let before = self.pos;
            let parsed_attr = self.parse_one_attr()?;
            if self.pos == before {
                // Nothing consumed — end of attrs
                return Ok(());
            }
            attrs.push(parsed_attr);
            // After attr, expect `,` or end
            match self.peek() {
                None => return Ok(()),
                Some(TokenTree::Punct(p)) if p.as_char() == ',' => {
                    self.bump();
                    continue;
                }
                _ => return Ok(()),
            }
        }
    }

    fn parse_one_attr(&mut self) -> syn::Result<Attr> {
        let name_tt = self
            .peek()
            .cloned()
            .ok_or_else(|| syn::Error::new(Span::call_site(), "expected attribute"))?;
        let (name, span) = match &name_tt {
            TokenTree::Ident(id) => (id.to_string(), id.span()),
            TokenTree::Group(g) if g.delimiter() == Delimiter::Bracket => {
                let inner = g.stream().to_string().trim().to_string();
                if inner.is_empty() {
                    return Err(syn::Error::new(g.span(), "empty attribute name"));
                }
                validate_xml_name(&inner, g.span())?;
                (inner, g.span())
            }
            _ => return Err(syn::Error::new(name_tt.span(), "expected attribute name")),
        };
        self.bump();
        let name = Spanned::new(name, span);
        self.parse_attr_value(name)
    }

    fn parse_attr_value(&mut self, name: Spanned<String>) -> syn::Result<Attr> {
        let tt = self
            .peek()
            .cloned()
            .ok_or_else(|| syn::Error::new(name.span, "expected attribute value"))?;
        match &tt {
            TokenTree::Punct(p) if p.as_char() == '=' => {
                self.bump(); // consume =
                let val_tt = self
                    .peek()
                    .cloned()
                    .ok_or_else(|| syn::Error::new(p.span(), "expected value after `=`"))?;
                if let TokenTree::Literal(lit) = &val_tt {
                    let lit_ts: TokenStream =
                        std::iter::once(TokenTree::Literal(lit.clone())).collect();
                    let lit_str: syn::LitStr = syn::parse2(lit_ts)?;
                    self.bump();
                    Ok(Attr::Lit {
                        name,
                        value: Spanned::new(lit_str.value(), lit_str.span()),
                    })
                } else {
                    let expr = self.collect_attr_expr()?;
                    Ok(Attr::Expr { name, value: expr })
                }
            }
            TokenTree::Punct(p) if p.as_char() == '?' => {
                self.bump(); // consume ?
                let peek2 = self.peek().cloned();
                if let Some(TokenTree::Punct(p2)) = &peek2 {
                    if p2.as_char() == '=' {
                        self.bump(); // consume =
                        let val_expr = self.collect_attr_expr()?;
                        // `attr?=value` — condition defaults to literal `true` (always present)
                        let cond = TokenStream::from(TokenTree::Ident(proc_macro2::Ident::new(
                            "true",
                            p2.span(),
                        )));
                        Ok(Attr::CondExpr {
                            name,
                            cond,
                            value: val_expr,
                        })
                    } else {
                        // bare `?` — peek next for the conditional expression
                        let cond = self.collect_attr_expr()?;
                        Ok(Attr::CondExpr {
                            name,
                            cond,
                            value: TokenStream::new(),
                        })
                    }
                } else {
                    // bare `?` without following expr — condition defaults to `true`
                    let cond = TokenStream::from(TokenTree::Ident(proc_macro2::Ident::new(
                        "true",
                        p.span(),
                    )));
                    Ok(Attr::CondExpr {
                        name,
                        cond,
                        value: TokenStream::new(),
                    })
                }
            }
            _ => self.err("expected `=` or `?` after attribute name"),
        }
    }

    /// Collect expression tokens for an attribute value (until `,`, `)`, `]`, `}` or end).
    fn collect_attr_expr(&mut self) -> syn::Result<TokenStream> {
        let mut buf = TokenStream::new();
        let mut paren_depth: u32 = 0;
        let mut bracket_depth: u32 = 0;
        let mut brace_depth: u32 = 0;
        loop {
            let tt = self.peek().cloned();
            match tt {
                None => break,
                Some(TokenTree::Group(g)) => {
                    let delim = g.delimiter();
                    let was_top = paren_depth == 0 && bracket_depth == 0 && brace_depth == 0;
                    self.bump();
                    match delim {
                        Delimiter::Parenthesis => paren_depth += 1,
                        Delimiter::Bracket => bracket_depth += 1,
                        Delimiter::Brace => brace_depth += 1,
                        Delimiter::None => {}
                    }
                    buf.extend(std::iter::once(TokenTree::Group(g)));
                    match delim {
                        Delimiter::Parenthesis => paren_depth -= 1,
                        Delimiter::Bracket => bracket_depth -= 1,
                        Delimiter::Brace => brace_depth -= 1,
                        Delimiter::None => {}
                    }
                    // Check if we should stop — comma or closing at top level
                    if was_top {
                        // After the group, check if next is `,`, `)`, `]`, or end of attr list
                        if matches!(self.peek(), Some(TokenTree::Punct(p)) if p.as_char() == ',') {
                            break;
                        }
                        // We don't break on `)` or `]` because we're at top level of attr expr,
                        // not inside parens. The outer attribute loop will see them.
                    }
                }
                Some(TokenTree::Punct(p)) => {
                    let ch = p.as_char();
                    let at_top = paren_depth == 0 && bracket_depth == 0 && brace_depth == 0;
                    if at_top && (ch == ',' || ch == ')' || ch == ']' || ch == '}') {
                        break;
                    }
                    self.bump();
                    buf.extend(std::iter::once(TokenTree::Punct(p)));
                }
                Some(tt) => {
                    self.bump();
                    buf.extend(std::iter::once(tt));
                }
            }
        }
        Ok(buf)
    }

    fn parse_body_block_opt(&mut self) -> syn::Result<Vec<Node>> {
        match self.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                let bumped = self.bump().expect("peeked `{...}` group");
                let TokenTree::Group(g) = bumped else {
                    return self.err("expected `{...}` body");
                };
                let mut parser = Parser::with_flags(g.stream(), self.cdata, self.pi);
                parser.parse_body()
            }
            _ => Ok(Vec::new()), // self-closing
        }
    }

    // ==================== Text ====================

    fn parse_text(&mut self) -> syn::Result<Node> {
        let tt = self.bump().expect("parse_node dispatched on Literal");
        let lit = match &tt {
            TokenTree::Literal(lit) => lit.clone(),
            _ => return Err(syn::Error::new(tt.span(), "expected a string literal")),
        };
        let single: TokenStream = std::iter::once(TokenTree::Literal(lit)).collect();
        if let Ok(s) = syn::parse2::<syn::LitStr>(single.clone()) {
            let value = s.value();
            if self.cdata && value.contains("]]>") {
                return Err(syn::Error::new(
                    s.span(),
                    "literal text inside `@cdata` must not contain the CDATA terminator `]]>`",
                ));
            }
            if self.pi && value.contains("?>") {
                return Err(syn::Error::new(
                    s.span(),
                    "literal text inside a processing instruction must not contain `?>`",
                ));
            }
            return Ok(Node::Text(Spanned::new(value, s.span())));
        }
        if let Ok(c) = syn::parse2::<syn::LitChar>(single) {
            return Ok(Node::Text(Spanned::new(c.value().to_string(), c.span())));
        }
        Err(syn::Error::new(
            tt.span(),
            "only string (or char) literals are valid text; use `@expr` to interpolate values",
        ))
    }

    // ==================== At ====================

    fn parse_at(&mut self) -> syn::Result<Node> {
        let at_span = self.bump().expect("`@` was peeked").span();
        let next = self
            .peek()
            .cloned()
            .ok_or_else(|| syn::Error::new(at_span, "expected expression after `@`"))?;
        match &next {
            TokenTree::Ident(id) => match id.to_string().as_str() {
                "if" => self.parse_if(at_span),
                "for" => self.parse_for(at_span),
                "while" => self.parse_while(at_span),
                "match" => self.parse_match(at_span),
                "cdata" => self.parse_cdata(at_span),
                "comment" => self.parse_comment_kw(at_span),
                _ => self.parse_splice(at_span),
            },
            _ => self.parse_splice(at_span),
        }
    }

    fn parse_splice(&mut self, at_span: Span) -> syn::Result<Node> {
        let expr_tokens = self.collect_expr_tokens()?;
        // Check for `.raw` modifier at the end
        let (expr_tokens, modifier) = self.strip_raw_modifier(expr_tokens)?;
        Ok(Node::Splice {
            expr: expr_tokens,
            modifier,
            span: at_span,
        })
    }

    fn collect_expr_tokens(&mut self) -> syn::Result<TokenStream> {
        let mut buf = TokenStream::new();
        let mut paren_depth: u32 = 0;
        let mut bracket_depth: u32 = 0;
        loop {
            let tt = self.peek().cloned();
            match tt {
                None => break,
                Some(TokenTree::Group(g)) => {
                    let delim = g.delimiter();
                    match delim {
                        Delimiter::Parenthesis => paren_depth += 1,
                        Delimiter::Bracket => bracket_depth += 1,
                        Delimiter::Brace => return Ok(buf), // top-level body block ends expr
                        Delimiter::None => {}
                    }
                    self.bump();
                    buf.extend(std::iter::once(TokenTree::Group(g)));
                    match delim {
                        Delimiter::Parenthesis => paren_depth -= 1,
                        Delimiter::Bracket => bracket_depth -= 1,
                        Delimiter::None => {}
                        Delimiter::Brace => {}
                    }
                }
                Some(TokenTree::Punct(p)) => {
                    let ch = p.as_char();
                    let at_top = paren_depth == 0 && bracket_depth == 0;
                    if at_top && (ch == ';' || ch == '@') {
                        break;
                    }
                    if at_top && ch == '/' {
                        // Check if next is `*` (comment)
                        if matches!(self.peek_at(1), Some(TokenTree::Punct(p2)) if p2.as_char() == '*')
                        {
                            break;
                        }
                    }
                    self.bump();
                    buf.extend(std::iter::once(TokenTree::Punct(p)));
                }
                Some(TokenTree::Ident(_)) => {
                    let at_top = paren_depth == 0 && bracket_depth == 0;
                    if at_top {
                        // Check if next is `{` — element start
                        if matches!(
                            self.peek_at(1),
                            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace
                        ) {
                            break;
                        }
                        // Also check for attr-like forms: `ident(name ... )` — bracketed attr with parens
                        // `name { ... }` would be the brace we already check.
                        // `name ( ... )` form would be... uncommon; not supported.
                    }
                    let tt = self.bump().expect("peeked ident exists");
                    buf.extend(std::iter::once(tt));
                }
                Some(tt) => {
                    self.bump();
                    buf.extend(std::iter::once(tt));
                }
            }
        }
        Ok(buf)
    }

    fn strip_raw_modifier(&self, tokens: TokenStream) -> syn::Result<(TokenStream, SpliceMod)> {
        let token_vec: Vec<TokenTree> = tokens.into_iter().collect();
        if token_vec.len() >= 2
            && let (TokenTree::Punct(p), TokenTree::Ident(id)) = (
                &token_vec[token_vec.len() - 2],
                &token_vec[token_vec.len() - 1],
            )
            && p.as_char() == '.'
            && id == "raw"
        {
            let mut new_tokens = token_vec.clone();
            new_tokens.pop();
            new_tokens.pop();
            return Ok((new_tokens.into_iter().collect(), SpliceMod::Raw));
        }
        Ok((token_vec.into_iter().collect(), SpliceMod::Escaped))
    }

    // ==================== Control flow ====================

    fn parse_if(&mut self, at_span: Span) -> syn::Result<Node> {
        self.bump(); // consume `if`
        let cond = self.collect_until_brace()?;
        let then_body = self.parse_body_block_opt()?;
        let else_body = if self.peek_kw("else") {
            self.bump();
            self.parse_body_block_opt()?
        } else {
            Vec::new()
        };
        let else_ = if else_body.is_empty() {
            None
        } else {
            Some(else_body)
        };
        Ok(Node::If {
            cond,
            then: then_body,
            else_,
            span: at_span,
        })
    }

    fn parse_for(&mut self, at_span: Span) -> syn::Result<Node> {
        self.bump(); // consume `for`
        let pat = self.collect_until_kw("in")?;
        self.bump(); // consume `in`
        let expr = self.collect_until_brace()?;
        let body = self.parse_body_block_opt()?;
        Ok(Node::For {
            pat,
            expr,
            body,
            span: at_span,
        })
    }

    fn parse_while(&mut self, at_span: Span) -> syn::Result<Node> {
        self.bump(); // consume `while`
        let cond = self.collect_until_brace()?;
        let body = self.parse_body_block_opt()?;
        Ok(Node::While {
            cond,
            body,
            span: at_span,
        })
    }

    fn parse_match(&mut self, at_span: Span) -> syn::Result<Node> {
        self.bump(); // consume `match`
        let expr = self.collect_until_brace()?;
        let body_tokens = match self.peek() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                let bumped = self.bump().expect("peeked `{...}` group");
                let TokenTree::Group(g) = bumped else {
                    return self.err("expected `{` after match expression");
                };
                g.stream()
            }
            _ => return self.err("expected `{` after match expression"),
        };
        let mut parser = Parser::new(body_tokens);
        let mut arms = Vec::new();
        while parser.pos < parser.tokens.len() {
            let pat = parser.collect_until_puncts(&['=', '>'])?;
            // Expect `=>`
            let arrow_span = parser.span_here();
            match (parser.peek(), parser.peek_at(1)) {
                (Some(TokenTree::Punct(p1)), Some(TokenTree::Punct(p2)))
                    if p1.as_char() == '=' && p2.as_char() == '>' =>
                {
                    parser.bump();
                    parser.bump();
                }
                _ => {
                    return Err(syn::Error::new(arrow_span, "expected `=>` in match arm"));
                }
            }
            let body = parser.parse_body_block_opt()?;
            // Optional comma after arm body
            if let Some(TokenTree::Punct(p)) = parser.peek()
                && p.as_char() == ','
            {
                parser.bump();
            }
            arms.push(MatchArm {
                pat,
                body,
                arrow_span,
            });
        }
        Ok(Node::Match {
            expr,
            arms,
            span: at_span,
        })
    }

    fn parse_cdata(&mut self, at_span: Span) -> syn::Result<Node> {
        self.bump(); // consume `cdata`
        let saved = self.cdata;
        self.cdata = true;
        let body = self.parse_body_block_opt()?;
        self.cdata = saved;
        Ok(Node::Cdata {
            body,
            span: at_span,
        })
    }

    fn peek_kw(&self, kw: &str) -> bool {
        match self.peek() {
            Some(TokenTree::Ident(id)) => id == kw,
            _ => false,
        }
    }

    /// Collect tokens until a top-level Group with brace delimiter, returning
    /// the collected tokens (not including the brace).
    fn collect_until_brace(&mut self) -> syn::Result<TokenStream> {
        let mut buf = TokenStream::new();
        let paren_depth: u32 = 0;
        let bracket_depth: u32 = 0;
        loop {
            let tt = self.peek().cloned();
            match tt {
                None => return self.err("expected `{`"),
                Some(TokenTree::Group(g)) => {
                    let delim = g.delimiter();
                    if delim == Delimiter::Brace && paren_depth == 0 && bracket_depth == 0 {
                        return Ok(buf);
                    }
                    self.bump();
                    buf.extend(std::iter::once(TokenTree::Group(g)));
                }
                Some(tt) => {
                    self.bump();
                    buf.extend(std::iter::once(tt));
                }
            }
        }
    }

    fn collect_until_kw(&mut self, kw: &str) -> syn::Result<TokenStream> {
        let mut buf = TokenStream::new();
        loop {
            let tt = self.peek().cloned();
            match tt {
                None => return self.err(format!("expected `{}`", kw)),
                Some(TokenTree::Ident(id)) if id == kw => {
                    return Ok(buf);
                }
                Some(TokenTree::Group(g)) => {
                    self.bump();
                    buf.extend(std::iter::once(TokenTree::Group(g)));
                }
                Some(tt) => {
                    self.bump();
                    buf.extend(std::iter::once(tt));
                }
            }
        }
    }

    fn collect_until_puncts(&mut self, puncts: &[char]) -> syn::Result<TokenStream> {
        let mut buf = TokenStream::new();
        let paren_depth: u32 = 0;
        let bracket_depth: u32 = 0;
        let brace_depth: u32 = 0;
        loop {
            let tt = self.peek().cloned();
            match tt {
                None => return self.err("expected more tokens"),
                Some(TokenTree::Group(g)) => {
                    self.bump();
                    buf.extend(std::iter::once(TokenTree::Group(g)));
                }
                Some(TokenTree::Punct(p))
                    if paren_depth == 0
                        && bracket_depth == 0
                        && brace_depth == 0
                        && puncts.contains(&p.as_char()) =>
                {
                    return Ok(buf);
                }
                Some(tt) => {
                    self.bump();
                    buf.extend(std::iter::once(tt));
                }
            }
        }
    }

    // ==================== Special XML nodes ====================

    fn parse_pi(&mut self) -> syn::Result<Node> {
        let q_span = self.bump().expect("`?` was peeked").span(); // consume `?`
        // Collect PI name (idents and hyphens, e.g. `xml`, `xml-stylesheet`).
        // Stop when we see an ident followed by `=` or `?` (which would be an attr).
        let mut name_str = String::new();
        let mut name_span = q_span;
        let mut has_any = false;
        loop {
            match self.peek() {
                Some(TokenTree::Ident(id)) => {
                    // Stop if this ident is followed by `=` or `?` (attr start)
                    if matches!(self.peek_at(1), Some(TokenTree::Punct(p)) if matches!(p.as_char(), '=' | '?'))
                    {
                        break;
                    }
                    if !has_any {
                        name_span = id.span();
                        has_any = true;
                    }
                    name_str.push_str(&id.to_string());
                    self.bump();
                }
                Some(TokenTree::Punct(p)) if p.as_char() == '-' => {
                    if !has_any {
                        name_span = p.span();
                        has_any = true;
                    }
                    name_str.push('-');
                    self.bump();
                }
                _ => break,
            }
        }
        if !has_any {
            return Err(syn::Error::new(
                q_span,
                "expected processing instruction name",
            ));
        }
        let name = Spanned::new(name_str, name_span);
        let attrs = self.parse_pi_attrs()?;
        let saved_pi = self.pi;
        self.pi = true;
        let body = self.parse_body_block_opt()?;
        self.pi = saved_pi;
        Ok(Node::Pi {
            name,
            attrs,
            body,
            span: q_span,
        })
    }

    fn parse_pi_attrs(&mut self) -> syn::Result<Vec<Attr>> {
        let mut attrs = Vec::new();
        while let Some(TokenTree::Ident(_)) = self.peek() {
            let is_attr = match self.peek_at(1) {
                Some(TokenTree::Punct(p)) => matches!(p.as_char(), '=' | '?'),
                _ => false,
            };
            if !is_attr {
                break;
            }
            let name_tt = self.bump().expect("peeked ident exists");
            let name_span = name_tt.span();
            let TokenTree::Ident(id) = &name_tt else {
                return Err(syn::Error::new(name_span, "expected attribute name"));
            };
            let name_str = id.to_string();
            let name = Spanned::new(name_str, name_span);
            let attr = self.parse_attr_value(name)?;
            // PI contents are terminated by `?>`, which must not appear inside.
            if let Attr::Lit { value, .. } = &attr
                && value.value.contains("?>")
            {
                return Err(syn::Error::new(
                    value.span,
                    "processing instruction content must not contain `?>`",
                ));
            }
            attrs.push(attr);
        }
        check_duplicate_attrs(&attrs)?;
        Ok(attrs)
    }

    fn parse_doctype(&mut self) -> syn::Result<Node> {
        let bang_span = self.bump().expect("`!` was peeked").span(); // consume `!`
        let kw_tt = self
            .peek()
            .cloned()
            .ok_or_else(|| syn::Error::new(bang_span, "expected `DOCTYPE` after `!`"))?;
        match &kw_tt {
            TokenTree::Ident(id) if id == "DOCTYPE" => {
                self.bump();
            }
            _ => return Err(syn::Error::new(kw_tt.span(), "expected `DOCTYPE`")),
        }
        // Collect DOCTYPE specification: optional root name, then optional
        // SYSTEM/PUBLIC + urls, until the next element/PI/brace body.
        let mut parts: Vec<TokenTree> = Vec::new();
        // Root name: only if not followed by `(...)` or `{...}` (those start an element).
        if let Some(TokenTree::Ident(_)) = self.peek() {
            let is_element_start = matches!(
                self.peek_at(1),
                Some(TokenTree::Group(g))
                    if matches!(g.delimiter(), Delimiter::Parenthesis | Delimiter::Brace)
            );
            if !is_element_start {
                parts.push(self.bump().expect("peeked ident exists"));
            }
        }
        // Optional SYSTEM/PUBLIC etc. — collect until we see an element start
        // (ident followed by `(...)` or `{...}`) or a top-level brace body group.
        loop {
            match self.peek().cloned() {
                None => break,
                Some(TokenTree::Group(g)) => {
                    if g.delimiter() == Delimiter::Brace {
                        break;
                    }
                    parts.push(self.bump().expect("peeked group exists"));
                }
                Some(TokenTree::Ident(_)) => {
                    let is_element_start = matches!(
                        self.peek_at(1),
                        Some(TokenTree::Group(g))
                            if matches!(g.delimiter(), Delimiter::Parenthesis | Delimiter::Brace)
                    );
                    if is_element_start {
                        break;
                    }
                    parts.push(self.bump().expect("peeked ident exists"));
                }
                Some(_) => {
                    parts.push(self.bump().expect("peeked token exists"));
                }
            }
        }
        // Join tokens with single spaces so the emitted text is deterministic
        // (independent of TokenStream printing quirks).
        let spec = parts
            .iter()
            .map(|tt| tt.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        Ok(Node::Doctype {
            spec,
            span: bang_span,
        })
    }

    fn parse_comment_kw(&mut self, at_span: Span) -> syn::Result<Node> {
        self.bump(); // consume `comment`
        // Expect a string literal
        let tt = self
            .peek()
            .cloned()
            .ok_or_else(|| syn::Error::new(at_span, "expected string literal after `@comment`"))?;
        match &tt {
            TokenTree::Literal(lit) => {
                let lit_ts: TokenStream =
                    std::iter::once(TokenTree::Literal(lit.clone())).collect();
                let lit_str: syn::LitStr = syn::parse2(lit_ts)?;
                let value = lit_str.value();
                if value.contains("--") {
                    return Err(syn::Error::new(
                        lit_str.span(),
                        "comment text must not contain `--` (forbidden by XML 1.0)",
                    ));
                }
                self.bump();
                Ok(Node::Comment {
                    text: Spanned::new(value, lit_str.span()),
                    span: at_span,
                })
            }
            _ => Err(syn::Error::new(
                tt.span(),
                "expected string literal after `@comment`",
            )),
        }
    }
}
