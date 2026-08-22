use olav::xml;

#[test]
fn simple_element_with_text() {
    let m = xml! { book { "hello" } };
    assert_eq!(m.as_str(), "<book>hello</book>");
}

#[test]
fn self_closing_element() {
    let m = xml! { br { } };
    assert_eq!(m.as_str(), "<br/>");
}

#[test]
fn nested_elements() {
    let m = xml! { catalog { book { "hi" } } };
    assert_eq!(m.as_str(), "<catalog><book>hi</book></catalog>");
}

#[test]
fn literal_attribute() {
    let m = xml! { book(id="42") { "x" } };
    assert_eq!(m.as_str(), "<book id=\"42\">x</book>");
}

#[test]
fn expression_attribute() {
    let id: i32 = 7;
    let m = xml! { book(id=id) { "x" } };
    assert_eq!(m.as_str(), "<book id=\"7\">x</book>");
}

#[test]
fn splice_text() {
    let name = "world";
    let m = xml! { p { "hello " @name } };
    assert_eq!(m.as_str(), "<p>hello world</p>");
}

#[test]
fn splice_escapes() {
    let s = "a & b";
    let m = xml! { p { @s } };
    assert_eq!(m.as_str(), "<p>a &amp; b</p>");
}

#[test]
fn text_literal_decodes_escapes() {
    let m = xml! { p { "line1\nline2" } };
    assert_eq!(m.as_str(), "<p>line1\nline2</p>");
}

#[test]
fn text_literal_decodes_escaped_quote() {
    let m = xml! { p { "say \"hi\"" } };
    assert_eq!(m.as_str(), "<p>say \"hi\"</p>");
}

#[test]
fn text_literal_raw_string() {
    let m = xml! { p { r#"a "quoted" & <b>"# } };
    assert_eq!(m.as_str(), "<p>a \"quoted\" &amp; &lt;b&gt;</p>");
}

#[test]
fn if_true() {
    let cond = true;
    let m = xml! { div { @if cond { "yes" } } };
    assert_eq!(m.as_str(), "<div>yes</div>");
}

#[test]
fn if_false() {
    let cond = false;
    let m = xml! { div { @if cond { "yes" } } };
    assert_eq!(m.as_str(), "<div></div>");
}

#[test]
fn if_else() {
    let cond = false;
    let m = xml! { div { @if cond { "yes" } else { "no" } } };
    assert_eq!(m.as_str(), "<div>no</div>");
}

#[test]
fn for_loop() {
    let xs = vec!["a", "b", "c"];
    let m = xml! { ul { @for x in &xs { li { @x } } } };
    assert_eq!(m.as_str(), "<ul><li>a</li><li>b</li><li>c</li></ul>");
}
