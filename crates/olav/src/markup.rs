use crate::escape;

/// A block of XML markup that does not need to be escaped.
///
/// `Markup` is essentially a `String` that has already been verified to be
/// well-formed by the [`xml!`](crate::xml) macro at compile time.
///
/// # Example
///
/// ```
/// use olav::xml;
///
/// let doc = xml! { root { "hello" } };
/// assert_eq!(doc.as_str(), "<root>hello</root>");
/// ```
#[derive(Debug, Clone, Default, Eq, PartialEq, Hash)]
pub struct Markup(pub(crate) String);

impl Markup {
    /// Create an empty `Markup`.
    ///
    /// # Example
    ///
    /// ```
    /// use olav::Markup;
    ///
    /// let m = Markup::new();
    /// assert_eq!(m.as_str(), "");
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self(String::new())
    }

    /// Create an empty `Markup` with the given capacity hint.
    ///
    /// # Example
    ///
    /// ```
    /// use olav::Markup;
    ///
    /// let m = Markup::with_capacity(1024);
    /// assert_eq!(m.as_str(), "");
    /// ```
    #[must_use]
    pub fn with_capacity(n: usize) -> Self {
        Self(String::with_capacity(n))
    }

    /// Consume and return the underlying `String`.
    ///
    /// # Example
    ///
    /// ```
    /// use olav::xml;
    ///
    /// let doc = xml! { root { "x" } };
    /// let s: String = doc.into_string();
    /// assert_eq!(s, "<root>x</root>");
    /// ```
    pub fn into_string(self) -> String {
        self.0
    }

    /// Borrow the markup as a string slice.
    ///
    /// # Example
    ///
    /// ```
    /// use olav::xml;
    ///
    /// let doc = xml! { root { "hello" } };
    /// assert_eq!(doc.as_str(), "<root>hello</root>");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Append a raw, pre-escaped string. Caller is responsible for ensuring
    /// the string does not contain characters that would break XML.
    ///
    /// # Example
    ///
    /// ```
    /// use olav::Markup;
    ///
    /// let mut m = Markup::new();
    /// m.push_str("<raw>");
    /// assert_eq!(m.as_str(), "<raw>");
    /// ```
    pub fn push_str(&mut self, s: &str) {
        self.0.push_str(s);
    }

    /// Append text content, escaping `& < >`.
    ///
    /// # Example
    ///
    /// ```
    /// use olav::Markup;
    ///
    /// let mut m = Markup::new();
    /// m.push_text("a & b < c");
    /// assert_eq!(m.as_str(), "a &amp; b &lt; c");
    /// ```
    pub fn push_text(&mut self, s: &str) {
        escape::escape_text(s, &mut self.0);
    }

    /// Append an attribute, in the form `name="value"`, escaping the value
    /// for attribute context.
    ///
    /// # Example
    ///
    /// ```
    /// use olav::Markup;
    ///
    /// let mut m = Markup::new();
    /// m.push_attr("href", "https://example.com/?a=1&b=2");
    /// assert_eq!(m.as_str(), " href=\"https://example.com/?a=1&amp;b=2\"");
    /// ```
    pub fn push_attr(&mut self, name: &str, value: &str) {
        self.0.push(' ');
        self.0.push_str(name);
        self.0.push_str("=\"");
        escape::escape_attr(value, &mut self.0);
        self.0.push('"');
    }

    /// Append a self-closing tag: `<name/>`.
    ///
    /// # Example
    ///
    /// ```
    /// use olav::Markup;
    ///
    /// let mut m = Markup::new();
    /// m.push_self_closing("br");
    /// assert_eq!(m.as_str(), "<br/>");
    /// ```
    pub fn push_self_closing(&mut self, name: &str) {
        self.0.push('<');
        self.0.push_str(name);
        self.0.push_str("/>");
    }

    /// Write the markup into any `fmt::Write` sink.
    ///
    /// # Example
    ///
    /// ```
    /// use olav::xml;
    ///
    /// let doc = xml! { root { "x" } };
    /// let mut s = String::new();
    /// doc.render_to_fmt(&mut s).unwrap();
    /// assert_eq!(s, "<root>x</root>");
    /// ```
    pub fn render_to_fmt<W: std::fmt::Write>(&self, w: &mut W) -> std::fmt::Result {
        w.write_str(&self.0)
    }
}

impl std::fmt::Display for Markup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Write for Markup {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.0.push_str(s);
        Ok(())
    }
}

impl std::ops::Deref for Markup {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

/// Escape hatch: wrap an arbitrary string as pre-escaped markup.
///
/// This bypasses the compile-time guarantees of [`xml!`](crate::xml) — the
/// string is inserted verbatim, the same way as [`PreEscaped`](crate::PreEscaped).
/// Only use it for strings you know are well-formed XML.
impl From<String> for Markup {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for Markup {
    fn from(s: &str) -> Self {
        Self(s.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_and_into_string() {
        let m = Markup::new();
        assert_eq!(m.into_string(), "");
    }

    #[test]
    fn push_str_is_raw() {
        let mut m = Markup::new();
        m.push_str("<raw>");
        assert_eq!(m.as_str(), "<raw>");
    }

    #[test]
    fn push_text_escapes() {
        let mut m = Markup::new();
        m.push_text("a & b < c > d");
        assert_eq!(m.as_str(), "a &amp; b &lt; c &gt; d");
    }

    #[test]
    fn push_attr_escapes_value() {
        let mut m = Markup::new();
        m.push_attr("href", "a & b");
        assert_eq!(m.as_str(), " href=\"a &amp; b\"");
    }

    #[test]
    fn push_self_closing_works() {
        let mut m = Markup::new();
        m.push_self_closing("br");
        assert_eq!(m.as_str(), "<br/>");
    }

    #[test]
    fn display_yields_underlying_string() {
        let m = Markup::from("<root/>".to_string());
        assert_eq!(format!("{}", m), "<root/>");
    }

    #[test]
    fn deref_to_str() {
        let m = Markup::from("hello".to_string());
        assert_eq!(&*m, "hello");
        assert!(m.starts_with("he"));
    }

    #[test]
    fn render_to_fmt_works() {
        let m = Markup::from("<a/>".to_string());
        let mut out = String::new();
        m.render_to_fmt(&mut out).unwrap();
        assert_eq!(out, "<a/>");
    }
}
