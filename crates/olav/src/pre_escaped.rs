/// A wrapper that renders its inner value **without** escaping.
///
/// Use this to splice pre-escaped or pre-built markup into a template. The
/// value is inserted verbatim — `&`, `<`, `>` are **not** re-escaped.
///
/// # Example
///
/// ```
/// use olav::{xml, PreEscaped};
///
/// // The HTML inside PreEscaped is inserted as-is, not escaped.
/// let html = "<b>bold</b>";
/// let doc = xml! { div { @PreEscaped(html) } };
/// assert_eq!(doc.as_str(), "<div><b>bold</b></div>");
///
/// // Without PreEscaped, the angle brackets would be escaped:
/// let raw = "<b>bold</b>";
/// let doc = xml! { div { @raw } };
/// assert_eq!(doc.as_str(), "<div>&lt;b&gt;bold&lt;/b&gt;</div>");
/// ```
pub struct PreEscaped<T: AsRef<str>>(pub T);

impl<T: AsRef<str>> PreEscaped<T> {
    /// Wrap a value so it renders without escaping.
    ///
    /// # Example
    ///
    /// ```
    /// use olav::PreEscaped;
    ///
    /// let pe = PreEscaped::new("<raw>");
    /// assert_eq!(pe.as_str(), "<raw>");
    /// ```
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrow the wrapped string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl<T: AsRef<str>> AsRef<str> for PreEscaped<T> {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}
