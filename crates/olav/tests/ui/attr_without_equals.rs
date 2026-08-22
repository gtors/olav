use olav::xml;

fn main() {
    let _ = xml! {
        root(broken attr) { }
    };
}
