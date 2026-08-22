//! XML escape utilities.
//!
//! Per XML 1.0 specification:
//! - Text content: `&` `<` `>` MUST be escaped
//! - Attribute values: `&` `<` `"` MUST be escaped; `'` SHOULD be escaped
//! - Control chars (U+0000..U+0020 except tab/newline/CR) are forbidden in XML 1.0
//!   and MUST be escaped as `&#xNN;`

/// Append `s` to `out`, escaping for XML text content.
///
/// Mirrors [`escape_attr`]: control characters forbidden in XML 1.0 are
/// emitted as numeric character references so the output never contains
/// raw invalid bytes.
pub fn escape_text(s: &str, out: &mut String) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\x00'..='\x08' | '\x0B' | '\x0C' | '\x0E'..='\x1F' => {
                use std::fmt::Write as _;
                let _ = write!(out, "&#x{:X};", ch as u32);
            }
            c => out.push(c),
        }
    }
}

/// Append `s` to `out`, escaping for XML attribute values.
pub fn escape_attr(s: &str, out: &mut String) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // Control chars forbidden in XML 1.0
            '\x00'..='\x08' | '\x0B' | '\x0C' | '\x0E'..='\x1F' => {
                use std::fmt::Write as _;
                let _ = write!(out, "&#x{:X};", ch as u32);
            }
            c => out.push(c),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_escapes_amp_lt_gt() {
        let mut s = String::new();
        escape_text("a & b < c > d", &mut s);
        assert_eq!(s, "a &amp; b &lt; c &gt; d");
    }

    #[test]
    fn text_keeps_quotes() {
        let mut s = String::new();
        escape_text("\"hello\" 'world'", &mut s);
        assert_eq!(s, "\"hello\" 'world'");
    }

    #[test]
    fn attr_escapes_amp_lt_quot_apos() {
        let mut s = String::new();
        escape_attr("a & b < c \" d ' e", &mut s);
        assert_eq!(s, "a &amp; b &lt; c &quot; d &apos; e");
    }

    #[test]
    fn attr_escapes_control_chars() {
        let mut s = String::new();
        escape_attr("a\x01b", &mut s);
        assert_eq!(s, "a&#x1;b");

        let mut s = String::new();
        escape_attr("\x00", &mut s);
        assert_eq!(s, "&#x0;");
    }

    #[test]
    fn attr_keeps_normal_chars() {
        let mut s = String::new();
        escape_attr("hello world 123 _-+=", &mut s);
        assert_eq!(s, "hello world 123 _-+=");
    }

    #[test]
    fn attr_keeps_tab_lf_cr() {
        let mut s = String::new();
        escape_attr("a\tb\nc\rd", &mut s);
        assert_eq!(s, "a\tb\nc\rd");
    }

    #[test]
    fn text_handles_unicode() {
        let mut s = String::new();
        escape_text("héllo & wörld", &mut s);
        assert_eq!(s, "héllo &amp; wörld");
    }

    #[test]
    fn text_escapes_control_chars() {
        let mut s = String::new();
        escape_text("a\x01b", &mut s);
        assert_eq!(s, "a&#x1;b");

        let mut s = String::new();
        escape_text("\x00", &mut s);
        assert_eq!(s, "&#x0;");
    }

    #[test]
    fn text_keeps_tab_lf_cr() {
        let mut s = String::new();
        escape_text("a\tb\nc\rd", &mut s);
        assert_eq!(s, "a\tb\nc\rd");
    }

    #[test]
    fn attr_handles_unicode() {
        let mut s = String::new();
        escape_attr("\"héllo\"", &mut s);
        assert_eq!(s, "&quot;héllo&quot;");
    }
}
