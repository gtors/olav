use olav::xml;

fn main() {
    let _ = xml! { root { @cdata { "a ]]> b" } } };
}
