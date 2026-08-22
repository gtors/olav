use olav::xml;

#[test]
fn if_let_pattern() {
    let opt: Option<i32> = Some(42);
    let m = xml! {
        root {
            @if let Some(v) = opt {
                value { @v }
            }
        }
    };
    assert_eq!(m.as_str(), "<root><value>42</value></root>");
}

#[test]
fn if_let_none() {
    let opt: Option<i32> = None;
    let m = xml! {
        root {
            @if let Some(v) = opt {
                value { @v }
            } else {
                none { }
            }
        }
    };
    assert_eq!(m.as_str(), "<root><none/></root>");
}

#[test]
fn nested_for() {
    let matrix = vec![vec![1, 2], vec![3, 4]];
    let m = xml! {
        matrix {
            @for row in &matrix {
                row {
                    @for val in row {
                        cell { @val }
                    }
                }
            }
        }
    };
    assert_eq!(
        m.as_str(),
        "<matrix><row><cell>1</cell><cell>2</cell></row><row><cell>3</cell><cell>4</cell></row></matrix>"
    );
}

#[test]
fn while_loop() {
    let items: Vec<i32> = vec![1, 2, 3];
    let mut iter = items.into_iter();
    let m = xml! {
        log {
            @while let Some(counter) = iter.next() {
                tick { @counter }
            }
        }
    };
    assert_eq!(
        m.as_str(),
        "<log><tick>1</tick><tick>2</tick><tick>3</tick></log>"
    );
}

#[test]
fn match_expr() {
    let n = 2;
    let m = xml! {
        result {
            @match n {
                1 => { one { } }
                2 => { two { } }
                _ => { other { } }
            }
        }
    };
    assert_eq!(m.as_str(), "<result><two/></result>");
}

#[test]
fn match_with_binding() {
    let opt: Option<&str> = Some("yes");
    let m = xml! {
        r {
            @match opt {
                Some(v) => { y { @v } }
                None => { n { } }
            }
        }
    };
    assert_eq!(m.as_str(), "<r><y>yes</y></r>");
}

#[test]
fn splice_format_args() {
    let name = "world";
    let n = 3;
    let m = xml! { p { @format!("hello {}, n={}", name, n) } };
    assert_eq!(m.as_str(), "<p>hello world, n=3</p>");
}

#[test]
fn splice_chars() {
    let ch = '<';
    let m = xml! { p { @ch } };
    assert_eq!(m.as_str(), "<p>&lt;</p>");
}

#[test]
fn splice_option_some() {
    // Option<T> renders the inner value when Some, nothing when None.
    let opt: Option<i32> = Some(7);
    let m = xml! { p { @opt } };
    assert_eq!(m.as_str(), "<p>7</p>");

    let none: Option<i32> = None;
    let m = xml! { p { @none } };
    assert_eq!(m.as_str(), "<p></p>");

    // String option escapes:
    let s: Option<&str> = Some("a & b");
    let m = xml! { p { @s } };
    assert_eq!(m.as_str(), "<p>a &amp; b</p>");
}

#[test]
fn splice_vec() {
    // Vec<T> renders each element in order.
    let v: Vec<&str> = vec!["a", "b", "c"];
    let m = xml! { p { @v } };
    assert_eq!(m.as_str(), "<p>abc</p>");
}

#[test]
fn splice_array() {
    let arr = [1, 2, 3];
    let m = xml! { p { @arr } };
    assert_eq!(m.as_str(), "<p>123</p>");
}

#[test]
fn raw_splice_str() {
    let safe = "<b>bold</b>";
    let m = xml! { div { @safe.raw } };
    assert_eq!(m.as_str(), "<div><b>bold</b></div>");
}

#[test]
fn raw_splice_does_not_escape() {
    let safe = "a & b";
    let m = xml! { div { @safe.raw } };
    assert_eq!(m.as_str(), "<div>a & b</div>");
}
