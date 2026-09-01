use olav::xml;
use quick_xml::Reader;
use quick_xml::events::{BytesText, Event};

fn parse_events(s: &str) -> Vec<Event<'_>> {
    let mut reader = Reader::from_str(s);
    reader.config_mut().trim_text(false);
    let mut events = Vec::new();
    let mut text_buf = String::new();
    let flush = |buf: &mut String, events: &mut Vec<Event<'_>>| {
        if !buf.is_empty() {
            events.push(Event::Text(BytesText::from_escaped(std::mem::take(buf))));
        }
    };
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => {
                flush(&mut text_buf, &mut events);
                break;
            }
            Ok(Event::Text(t)) => {
                text_buf.push_str(t.as_ref());
            }
            Ok(Event::GeneralRef(r)) => match r.as_ref() {
                "amp" => text_buf.push('&'),
                "lt" => text_buf.push('<'),
                "gt" => text_buf.push('>'),
                "quot" => text_buf.push('"'),
                "apos" => text_buf.push('\''),
                _ => {}
            },
            Ok(e) => {
                flush(&mut text_buf, &mut events);
                events.push(e);
            }
            Err(e) => panic!("parse error: {:?}", e),
        }
    }
    events
}

#[test]
fn roundtrip_simple_element() {
    let m = xml! { root { "hello" } };
    let events = parse_events(&m);
    assert!(matches!(events[0], Event::Start(ref e) if e.name().as_ref() == "root"));
    assert!(matches!(events[1], Event::Text(ref e) if e.as_ref() == "hello"));
    assert!(matches!(events[2], Event::End(ref e) if e.name().as_ref() == "root"));
}

#[test]
fn roundtrip_self_closing() {
    let m = xml! { br { } };
    let events = parse_events(&m);
    assert!(matches!(events[0], Event::Empty(ref e) if e.name().as_ref() == "br"));
}

#[test]
fn roundtrip_with_attribute() {
    let m = xml! { book(id="42") { "x" } };
    let events = parse_events(&m);
    if let Event::Start(ref e) = events[0] {
        assert_eq!(e.name().as_ref(), "book");
        let mut attrs: Vec<_> = e.attributes().collect();
        assert_eq!(attrs.len(), 1);
        let attr = attrs.remove(0).unwrap();
        assert_eq!(attr.key.as_ref(), "id");
        assert_eq!(attr.value.as_ref(), "42");
    } else {
        panic!("expected Start event");
    }
}

#[test]
fn roundtrip_escapes_text() {
    let s = "a & b < c > d";
    let m = xml! { p { @s } };
    let events = parse_events(&m);
    if let Event::Text(ref e) = events[1] {
        assert_eq!(e.as_ref(), "a & b < c > d");
    } else {
        panic!("expected Text event");
    }
}

#[test]
fn roundtrip_namespaced() {
    let m = xml! { [svg:svg] { [svg:rect](x="10") { } } };
    let events = parse_events(&m);
    assert!(matches!(events[0], Event::Start(ref e) if e.name().as_ref() == "svg:svg"));
    assert!(matches!(events[1], Event::Empty(ref e) if e.name().as_ref() == "svg:rect"));
}

#[test]
fn roundtrip_pi() {
    let m = xml! {
        ?xml version="1.0"
        root { }
    };
    let events = parse_events(&m);
    if let Event::Decl(ref e) = events[0] {
        let v = e.version().unwrap();
        assert_eq!(v.as_ref(), "1.0");
    } else {
        panic!("expected Decl event");
    }
}

#[test]
fn roundtrip_pi_stylesheet() {
    let m = xml! {
        ?xml-stylesheet type="text/xsl" href="style.xsl"
        root { }
    };
    let events = parse_events(&m);
    if let Event::PI(ref e) = events[0] {
        assert_eq!(e.target(), "xml-stylesheet");
    } else {
        panic!("expected PI event");
    }
}

#[test]
fn roundtrip_doctype() {
    let m = xml! {
        !DOCTYPE html
        root { }
    };
    let events = parse_events(&m);
    assert!(matches!(events[0], Event::DocType(ref e) if e.as_ref().contains("html")));
}

#[test]
fn roundtrip_comment() {
    let m = xml! { @comment "a note" root { } };
    let events = parse_events(&m);
    if let Event::Comment(ref e) = events[0] {
        assert_eq!(e.as_ref(), "a note");
    } else {
        panic!("expected Comment event");
    }
}

#[test]
fn roundtrip_cdata() {
    let m = xml! {
        code {
            @cdata {
                "<tag> & stuff"
            }
        }
    };
    let events = parse_events(&m);
    if let Event::CData(ref e) = events[1] {
        assert_eq!(e.as_ref(), "<tag> & stuff");
    } else {
        panic!("expected CData event");
    }
}

#[test]
fn roundtrip_control_flow() {
    let items = vec!["a", "b", "c"];
    let m = xml! {
        list {
            @for x in &items {
                item { @x }
            }
        }
    };
    let events = parse_events(&m);
    // start list, (start item, text, end item) x 3, end list
    assert_eq!(events.len(), 11);
}

#[test]
fn roundtrip_complex_doc() {
    let title = "Olav";
    let m = xml! {
        ?xml version="1.0" encoding="UTF-8"
        !DOCTYPE html
        html(xmlns="urn:x") {
            head {
                title { @title }
            }
            body {
                @comment "main"
                p(class="intro") { "hello" }
                @if true {
                    div { "shown" }
                }
            }
        }
    };
    // Just verify it parses without error
    let events = parse_events(&m);
    assert!(!events.is_empty());
}
