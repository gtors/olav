use olav::xml;

#[test]
fn bracketed_element_name() {
    let m = xml! { [ns:book] { "hi" } };
    assert_eq!(m.as_str(), "<ns:book>hi</ns:book>");
}

#[test]
fn bracketed_self_closing() {
    let m = xml! { [svg:rect] { } };
    assert_eq!(m.as_str(), "<svg:rect/>");
}

#[test]
fn bracketed_nested() {
    let m = xml! { [svg:svg] { [svg:rect] { } } };
    assert_eq!(m.as_str(), "<svg:svg><svg:rect/></svg:svg>");
}

#[test]
fn bracketed_with_attrs() {
    let m = xml! { [svg:rect](x="10", y="20") { } };
    assert_eq!(m.as_str(), "<svg:rect x=\"10\" y=\"20\"/>");
}

#[test]
fn xml_namespace_declaration() {
    let m = xml! {
        root(xmlns="http://example.com/ns") {
            child { "x" }
        }
    };
    assert_eq!(
        m.as_str(),
        "<root xmlns=\"http://example.com/ns\"><child>x</child></root>"
    );
}

#[test]
fn multiple_attrs() {
    let m = xml! {
        book(id="1", lang="en", edition="2") {
            "x"
        }
    };
    assert_eq!(
        m.as_str(),
        "<book id=\"1\" lang=\"en\" edition=\"2\">x</book>"
    );
}

#[test]
fn attr_with_string_value() {
    let v: String = "hello".into();
    let m = xml! { div(class=v) { } };
    assert_eq!(m.as_str(), "<div class=\"hello\"/>");
}

#[test]
fn attr_with_format() {
    let x = 5;
    let y = 10;
    let m = xml! { point(x=x, y=y) { } };
    assert_eq!(m.as_str(), "<point x=\"5\" y=\"10\"/>");
}

#[test]
fn deeply_nested() {
    let m = xml! {
        a {
            b {
                c {
                    d {
                        "deep"
                    }
                }
            }
        }
    };
    assert_eq!(m.as_str(), "<a><b><c><d>deep</d></c></b></a>");
}
