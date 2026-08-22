//! The `Render` trait — anything that can be spliced into a `xml!` template.

use std::borrow::Cow;
use std::fmt::{self, Write as _};
use std::sync::Arc;

use crate::Markup;
use crate::PreEscaped;

/// A type that can be rendered as XML markup.
///
/// The [`xml!`](crate::xml) macro generates code that calls [`Render::render_to`]
/// for every `@expr` splice. Implementors must make sure that the produced
/// output is properly escaped for the context in which it is used (text vs
/// attribute).
///
/// The trait provides default implementations: `.render_to()` appends to a
/// [`Markup`], and `.render()` returns the result wrapped in a new [`Markup`].
///
/// # Built-in impls
///
/// `Render` is implemented for:
///
/// - `&str`, `String`, `Cow<'_, str>`, `char` — text content is XML-escaped
/// - `bool`, all integer types, `f32`, `f64` — formatted via `Display`
/// - `&T`, `&mut T`, `Box<T>`, `Arc<T>` — delegates to inner value
/// - `fmt::Arguments<'_>` — for `format_args!()` output
/// - [`Markup`] — raw insertion (already escaped)
/// - [`PreEscaped<T>`](PreEscaped) — raw insertion (for pre-built markup)
///
/// # Example
///
/// ```
/// use olav::{xml, Render};
///
/// let name = "world";
/// let doc = xml! { root { "hello " @name } };
/// assert_eq!(doc.as_str(), "<root>hello world</root>");
/// ```
pub trait Render {
    /// Append the rendered representation of `self` to `buf`.
    ///
    /// Implementors should escape the value appropriately for the context
    /// (text vs attribute) — the blanket impls handle this by calling
    /// [`Markup::push_text`](crate::Markup::push_text) (which escapes text
    /// characters) or [`Markup::push_str`](crate::Markup::push_str) (raw, for
    /// pre-escaped values).
    fn render_to(&self, buf: &mut Markup);

    /// Render `self` to a fresh [`Markup`].
    ///
    /// # Example
    ///
    /// ```
    /// use olav::Render;
    ///
    /// let m = "a & b".render();
    /// assert_eq!(m.as_str(), "a &amp; b");
    /// ```
    #[must_use]
    fn render(&self) -> Markup {
        let mut buf = Markup::new();
        self.render_to(&mut buf);
        buf
    }
}

impl Render for Markup {
    fn render_to(&self, buf: &mut Markup) {
        buf.push_str(self.as_str());
    }
}

impl Render for str {
    fn render_to(&self, buf: &mut Markup) {
        buf.push_text(self);
    }
}

impl Render for String {
    fn render_to(&self, buf: &mut Markup) {
        buf.push_text(self);
    }
}

impl<'a> Render for Cow<'a, str> {
    fn render_to(&self, buf: &mut Markup) {
        buf.push_text(self);
    }
}

impl Render for char {
    fn render_to(&self, buf: &mut Markup) {
        let mut tmp = [0u8; 4];
        buf.push_text(self.encode_utf8(&mut tmp));
    }
}

macro_rules! impl_render_for_display_int {
    ($($t:ty),* $(,)?) => {
        $(
            impl Render for $t {
                fn render_to(&self, buf: &mut Markup) {
                    let _ = write!(buf, "{}", self);
                }
            }
        )*
    };
}

impl_render_for_display_int! {
    i8, i16, i32, i64, i128, isize,
    u8, u16, u32, u64, u128, usize,
    f32, f64,
}

impl Render for bool {
    fn render_to(&self, buf: &mut Markup) {
        buf.push_str(if *self { "true" } else { "false" });
    }
}

impl<T: Render + ?Sized> Render for &T {
    fn render_to(&self, buf: &mut Markup) {
        T::render_to(self, buf);
    }
}

impl<T: Render + ?Sized> Render for &mut T {
    fn render_to(&self, buf: &mut Markup) {
        T::render_to(self, buf);
    }
}

impl<T: Render + ?Sized> Render for Box<T> {
    fn render_to(&self, buf: &mut Markup) {
        T::render_to(self, buf);
    }
}

impl<T: Render + ?Sized> Render for Arc<T> {
    fn render_to(&self, buf: &mut Markup) {
        T::render_to(self, buf);
    }
}

impl Render for fmt::Arguments<'_> {
    fn render_to(&self, buf: &mut Markup) {
        let _ = write!(buf, "{}", self);
    }
}

impl<T: AsRef<str>> Render for PreEscaped<T> {
    fn render_to(&self, buf: &mut Markup) {
        buf.push_str(self.0.as_ref());
    }
}

/// Wrapper that renders any [`std::fmt::Display`] value as escaped text.
///
/// Useful for third-party types you can't (or don't want to) implement
/// [`Render`] for manually.
///
/// # Example
///
/// ```
/// use olav::{xml, DisplayValue};
/// use std::net::Ipv4Addr;
///
/// let ip = Ipv4Addr::new(192, 168, 0, 1);
/// let doc = xml! { addr { @DisplayValue(ip) } };
/// assert_eq!(doc.as_str(), "<addr>192.168.0.1</addr>");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayValue<T>(pub T);

