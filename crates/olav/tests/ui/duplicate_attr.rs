use olav::xml;

fn main() {
    let _ = xml! { root(id="1", id="2") { "x" } };
}
