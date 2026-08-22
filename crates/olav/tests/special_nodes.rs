use olav::xml;

#[test]
fn xml_declaration() {
    let m = xml! {
        ?xml version="1.0" encoding="UTF-8"
        root { "hello" }
    };
    assert_eq!(
        m.as_str(),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><root>hello</root>"
    );
}

#[test]
fn xml_declaration_minimal() {
    let m = xml! {
        ?xml version="1.0"
        root { }
    };
    assert_eq!(m.as_str(), "<?xml version=\"1.0\"?><root/>");
}

#[test]
fn processing_instruction() {
    let m = xml! {
        ?xml-stylesheet type="text/xsl" href="style.xsl"
        root { "x" }
    };
    assert_eq!(
        m.as_str(),
        "<?xml-stylesheet type=\"text/xsl\" href=\"style.xsl\"?><root>x</root>"
    );
}

#[test]
fn doctype_simple() {
    let m = xml! {
        !DOCTYPE html
        root { }
    };
    assert_eq!(m.as_str(), "<!DOCTYPE html><root/>");
}

#[test]
fn doctype_with_system() {
    let m = xml! {
        !DOCTYPE html SYSTEM "about:legacy-compat"
        root { }
    };
    assert_eq!(
        m.as_str(),
        "<!DOCTYPE html SYSTEM \"about:legacy-compat\"><root/>"
    );
}

#[test]
fn comment_simple() {
    let m = xml! { @comment "a comment" root { "x" } };
    assert_eq!(m.as_str(), "<!--a comment--><root>x</root>");
}

#[test]
fn comment_inside_element() {
    let m = xml! { root { @comment "note" "x" } };
    assert_eq!(m.as_str(), "<root><!--note-->x</root>");
}

#[test]
fn cdata_section() {
    let m = xml! {
        code {
            @cdata {
                "<tag> & stuff"
            }
        }
    };
    assert_eq!(m.as_str(), "<code><![CDATA[<tag> & stuff]]></code>");
}

#[test]
fn cdata_with_text() {
    let m = xml! {
        data {
            @cdata {
                "raw " "stuff"
            }
        }
    };
    assert_eq!(m.as_str(), "<data><![CDATA[raw stuff]]></data>");
}

#[test]
fn cdata_text_is_verbatim_not_escaped() {
    let m = xml! {
        data {
            @cdata {
                "1 < 2 && 3 > 2"
            }
        }
    };
    assert_eq!(m.as_str(), "<data><![CDATA[1 < 2 && 3 > 2]]></data>");
}

#[test]
fn cdata_with_control_flow_and_raw_splice() {
    let s = "a & b";
    let m = xml! {
        data {
            @cdata {
                @if true { "x < y" }
                @s.raw
            }
        }
    };
    assert_eq!(m.as_str(), "<data><![CDATA[x < ya & b]]></data>");
}

#[test]
fn full_document() {
    let title = "Olav";
    let m = xml! {
        ?xml version="1.0" encoding="UTF-8"
        !DOCTYPE html
        html {
            head {
                title { @title }
            }
            body {
                @comment "main content"
                p { "hello world" }
                @if true {
                    div { "shown" }
                }
            }
        }
    };
    let expected = "<?xml version=\"1.0\" encoding=\"UTF-8\"?><!DOCTYPE html>\
        <html><head><title>Olav</title></head>\
        <body><!--main content--><p>hello world</p><div>shown</div></body></html>";
    assert_eq!(m.as_str(), expected);
}
