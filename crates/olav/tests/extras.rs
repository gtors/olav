use olav::xml;

#[test]
fn cyrillic_full_document_example_from_readme() {
    // Example from the README — Cyrillic-named XML document
    let автор = "Лев Толстой";
    let год: i32 = 2026;
    let _doc = xml! {
        ?xml version="1.0" encoding="UTF-8"
        каталог {
            книга(id="1", автор=автор, год=год) {
                название { "Война и мир" }
                жанр { "роман" }
            }
            книга(id="2", автор=автор, год=год) {
                название { "Анна Каренина" }
                жанр { "роман" }
            }
            @for жанр in &["роман", "повесть", "рассказ"] {
                метка { @жанр }
            }
        }
    };
    // Just verify it compiles and produces non-empty output
    let s = _doc.into_string();
    assert!(s.contains("<каталог>"));
    assert!(s.contains("Война и мир"));
    assert!(s.contains("Анна Каренина"));
    assert!(s.contains("Лев Толстой"));
    assert!(s.contains("метка"));
}

#[test]
fn cyrillic_bare_ident_element() {
    let m = xml! { книга { "текст" } };
    assert_eq!(m.as_str(), "<книга>текст</книга>");
}

#[test]
fn cyrillic_nested() {
    let m = xml! {
        библиотека {
            книга(id="1") { "Война и мир" }
            книга(id="2") { "Анна Каренина" }
        }
    };
    assert_eq!(
        m.as_str(),
        "<библиотека><книга id=\"1\">Война и мир</книга><книга id=\"2\">Анна Каренина</книга></библиотека>"
    );
}

#[test]
fn cyrillic_self_closing() {
    let m = xml! { страница { } };
    assert_eq!(m.as_str(), "<страница/>");
}

#[test]
fn cyrillic_with_attrs_and_text() {
    let автор = "Толстой";
    let m = xml! {
        книга(язык="ru", автор=автор) {
            "Содержание"
        }
    };
    assert_eq!(
        m.as_str(),
        "<книга язык=\"ru\" автор=\"Толстой\">Содержание</книга>"
    );
}

#[test]
fn cyrillic_with_brackets() {
    let m = xml! {
        [документ:книга] { "x" }
    };
    assert_eq!(m.as_str(), "<документ:книга>x</документ:книга>");
}

#[test]
fn xml_standalone_attr() {
    let m = xml! {
        ?xml version="1.0" encoding="UTF-8" standalone="yes"
        root { }
    };
    let s = m.as_str();
    assert!(s.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>"));
    assert!(s.ends_with("<root/>"));
}

#[test]
fn doctype_with_internal_subset_entity() {
    let m = xml! {
        !DOCTYPE root [
            <!ENTITY author "John Doe">
            <!ENTITY year "2024">
        ]
        root { }
    };
    let s = m.as_str();
    assert!(s.starts_with("<!DOCTYPE root [<!ENTITY author \"John Doe\">"));
    assert!(s.contains("<!ENTITY year \"2024\">"));
    assert!(s.ends_with("]><root/>"));
}

#[test]
fn doctype_with_internal_subset_element() {
    let m = xml! {
        !DOCTYPE root [
            <!ELEMENT root (#PCDATA)>
            <!ELEMENT child EMPTY>
        ]
        root { "x" }
    };
    let s = m.as_str();
    assert!(s.contains("<!ELEMENT root"));
    assert!(s.contains("(#PCDATA)>"));
    assert!(s.contains("<!ELEMENT child EMPTY>"));
    assert!(s.ends_with("]><root>x</root>"));
}

#[test]
fn doctype_with_internal_subset_and_external() {
    let m = xml! {
        !DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Strict//EN" "xhtml1.dtd" [
            <!ENTITY copy "©">
        ]
        html { }
    };
    let s = m.as_str();
    assert!(
        s.starts_with("<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.0 Strict//EN\" \"xhtml1.dtd\"")
    );
    assert!(s.contains("<!ENTITY copy \"©\">"));
    assert!(s.contains("]><html/>"));
}

#[test]
fn element_name_with_dashes() {
    let m = xml! { [my-element] { "x" } };
    assert_eq!(m.as_str(), "<my-element>x</my-element>");
}

#[test]
fn attributes_with_special_chars() {
    let m = xml! {
        root([xlink:href]="https://example.com", [data-x]="42", [xml:lang]="en") {
            "x"
        }
    };
    let s = m.as_str();
    assert!(s.contains("xlink:href=\"https://example.com\""));
    assert!(s.contains("data-x=\"42\""));
    assert!(s.contains("xml:lang=\"en\""));
}