impl<T: std::fmt::Display> Render for DisplayValue<T> {
    fn render_to(&self, buf: &mut Markup) {
        let _ = write!(buf, "{}", self.0);
    }
}

/// `Option<T>` renders as the inner value when `Some`, nothing when `None`.
///
/// # Example
///
/// ```
/// use olav::xml;
///
/// let v: Option<i32> = Some(42);
/// let doc = xml! { p { @v } };
/// assert_eq!(doc.as_str(), "<p>42</p>");
///
/// let v: Option<i32> = None;
/// let doc = xml! { p { @v } };
/// assert_eq!(doc.as_str(), "<p></p>");
/// ```
impl<T: Render> Render for Option<T> {
    fn render_to(&self, buf: &mut Markup) {
        if let Some(v) = self {
            v.render_to(buf);
        }
    }
}

/// `Vec<T>` renders each element in order, concatenated.
///
/// # Example
///
/// ```
/// use olav::xml;
///
/// let v = vec!["a", "b", "c"];
/// let doc = xml! { p { @v } };
/// assert_eq!(doc.as_str(), "<p>abc</p>");
/// ```
impl<T: Render> Render for Vec<T> {
    fn render_to(&self, buf: &mut Markup) {
        for v in self {
            v.render_to(buf);
        }
    }
}

/// `&[T]` renders each element in order, concatenated.
impl<T: Render> Render for [T] {
    fn render_to(&self, buf: &mut Markup) {
        for v in self {
            v.render_to(buf);
        }
    }
}

/// `[T; N]` arrays render each element in order, concatenated.
impl<T: Render, const N: usize> Render for [T; N] {
    fn render_to(&self, buf: &mut Markup) {
        for v in self {
            v.render_to(buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markup_renders_raw() {
        let m = Markup::from("<a/>".to_string());
        let mut buf = Markup::new();
        m.render_to(&mut buf);
        assert_eq!(buf.as_str(), "<a/>");
    }

    #[test]
    fn str_escapes() {
        let mut buf = Markup::new();
        "a & b".render_to(&mut buf);
        assert_eq!(buf.as_str(), "a &amp; b");
    }

    #[test]
    fn string_escapes() {
        let mut buf = Markup::new();
        String::from("a < b").render_to(&mut buf);
        assert_eq!(buf.as_str(), "a &lt; b");
    }

    #[test]
    fn cow_escapes() {
        let mut buf = Markup::new();
        Cow::Borrowed("a & b").render_to(&mut buf);
        assert_eq!(buf.as_str(), "a &amp; b");
    }

    #[test]
    fn char_escapes() {
        let mut buf = Markup::new();
        '<'.render_to(&mut buf);
        assert_eq!(buf.as_str(), "&lt;");
    }

    #[test]
    fn integers_render_via_display() {
        let mut buf = Markup::new();
        42i32.render_to(&mut buf);
        assert_eq!(buf.as_str(), "42");

        let mut buf = Markup::new();
        (-7i64).render_to(&mut buf);
        assert_eq!(buf.as_str(), "-7");

        let mut buf = Markup::new();
        let v: f64 = 1.5;
        v.render_to(&mut buf);
        assert_eq!(buf.as_str(), "1.5");
    }

    #[test]
    fn bool_renders_true_false() {
        let mut buf = Markup::new();
        true.render_to(&mut buf);
        assert_eq!(buf.as_str(), "true");

        let mut buf = Markup::new();
        false.render_to(&mut buf);
        assert_eq!(buf.as_str(), "false");
    }

    #[test]
    fn ref_delegates() {
        let s = String::from("a & b");
        let mut buf = Markup::new();
        s.render_to(&mut buf);
        assert_eq!(buf.as_str(), "a &amp; b");
    }

    #[test]
    fn box_delegates() {
        let s = String::from("a & b");
        let b: Box<String> = Box::new(s);
        let mut buf = Markup::new();
        b.render_to(&mut buf);
        assert_eq!(buf.as_str(), "a &amp; b");
    }

    #[test]
    fn arc_delegates() {
        let s = String::from("a & b");
        let a: Arc<String> = Arc::new(s);
        let mut buf = Markup::new();
        a.render_to(&mut buf);
        assert_eq!(buf.as_str(), "a &amp; b");
    }

    #[test]
    fn pre_escaped_inserts_raw() {
        let pe = PreEscaped("<b>raw</b>");
        let mut buf = Markup::new();
        pe.render_to(&mut buf);
        assert_eq!(buf.as_str(), "<b>raw</b>");
    }

    #[test]
    fn format_args_renders() {
        let name = "world";
        let mut buf = Markup::new();
        format_args!("hello {} & {}", name, name).render_to(&mut buf);
        assert_eq!(buf.as_str(), "hello world & world");
    }

    #[test]
    fn render_returns_markup() {
        let m = "a & b".render();
        assert_eq!(m.as_str(), "a &amp; b");
    }
}
