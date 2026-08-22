use olav::xml;

fn main() {
    let _ = xml! { root { @comment "a -- b" } };
}
